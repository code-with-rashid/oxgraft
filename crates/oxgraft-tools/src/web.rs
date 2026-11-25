//! `web_fetch`: read a web page as text (GRAFT-0027, after OpenCode's webfetch,
//! Claude Code's WebFetch and Gemini CLI's web_fetch). Needs approval under the
//! default rules (`ask` fallback).

use std::io::Read as _;
use std::time::Duration;

use oxgraft_seams::{Action, EditFormat, Tool, ToolContext, ToolOutput, ToolSpec};
use serde_json::{Value, json};

use crate::util::{str_field, truncate};

const MAX_BYTES: u64 = 3 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default)]
pub struct WebFetch;

impl Tool for WebFetch {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "web_fetch".into(),
            description: "Fetch an http(s) URL and return its content as plain text (HTML is converted). Use it to \
                read documentation or issues the task refers to."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {"url": {"type": "string", "description": "http:// or https:// URL"}},
                "required": ["url"]
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        Action { tool: "web_fetch".into(), resource: input.get("url").and_then(Value::as_str).unwrap_or("").into() }
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let url = match str_field(input, "url") {
            Ok(u) => u.trim(),
            Err(e) => return ToolOutput::error(e),
        };
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return ToolOutput::error("only http:// and https:// URLs can be fetched");
        }
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirects(5)
            .user_agent(concat!("oxgraft/", env!("CARGO_PKG_VERSION")))
            .build();
        let response = match agent.get(url).call() {
            Ok(r) => r,
            Err(ureq::Error::Status(code, _)) => return ToolOutput::error(format!("HTTP {code} fetching {url}")),
            Err(e) => return ToolOutput::error(format!("could not fetch {url}: {e}")),
        };
        let content_type = response.content_type().to_ascii_lowercase();
        let mut bytes = Vec::new();
        if let Err(e) = response.into_reader().take(MAX_BYTES).read_to_end(&mut bytes) {
            return ToolOutput::error(format!("error reading {url}: {e}"));
        }
        if !(content_type.starts_with("text/") || content_type.contains("json") || content_type.contains("xml")) {
            return ToolOutput::error(format!(
                "{url} returned `{content_type}` ({} bytes), which is not text",
                bytes.len()
            ));
        }
        let body = String::from_utf8_lossy(&bytes);
        let text = if content_type.contains("html") { html_to_text(&body) } else { body.into_owned() };
        ToolOutput::ok(crate::util::untrusted(url, &truncate(text.trim(), ctx.max_output_chars)))
    }
}

/// Crude but dependency-free HTML to text: drops script/style/head, keeps line
/// structure for block elements, decodes common entities.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let lower = html.to_ascii_lowercase();
    let mut i = 0;
    let bytes = html.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let Some(end) = lower[i..].find('>').map(|e| i + e) else { break };
            let tag = &lower[i + 1..end];
            let name: String = tag.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            if !tag.starts_with('/') && matches!(name.as_str(), "script" | "style" | "head" | "noscript" | "svg") {
                let close = format!("</{name}");
                i = lower[end..].find(&close).map(|c| end + c).unwrap_or(bytes.len());
                if let Some(e) = lower[i..].find('>') {
                    i += e + 1;
                }
                continue;
            }
            if matches!(
                name.as_str(),
                "p" | "div"
                    | "br"
                    | "li"
                    | "tr"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "pre"
                    | "section"
                    | "article"
            ) {
                out.push('\n');
            }
            i = end + 1;
        } else {
            let next = lower[i..].find('<').map(|n| i + n).unwrap_or(bytes.len());
            out.push_str(&html[i..next]);
            i = next;
        }
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    let mut lines: Vec<String> = Vec::new();
    for line in decoded.lines() {
        let l = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if !(l.is_empty() && lines.last().is_some_and(|p| p.is_empty())) {
            lines.push(l);
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;
    use std::io::{BufRead, BufReader, Write};

    #[test]
    fn converts_html() {
        let html = "<html><head><title>x</title><style>p{}</style></head><body><h1>Title</h1><p>Hello &amp; welcome</p>\
                    <script>evil()</script><ul><li>one</li><li>two</li></ul></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("Hello & welcome"));
        assert!(text.contains("one\ntwo") || text.contains("one\n\ntwo"), "{text}");
        assert!(!text.contains("evil") && !text.contains("p{}"));
    }

    #[test]
    fn fetches_over_http_and_rejects_other_schemes() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut line = String::new();
            while r.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            let body = "<p>Docs page</p>";
            let _ = write!(
                s,
                "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
        });
        let f = Fixture::new("web");
        let out = f.run(&WebFetch, json!({"url": format!("http://{addr}/docs")}));
        assert!(!out.is_error && out.content.contains("Docs page"), "{}", out.content);
        assert!(f.run(&WebFetch, json!({"url": "file:///etc/passwd"})).is_error);
    }
}
