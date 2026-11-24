//! Minimal MCP client over stdio (GRAFT-0022, after goose's MCP-first design and
//! Claude Code's MCP support). Newline-delimited JSON-RPC 2.0; every request has a
//! timeout so a broken server can never hang the agent.

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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

type Pending = Arc<Mutex<HashMap<u64, Sender<Result<Value, String>>>>>;

/// A running MCP server.
pub struct Client {
    name: String,
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    pending: Pending,
    next_id: Mutex<u64>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClient").field("name", &self.name).finish_non_exhaustive()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
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
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            pending: pending.clone(),
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

        client.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "oxgraft", "version": env!("CARGO_PKG_VERSION")}
            }),
            INIT_TIMEOUT,
        )?;
        client.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))?;
        Ok(client)
    }

    fn send(&self, msg: &Value) -> Result<(), String> {
        let mut stdin = self.stdin.lock().map_err(|_| "stdin poisoned")?;
        writeln!(stdin, "{msg}").and_then(|_| stdin.flush()).map_err(|e| format!("MCP server `{}`: {e}", self.name))
    }

    pub fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        let id = {
            let mut n = self.next_id.lock().map_err(|_| "id poisoned")?;
            *n += 1;
            *n
        };
        let (tx, rx) = channel();
        self.pending.lock().map_err(|_| "pending poisoned")?.insert(id, tx);
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        match rx.recv_timeout(timeout) {
            Ok(r) => r.map_err(|e| format!("MCP server `{}`: {method}: {e}", self.name)),
            Err(_) => {
                if let Ok(mut p) = self.pending.lock() {
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
    let client = Client::spawn(name, config, cwd)?;
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
                    ToolOutput::ok(if text.is_empty() { "(no output)".into() } else { text })
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
        let Some(id) = msg.get("id").cloned() else { continue };
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
            _ => {
                let _ = writeln!(
                    out,
                    "{}",
                    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "unknown"}})
                );
                continue;
            }
        };
        let _ = writeln!(out, "{}", json!({"jsonrpc": "2.0", "id": id, "result": result}));
        let _ = out.flush();
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
        let cfg =
            ServerConfig { command: "definitely-not-a-real-binary-oxgraft".into(), args: vec![], env: HashMap::new() };
        match connect("x", &cfg, &std::env::temp_dir()) {
            Err(e) => assert!(e.contains("cannot start"), "{e}"),
            Ok(_) => panic!("expected an error"),
        }
    }
}
