//! Minimal MCP client (GRAFT-0022, after goose's MCP-first design and Claude Code's
//! MCP support): stdio (newline-delimited JSON-RPC 2.0) or Streamable HTTP
//! (GRAFT-0072: POST per message, JSON or SSE replies, `Mcp-Session-Id`). Every
//! request has a timeout so a broken server can never hang the agent.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oxgraft_seams::{Action, EditFormat, Tool, ToolContext, ToolOutput, ToolSpec};
use serde::Deserialize;
use serde_json::{Value, json};

const PROTOCOL_VERSION: &str = "2025-06-18";
const INIT_TIMEOUT: Duration = Duration::from_secs(30);
const CALL_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct ServerConfig {
    /// Program to start for a stdio server.
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// URL of a Streamable HTTP server (instead of `command`).
    #[serde(default)]
    pub url: Option<String>,
    /// Extra HTTP headers, e.g. `Authorization`. Values may name env vars as `${VAR}`.
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

type Pending = Arc<Mutex<HashMap<u64, Sender<Result<Value, String>>>>>;

enum Transport {
    Stdio { child: Mutex<Child>, stdin: Mutex<ChildStdin>, pending: Pending },
    Http { url: String, headers: Vec<(String, String)>, session: Mutex<Option<String>> },
}

/// A connected MCP server.
pub struct Client {
    name: String,
    transport: Transport,
    next_id: Mutex<u64>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClient").field("name", &self.name).finish_non_exhaustive()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Transport::Stdio { child, .. } = &self.transport
            && let Ok(mut child) = child.lock()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// `${VAR}` in a header value is replaced by that environment variable.
fn expand_env(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let Some(end) = rest[start..].find('}') else { break };
        out.push_str(&rest[..start]);
        out.push_str(&std::env::var(&rest[start + 2..start + end]).unwrap_or_default());
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    out
}

impl Client {
    pub fn spawn(name: &str, config: &ServerConfig, cwd: &std::path::Path) -> Result<Arc<Self>, String> {
        let mut child = Command::new(&config.command)
            .args(&config.args)
            .envs(&config.env)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("MCP server `{name}`: cannot start `{}`: {e}", config.command))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let client = Arc::new(Self {
            name: name.into(),
            transport: Transport::Stdio {
                child: Mutex::new(child),
                stdin: Mutex::new(stdin),
                pending: pending.clone(),
            },
            next_id: Mutex::new(1),
        });

        let weak = Arc::downgrade(&client);
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                let id = msg.get("id").and_then(Value::as_u64);
                if msg.get("method").is_some() {
                    // A request or notification from the server. We support no client
                    // features (sampling, roots, elicitation): answer requests with an error.
                    if let (Some(id), Some(c)) = (msg.get("id").cloned(), weak.upgrade()) {
                        let _ = c.send(&json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "not supported by oxgraft"}}));
                    }
                    continue;
                }
                let Some(id) = id else { continue };
                let tx = pending.lock().ok().and_then(|mut p| p.remove(&id));
                if let Some(tx) = tx {
                    let result = match (msg.get("result"), msg.get("error")) {
                        (Some(r), _) => Ok(r.clone()),
                        (_, Some(e)) => Err(e.get("message").and_then(Value::as_str).unwrap_or("error").to_string()),
                        _ => Err("malformed response".into()),
                    };
                    let _ = tx.send(result);
                }
            }
            // Server exited: fail everything still waiting.
            if let Ok(mut p) = pending.lock() {
                for (_, tx) in p.drain() {
                    let _ = tx.send(Err("MCP server exited".into()));
                }
            }
        });

        client.initialize()?;
        Ok(client)
    }

    /// Connect to a Streamable HTTP server (GRAFT-0072).
    pub fn connect_http(name: &str, url: &str, headers: &HashMap<String, String>) -> Result<Arc<Self>, String> {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(format!("MCP server `{name}`: url must start with http:// or https://"));
        }
        let mut headers: Vec<(String, String)> = headers.iter().map(|(k, v)| (k.clone(), expand_env(v))).collect();
        headers.sort();
        let client = Arc::new(Self {
            name: name.into(),
            transport: Transport::Http { url: url.into(), headers, session: Mutex::new(None) },
            next_id: Mutex::new(1),
        });
        client.initialize()?;
        Ok(client)
    }

    fn initialize(&self) -> Result<(), String> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "oxgraft", "version": env!("CARGO_PKG_VERSION")}
            }),
            INIT_TIMEOUT,
        )?;
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
    }

    fn send(&self, msg: &Value) -> Result<(), String> {
        match &self.transport {
            Transport::Stdio { stdin, .. } => {
                let mut stdin = stdin.lock().map_err(|_| "stdin poisoned")?;
                writeln!(stdin, "{msg}")
                    .and_then(|_| stdin.flush())
                    .map_err(|e| format!("MCP server `{}`: {e}", self.name))
            }
            Transport::Http { .. } => self.http_post(msg, INIT_TIMEOUT).map(|_| ()),
        }
    }

    /// POST one JSON-RPC message; returns the response for `msg`'s id, if any.
    fn http_post(&self, msg: &Value, timeout: Duration) -> Result<Option<Value>, String> {
        let Transport::Http { url, headers, session } = &self.transport else { return Ok(None) };
        let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout(timeout).build();
        let mut req = agent
            .post(url)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json, text/event-stream")
            .set("MCP-Protocol-Version", PROTOCOL_VERSION);
        for (k, v) in headers {
            req = req.set(k, v);
        }
        if let Some(id) = session.lock().map_err(|_| "session poisoned")?.as_deref() {
            req = req.set("Mcp-Session-Id", id);
        }
        let resp = match req.send_string(&msg.to_string()) {
            Ok(r) => r,
            Err(ureq::Error::Status(code, r)) => {
                let body: String = r.into_string().unwrap_or_default().chars().take(300).collect();
                return Err(format!("MCP server `{}`: HTTP {code}: {body}", self.name));
            }
            Err(e) => return Err(format!("MCP server `{}`: {e}", self.name)),
        };
        if let Some(id) = resp.header("Mcp-Session-Id") {
            *session.lock().map_err(|_| "session poisoned")? = Some(id.to_string());
        }
        let want = msg.get("id").cloned();
        if want.is_none() {
            return Ok(None); // a notification: 202 Accepted, no body
        }
        let is_sse = resp.content_type().starts_with("text/event-stream");
        let body = resp.into_string().map_err(|e| format!("MCP server `{}`: {e}", self.name))?;
        let candidates: Vec<Value> = if is_sse {
            body.split("\n\n")
                .filter_map(|event| {
                    let data: Vec<&str> = event
                        .lines()
                        .filter_map(|l| l.strip_prefix("data:").map(|d| d.strip_prefix(' ').unwrap_or(d)))
                        .collect();
                    serde_json::from_str(&data.join("\n")).ok()
                })
                .collect()
        } else {
            vec![serde_json::from_str(&body).map_err(|e| format!("MCP server `{}`: bad JSON: {e}", self.name))?]
        };
        let found = candidates
            .into_iter()
            .flat_map(|v| if let Value::Array(a) = v { a } else { vec![v] })
            .find(|m| m.get("id") == want.as_ref());
        Ok(found)
    }

    pub fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        let id = {
            let mut n = self.next_id.lock().map_err(|_| "id poisoned")?;
            *n += 1;
            *n
        };
        let msg = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let pending = match &self.transport {
            Transport::Stdio { pending, .. } => pending,
            Transport::Http { .. } => {
                let reply = self
                    .http_post(&msg, timeout)?
                    .ok_or_else(|| format!("MCP server `{}`: {method}: no response in the reply", self.name))?;
                return match (reply.get("result"), reply.get("error")) {
                    (Some(r), _) => Ok(r.clone()),
                    (_, Some(e)) => Err(format!(
                        "MCP server `{}`: {method}: {}",
                        self.name,
                        e.get("message").and_then(Value::as_str).unwrap_or("error")
                    )),
                    _ => Err(format!("MCP server `{}`: {method}: malformed response", self.name)),
                };
            }
        };
        let (tx, rx) = channel();
        pending.lock().map_err(|_| "pending poisoned")?.insert(id, tx);
        self.send(&msg)?;
        match rx.recv_timeout(timeout) {
            Ok(r) => r.map_err(|e| format!("MCP server `{}`: {method}: {e}", self.name)),
            Err(_) => {
                if let Ok(mut p) = pending.lock() {
                    p.remove(&id);
                }
                Err(format!("MCP server `{}`: {method} timed out after {}s", self.name, timeout.as_secs()))
            }
        }
    }

    /// All tools the server offers (follows pagination).
    pub fn list_tools(&self) -> Result<Vec<Value>, String> {
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..50 {
            let params = match &cursor {
                Some(c) => json!({"cursor": c}),
                None => json!({}),
            };
            let result = self.request("tools/list", params, INIT_TIMEOUT)?;
            tools.extend(result.get("tools").and_then(Value::as_array).cloned().unwrap_or_default());
            cursor = result.get("nextCursor").and_then(Value::as_str).map(str::to_string);
            if cursor.is_none() {
                break;
            }
        }
        Ok(tools)
    }
}

/// One MCP tool exposed to the model as `mcp__<server>__<tool>`.
#[derive(Debug)]
pub struct McpTool {
    client: Arc<Client>,
    remote_name: String,
    spec: ToolSpec,
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect()
}

/// The name the model sees, limited to 64 characters of `[A-Za-z0-9_-]`.
pub fn tool_name(server: &str, tool: &str) -> String {
    let name = format!("mcp__{}__{}", sanitize(server), sanitize(tool));
    name.chars().take(64).collect()
}

/// Start a server and wrap its tools. Errors are returned, not panicked, so one
/// broken server never stops the session from starting.
pub fn connect(name: &str, config: &ServerConfig, cwd: &std::path::Path) -> Result<Vec<Box<dyn Tool>>, String> {
    let client = match (&config.url, config.command.trim().is_empty()) {
        (Some(url), true) => Client::connect_http(name, url, &config.headers)?,
        (None, false) => Client::spawn(name, config, cwd)?,
        (Some(_), false) => return Err(format!("MCP server `{name}`: set either `command` or `url`, not both")),
        (None, true) => return Err(format!("MCP server `{name}`: needs a `command` (stdio) or a `url` (HTTP)")),
    };
    let tools = client.list_tools()?;
    Ok(tools
        .into_iter()
        .filter_map(|t| {
            let remote = t.get("name")?.as_str()?.to_string();
            let mut schema = t.get("inputSchema").cloned().unwrap_or_else(|| json!({"type": "object"}));
            if schema.get("type").is_none() {
                schema["type"] = json!("object");
            }
            let description = t.get("description").and_then(Value::as_str).unwrap_or("").to_string();
            Some(Box::new(McpTool {
                client: client.clone(),
                spec: ToolSpec {
                    name: tool_name(name, &remote),
                    description: format!("[MCP server `{name}`] {description}"),
                    input_schema: schema,
                },
                remote_name: remote,
            }) as Box<dyn Tool>)
        })
        .collect())
}

impl Tool for McpTool {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        self.spec.clone()
    }

    fn action(&self, input: &Value) -> Action {
        Action { tool: self.spec.name.clone(), resource: input.to_string().chars().take(200).collect() }
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let args = if input.is_object() { input.clone() } else { json!({}) };
        match self.client.request("tools/call", json!({"name": self.remote_name, "arguments": args}), CALL_TIMEOUT) {
            Ok(result) => {
                let mut text = String::new();
                for item in result.get("content").and_then(Value::as_array).into_iter().flatten() {
                    match item.get("type").and_then(Value::as_str) {
                        Some("text") => text.push_str(item.get("text").and_then(Value::as_str).unwrap_or("")),
                        Some(other) => text.push_str(&format!("[{other} content omitted]")),
                        None => {}
                    }
                    text.push('\n');
                }
                if text.trim().is_empty()
                    && let Some(s) = result.get("structuredContent")
                {
                    text = s.to_string();
                }
                let text = crate::util::truncate(text.trim_end(), ctx.max_output_chars);
                if result.get("isError").and_then(Value::as_bool).unwrap_or(false) {
                    ToolOutput::error(text)
                } else {
                    ToolOutput::ok(if text.is_empty() {
                        "(no output)".into()
                    } else {
                        crate::util::untrusted(&format!("MCP tool {}", self.spec.name), &text)
                    })
                }
            }
            Err(e) => ToolOutput::error(e),
        }
    }
}

/// A tiny MCP server used by tests and `oxgraft __mcp-fixture`: tools `echo` and `fail`.
pub fn serve_fixture() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        if let Some(reply) = fixture_reply(&msg) {
            let _ = writeln!(out, "{reply}");
            let _ = out.flush();
        }
    }
}

/// The fixture server's reply to one message (`None` for notifications).
pub fn fixture_reply(msg: &Value) -> Option<Value> {
    {
        let id = msg.get("id").cloned()?;
        let result = match msg.get("method").and_then(Value::as_str) {
            Some("initialize") => {
                json!({"protocolVersion": PROTOCOL_VERSION, "capabilities": {"tools": {}}, "serverInfo": {"name": "fixture", "version": "1"}})
            }
            Some("tools/list") => json!({"tools": [
                {"name": "echo", "description": "Echo the text back", "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}},
                {"name": "fail", "description": "Always fails", "inputSchema": {"type": "object"}}
            ]}),
            Some("tools/call") => match msg.pointer("/params/name").and_then(Value::as_str) {
                Some("echo") => {
                    let text = msg.pointer("/params/arguments/text").and_then(Value::as_str).unwrap_or("");
                    json!({"content": [{"type": "text", "text": format!("echo: {text}")}]})
                }
                _ => json!({"content": [{"type": "text", "text": "it failed"}], "isError": true}),
            },
            _ => return Some(json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "unknown"}})),
        };
        Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_sanitized_and_bounded() {
        assert_eq!(tool_name("git hub", "create.issue"), "mcp__git_hub__create_issue");
        assert!(tool_name(&"x".repeat(100), "y").len() <= 64);
    }

    #[test]
    fn missing_server_is_an_error_not_a_panic() {
        let cfg = ServerConfig { command: "definitely-not-a-real-binary-oxgraft".into(), ..Default::default() };
        match connect("x", &cfg, &std::env::temp_dir()) {
            Err(e) => assert!(e.contains("cannot start"), "{e}"),
            Ok(_) => panic!("expected an error"),
        }
    }

    /// Serve the fixture over HTTP: JSON for most replies, SSE for tools/list, and a
    /// session id that later requests must send back.
    fn http_fixture() -> (String, std::sync::mpsc::Receiver<Vec<String>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(6) {
                let Ok(mut stream) = stream else { break };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut headers = Vec::new();
                let mut len = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let line = line.trim_end().to_string();
                    if line.is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                    headers.push(line);
                }
                let mut body = vec![0; len];
                reader.read_exact(&mut body).unwrap();
                let _ = tx.send(headers);
                let msg: Value = serde_json::from_slice(&body).unwrap();
                let reply = fixture_reply(&msg);
                let (status, ctype, text) = match (&reply, msg.get("method").and_then(Value::as_str)) {
                    (None, _) => ("202 Accepted", "application/json", String::new()),
                    (Some(r), Some("tools/list")) => {
                        ("200 OK", "text/event-stream", format!("event: message\ndata: {r}\n\n"))
                    }
                    (Some(r), _) => ("200 OK", "application/json", r.to_string()),
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\ncontent-type: {ctype}\r\nmcp-session-id: s-123\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
                    text.len()
                );
            }
        });
        (url, rx)
    }

    // SPEC-0005 AC9 (GRAFT-0072)
    #[test]
    fn http_servers_work_with_json_and_sse_replies() {
        let (url, rx) = http_fixture();
        let config = ServerConfig {
            url: Some(url),
            headers: HashMap::from([("Authorization".to_string(), "Bearer t0ken".to_string())]),
            ..Default::default()
        };
        let tools = connect("remote", &config, std::path::Path::new(".")).unwrap();
        let names: Vec<String> = tools.iter().map(|t| t.spec(&oxgraft_strategies::edit::SearchReplace).name).collect();
        assert_eq!(names, vec!["mcp__remote__echo", "mcp__remote__fail"]);
        let seen: Vec<Vec<String>> = rx.try_iter().collect();
        assert!(seen[0].iter().any(|h| h == "Authorization: Bearer t0ken"), "{seen:?}");
        assert!(!seen[0].iter().any(|h| h.to_ascii_lowercase().starts_with("mcp-session-id")), "no session yet");
        assert!(seen[1].iter().any(|h| h.eq_ignore_ascii_case("mcp-session-id: s-123")), "{seen:?}");
        let path = std::env::var("PATH").unwrap_or_default();
        assert_eq!(expand_env("a${PATH}b"), format!("a{path}b"));
        assert_eq!(expand_env("${OXGRAFT_SURELY_UNSET_VAR}x"), "x");
        let bad = ServerConfig { url: Some("ftp://x".into()), ..Default::default() };
        assert!(connect("x", &bad, std::path::Path::new(".")).is_err());
        let both = ServerConfig { url: Some("http://x".into()), command: "y".into(), ..Default::default() };
        assert!(connect("x", &both, std::path::Path::new(".")).err().is_some_and(|e| e.contains("not both")));
    }
}
