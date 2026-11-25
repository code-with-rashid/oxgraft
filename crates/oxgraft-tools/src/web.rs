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
        // Redirects are followed by hand so every hop is checked (GRAFT-0073).
        // The resolver refuses blocked addresses at connect time, so the address
        // checked is the address used (no DNS-rebinding gap).
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirects(0)
            .resolver(guarded_resolve)
            .user_agent(concat!("oxgraft/", env!("CARGO_PKG_VERSION")))
            .build();
        let mut current = match url::Url::parse(url) {
            Ok(u) => u,
            Err(e) => return ToolOutput::error(format!("invalid URL {url}: {e}")),
        };
        let mut hops = 0;
        let response = loop {
            if let Err(e) = check_destination(current.as_str()) {
                return ToolOutput::error(e);
            }
            let r = match agent.get(current.as_str()).call() {
                Ok(r) => r,
                Err(ureq::Error::Status(code, _)) => {
                    return ToolOutput::error(format!("HTTP {code} fetching {current}"));
                }
                Err(e) => return ToolOutput::error(format!("could not fetch {current}: {e}")),
            };
            match (r.status(), r.header("location")) {
                (300..=399, Some(location)) if hops < 5 => {
                    current = match current.join(location) {
                        Ok(next) => next,
                        Err(e) => return ToolOutput::error(format!("bad redirect from {current}: {e}")),
                    };
                    hops += 1;
                }
                (300..=399, _) => return ToolOutput::error(format!("too many redirects fetching {url}")),
                _ => break r,
            }
        };
        let url = current.as_str().to_string();
        let url = url.as_str();
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

/// Addresses `web_fetch` never connects to (GRAFT-0073): link-local (cloud
/// metadata lives there), unspecified, and known metadata addresses. Loopback and
/// private networks stay allowed (local dev servers).
fn blocked_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => v4.is_link_local() || v4.is_unspecified() || v4.octets() == [100, 100, 100, 200],
        IpAddr::V6(v6) => {
            (v6.segments()[0] & 0xffc0) == 0xfe80
                || v6.is_unspecified()
                || v6.segments() == [0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x254]
                || v6.to_ipv4_mapped().is_some_and(|v4| v4.is_link_local() || v4.is_unspecified())
        }
    }
}

/// ureq resolver that drops blocked addresses; fails if none are left.
fn guarded_resolve(netloc: &str) -> std::io::Result<Vec<std::net::SocketAddr>> {
    use std::net::ToSocketAddrs;
    let all: Vec<_> = netloc.to_socket_addrs()?.collect();
    let allowed: Vec<_> = all.iter().copied().filter(|a| !blocked_ip(a.ip())).collect();
    if allowed.is_empty() && !all.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("refusing to connect to {netloc}: link-local and metadata addresses are blocked"),
        ));
    }
    Ok(allowed)
}

/// Refuse URLs aimed at cloud metadata or link-local hosts. The URL is parsed the
/// way the HTTP client parses it (WHATWG rules: `\` as `/`, percent-encoding,
/// tabs, numeric IPv4 forms), so the checked host is the host that is fetched;
/// `guarded_resolve` checks the resolved address again at connect time.
pub fn check_destination(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid URL {url}: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("only http:// and https:// URLs can be fetched".into());
    }
    let refused = || Err(format!("refusing to fetch {url}: link-local and metadata addresses are blocked"));
    match parsed.host() {
        None => Err(format!("no host in {url}")),
        Some(url::Host::Ipv4(ip)) if blocked_ip(ip.into()) => refused(),
        Some(url::Host::Ipv6(ip)) if blocked_ip(ip.into()) => refused(),
        Some(url::Host::Domain(d)) => {
            let d = d.trim_end_matches('.').to_ascii_lowercase();
            if matches!(d.as_str(), "metadata.google.internal" | "metadata" | "instance-data") {
                return Err(format!("refusing to fetch {url}: cloud metadata endpoints are blocked"));
            }
            let port = parsed.port_or_known_default().unwrap_or(80);
            match guarded_resolve(&format!("{d}:{port}")) {
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => refused(),
                _ => Ok(()), // unresolvable hosts fail later with a normal network error
            }
        }
        Some(_) => Ok(()),
    }
}

/// Dependency-free HTML to Markdown-ish text: drops script/style/head/nav/footer
/// and comments; keeps headings (`#`), list items (`- `), links (`[text](url)`),
/// `<pre>` blocks (fenced, whitespace kept), inline code and table cells (`|`);
/// decodes named and numeric entities.
pub fn html_to_text(html: &str) -> String {
    const SKIP: &[&str] =
        &["script", "style", "head", "noscript", "svg", "nav", "footer", "template", "iframe", "form"];
    const BLOCK: &[&str] = &[
        "p",
        "div",
        "br",
        "tr",
        "section",
        "article",
        "main",
        "header",
        "table",
        "ul",
        "ol",
        "dl",
        "dt",
        "dd",
        "blockquote",
        "hr",
    ];
    let lower = html.to_ascii_lowercase();
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len() / 2);
    let mut links: Vec<Option<String>> = Vec::new();
    let mut pre = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        if lower[i..].starts_with("<!--") {
            i = lower[i..].find("-->").map_or(bytes.len(), |e| i + e + 3);
            continue;
        }
        if bytes[i] != b'<' {
            let next = lower[i..].find('<').map_or(bytes.len(), |n| i + n);
            let text = &html[i..next];
            if pre > 0 {
                // Protect line breaks and spaces inside <pre> from whitespace collapsing.
                out.push_str(&text.replace('\n', "\u{1}").replace(' ', "\u{2}").replace('\t', "\u{3}"));
            } else {
                out.push_str(text);
            }
            i = next;
            continue;
        }
        let Some(end) = lower[i..].find('>').map(|e| i + e) else { break };
        let raw = &html[i + 1..end];
        let tag = &lower[i + 1..end];
        let closing = tag.starts_with('/');
        let name: String = tag.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        i = end + 1;
        if !closing && SKIP.contains(&name.as_str()) && !tag.ends_with('/') {
            let close = format!("</{name}");
            i = lower[i..].find(&close).map_or(bytes.len(), |c| i + c);
            i = lower[i..].find('>').map_or(bytes.len(), |e| i + e + 1);
            continue;
        }
        match (name.as_str(), closing) {
            (h, false) if h.len() == 2 && h.starts_with('h') && h.as_bytes()[1].is_ascii_digit() => {
                let level = (h.as_bytes()[1] - b'0') as usize;
                out.push_str(&format!("\n\n{} ", "#".repeat(level.clamp(1, 6))));
            }
            (h, true) if h.len() == 2 && h.starts_with('h') && h.as_bytes()[1].is_ascii_digit() => out.push_str("\n\n"),
            ("li", false) => out.push_str("\n- "),
            ("pre", false) => {
                pre += 1;
                out.push_str("\n```\u{1}");
            }
            ("pre", true) => {
                pre = pre.saturating_sub(1);
                out.push_str("\u{1}```\n");
            }
            ("code", _) if pre == 0 => out.push('`'),
            ("td" | "th", false) => out.push_str(" | "),
            ("a", false) => {
                let href = attribute(raw, "href").filter(|h| {
                    !h.is_empty() && !h.starts_with('#') && !h.to_ascii_lowercase().starts_with("javascript:")
                });
                if href.is_some() {
                    out.push('[');
                }
                links.push(href);
            }
            ("a", true) => {
                if let Some(Some(href)) = links.pop() {
                    out.push_str(&format!("]({href})"));
                }
            }
            (n, _) if BLOCK.contains(&n) => out.push('\n'),
            _ => {}
        }
    }
    let decoded = decode_entities(&out);
    let mut lines: Vec<String> = Vec::new();
    for line in decoded.lines() {
        let l = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if !(l.is_empty() && lines.last().is_some_and(|p| p.is_empty())) {
            lines.push(l);
        }
    }
    // Restore the line breaks kept inside <pre> (marked with \u{1}).
    lines.join("\n").replace('\u{1}', "\n").replace('\u{2}', " ").replace('\u{3}', "\t").trim().to_string()
}

/// The value of `name="..."` (or single-quoted / bare) in a raw tag.
fn attribute(raw_tag: &str, name: &str) -> Option<String> {
    let lower = raw_tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(pos) = lower[from..].find(name).map(|p| from + p) {
        from = pos + name.len();
        let before_ok = pos == 0 || lower.as_bytes()[pos - 1].is_ascii_whitespace();
        let rest = raw_tag[from..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        let value = rest[1..].trim_start();
        let v = match value.chars().next() {
            Some(q @ ('"' | '\'')) => value[1..].split(q).next().unwrap_or(""),
            _ => value.split(|c: char| c.is_whitespace() || c == '>').next().unwrap_or(""),
        };
        return Some(decode_entities(v));
    }
    None
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        // Look for `;` within the next 12 characters (char-safe: never split UTF-8).
        let Some(semi) = rest.char_indices().take(12).find(|(_, c)| *c == ';').map(|(i, _)| i) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            "mdash" => Some('—'),
            "ndash" => Some('–'),
            "hellip" => Some('…'),
            "copy" => Some('©'),
            e if e.starts_with("#x") || e.starts_with("#X") => {
                u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32)
            }
            e if e.starts_with('#') => e[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
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
        assert!(text.contains("- one\n- two"), "{text}");
        assert!(!text.contains("evil") && !text.contains("p{}"));
    }

    // SPEC-0002 AC15 (GRAFT-0073)
    #[test]
    fn metadata_and_link_local_destinations_are_blocked() {
        for url in [
            "http://169.254.169.254/latest/meta-data/",
            "http://[fe80::1]/x",
            "http://metadata.google.internal/computeMetadata/v1/",
            "http://user:pw@169.254.170.2:80/",
            "http://[fd00:ec2::254]/",
            "http://100.100.100.200/latest",
            "http://0.0.0.0:8080/",
            // Parser-differential bypasses found in review (GRAFT-0073 revision).
            "http://169.254.169.254\\@example.com/",
            "http://169.254.169.%32%35%34/",
            "http://169.254.\t169.254/",
            "http://2852039166/",
            "http://0xA9FEA9FE/",
            "http://[::ffff:169.254.169.254]/",
            "http://METADATA.GOOGLE.INTERNAL./",
        ] {
            assert!(check_destination(url).is_err(), "{url}");
        }
        for url in ["http://127.0.0.1:3000/", "http://localhost:8080/docs", "https://192.168.1.5/", "http://10.0.0.1/"]
        {
            assert!(check_destination(url).is_ok(), "{url}");
        }
        assert!(guarded_resolve("169.254.169.254:80").is_err());
        assert!(guarded_resolve("127.0.0.1:80").is_ok());
    }

    #[test]
    fn redirects_to_metadata_are_refused() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut line = String::new();
            while r.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            let _ = write!(
                s,
                "HTTP/1.1 302 Found\r\nlocation: http://169.254.169.254/latest/meta-data/\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
            );
        });
        let f = Fixture::new("redirect");
        let out = f.run(&WebFetch, json!({"url": format!("http://{addr}/")}));
        assert!(out.is_error && out.content.contains("refusing"), "{}", out.content);
    }

    #[test]
    fn html_conversion_never_panics() {
        let pieces = [
            "<",
            ">",
            "&",
            ";",
            "#",
            "x",
            "é",
            "日本",
            "<a href=",
            "\"",
            "'",
            "</a>",
            "<pre>",
            "</pre>",
            "<!--",
            "-->",
            "&#",
            "&#x1F600;",
            "<h7>",
            "<script>",
            " ",
            "\n",
            "&amp",
        ];
        let mut seed = 0x2545_f491_u64;
        for _ in 0..3_000 {
            let mut s = String::new();
            for _ in 0..(seed % 40) {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                s.push_str(pieces[(seed % pieces.len() as u64) as usize]);
            }
            let _ = html_to_text(&s);
        }
    }

    // SPEC-0002 AC12
    #[test]
    fn converts_html_to_markdownish_text() {
        let html = r##"<html><body><nav><a href="/">Home</a> menu junk</nav><!-- hidden -->
            <h2>Install</h2><p>Run <code>cargo build</code> or see <a href="https://x.dev/docs?a=1&amp;b=2">the docs</a>.
            <a href="#top">top</a></p>
            <pre>fn main() {
    println!("hi");
}</pre>
            <table><tr><th>Flag</th><th>Use</th></tr><tr><td>--json</td><td>events</td></tr></table>
            <p>caf&#233; &#x2713; &mdash; 5 &lt; 6</p><footer>© footer junk</footer></body></html>"##;
        let text = html_to_text(html);
        assert!(text.contains("## Install"), "{text}");
        assert!(text.contains("Run `cargo build` or see [the docs](https://x.dev/docs?a=1&b=2)."), "{text}");
        assert!(text.contains("top") && !text.contains("(#top)"), "{text}");
        assert!(text.contains("```\nfn main() {\n    println!(\"hi\");\n}\n```"), "pre keeps whitespace:\n{text}");
        assert!(text.contains("| Flag | Use") && text.contains("| --json | events"), "{text}");
        assert!(text.contains("café ✓ — 5 < 6"), "{text}");
        assert!(!text.contains("menu junk") && !text.contains("footer junk") && !text.contains("hidden"), "{text}");
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
