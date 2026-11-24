//! Anthropic Messages API over raw HTTP (Rust has no official SDK).
//! Shapes follow the API reference for `anthropic-version: 2026-10-21`.

use std::time::Duration;

use oxgraft_seams::{
    Block, Completion, CompletionRequest, ErrorKind, Provider, ProviderError, Role, StopReason, Usage,
};
use serde_json::{Value, json};

use crate::http::post_json;

pub const ID: &str = "anthropic";
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
const API_VERSION: &str = "2026-10-21";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

enum Credential {
    ApiKey(String),
    Bearer(String),
}

pub struct Anthropic {
    credential: Credential,
    base_url: String,
    effort: Option<String>,
    fallbacks: bool,
}

impl std::fmt::Debug for Anthropic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Anthropic").field("base_url", &self.base_url).finish_non_exhaustive()
    }
}

impl Anthropic {
    /// Reads `ANTHROPIC_API_KEY` (or `ANTHROPIC_AUTH_TOKEN`) and `ANTHROPIC_BASE_URL`.
    pub fn from_env(effort: Option<String>) -> Result<Self, ProviderError> {
        let credential = match (env("ANTHROPIC_API_KEY"), env("ANTHROPIC_AUTH_TOKEN")) {
            (Some(key), _) => Credential::ApiKey(key),
            (None, Some(token)) => Credential::Bearer(token),
            (None, None) => {
                return Err(ProviderError::new(
                    ID,
                    ErrorKind::Config,
                    "no credentials: set ANTHROPIC_API_KEY (or ANTHROPIC_AUTH_TOKEN), or pick another model with --model",
                ));
            }
        };
        let base_url = env("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|| "https://api.anthropic.com".into())
            .trim_end_matches('/')
            .into();
        // Server-side fallbacks are only accepted by the first-party API.
        let fallbacks =
            base_url == "https://api.anthropic.com" && env("OXGRAFT_ANTHROPIC_FALLBACKS").as_deref() != Some("0");
        Ok(Self { credential, base_url, effort, fallbacks })
    }

    pub fn request_body(&self, req: &CompletionRequest) -> Value {
        let mut body = json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "messages": messages_json(req),
        });
        if !req.system.is_empty() {
            body["system"] = json!(req.system);
        }
        if !req.tools.is_empty() {
            body["tools"] = req
                .tools
                .iter()
                .map(|t| json!({"name": t.name, "description": t.description, "input_schema": t.input_schema}))
                .collect();
        }
        if let Some(effort) = &self.effort {
            body["output_config"] = json!({ "effort": effort });
        }
        if self.fallbacks && supports_fallbacks(&req.model) {
            body["fallbacks"] = json!("default");
        }
        body
    }
}

fn supports_fallbacks(model: &str) -> bool {
    matches!(model, "claude-opus-5-5" | "claude-opus-5" | "claude-sonnet-5-5" | "claude-fable-5-2" | "claude-fable-5-1")
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn messages_json(req: &CompletionRequest) -> Vec<Value> {
    req.messages
        .iter()
        .map(|m| {
            let mut content: Vec<Value> = m
                .content
                .iter()
                .filter_map(|b| match b {
                    Block::Text { text } if text.is_empty() => None,
                    Block::Text { text } => Some(json!({"type": "text", "text": text})),
                    Block::ToolUse { id, name, input } => {
                        Some(json!({"type": "tool_use", "id": id, "name": name, "input": input}))
                    }
                    Block::ToolResult { tool_use_id, content, is_error } => Some(json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": if content.is_empty() { "(no output)" } else { content.as_str() },
                        "is_error": is_error,
                    })),
                    Block::Opaque { provider, raw } if provider == ID => Some(raw.clone()),
                    Block::Opaque { .. } => None,
                })
                .collect();
            if content.is_empty() {
                content.push(json!({"type": "text", "text": "(empty)"}));
            }
            let role = match m.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };
            json!({"role": role, "content": content})
        })
        .collect()
}

pub fn parse_response(v: &Value) -> Result<Completion, ProviderError> {
    let blocks = v
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::new(ID, ErrorKind::Protocol, "response has no content array"))?;
    let mut content = Vec::with_capacity(blocks.len());
    for b in blocks {
        match b.get("type").and_then(Value::as_str) {
            Some("text") => content.push(Block::Text { text: b["text"].as_str().unwrap_or_default().to_string() }),
            Some("tool_use") => content.push(Block::ToolUse {
                id: b["id"].as_str().unwrap_or_default().to_string(),
                name: b["name"].as_str().unwrap_or_default().to_string(),
                input: b.get("input").cloned().unwrap_or_else(|| json!({})),
            }),
            // thinking, redacted_thinking, fallback, server tool blocks, future types:
            // keep verbatim so they can be sent back unchanged (append-only history).
            _ => content.push(Block::Opaque { provider: ID.into(), raw: b.clone() }),
        }
    }
    let stop = match v.get("stop_reason").and_then(Value::as_str) {
        Some("end_turn") | Some("stop_sequence") => StopReason::EndTurn,
        Some("tool_use") => StopReason::ToolUse,
        Some("max_tokens") => StopReason::MaxTokens,
        Some("refusal") => StopReason::Refusal,
        Some(other) => StopReason::Other(other.to_string()),
        None => StopReason::Other("missing".into()),
    };
    let u = &v["usage"];
    let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
    let usage = Usage {
        input_tokens: n("input_tokens") + n("cache_read_input_tokens") + n("cache_creation_input_tokens"),
        output_tokens: n("output_tokens"),
    };
    Ok(Completion { content, stop, usage })
}

impl Provider for Anthropic {
    fn id(&self) -> &str {
        ID
    }

    fn complete(&self, req: &CompletionRequest) -> Result<Completion, ProviderError> {
        let mut headers =
            vec![("anthropic-version", API_VERSION.to_string()), ("content-type", "application/json".into())];
        match &self.credential {
            Credential::ApiKey(k) => headers.push(("x-api-key", k.clone())),
            Credential::Bearer(t) => headers.push(("authorization", format!("Bearer {t}"))),
        }
        if self.fallbacks && supports_fallbacks(&req.model) {
            headers.push(("anthropic-beta", FALLBACK_BETA.into()));
        }
        let url = format!("{}/v1/messages", self.base_url);
        let v = post_json(ID, &url, &headers, &self.request_body(req), Duration::from_secs(600))?;
        parse_response(&v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxgraft_seams::{Message, ToolSpec};

    fn provider() -> Anthropic {
        Anthropic {
            credential: Credential::ApiKey("k".into()),
            base_url: "https://api.anthropic.com".into(),
            effort: Some("high".into()),
            fallbacks: true,
        }
    }

    // SPEC-0001 AC2
    #[test]
    fn serialises_requests_in_api_shape() {
        let thinking = json!({"type": "thinking", "thinking": "", "signature": "sig"});
        let req = CompletionRequest {
            model: DEFAULT_MODEL.into(),
            system: "sys".into(),
            messages: vec![
                Message::user("hi"),
                Message::assistant(vec![
                    Block::Opaque { provider: ID.into(), raw: thinking.clone() },
                    Block::Opaque { provider: "openai".into(), raw: json!({}) },
                    Block::ToolUse { id: "t1".into(), name: "read".into(), input: json!({"path": "a"}) },
                ]),
                Message {
                    role: Role::User,
                    content: vec![Block::ToolResult { tool_use_id: "t1".into(), content: "x".into(), is_error: false }],
                },
            ],
            tools: vec![ToolSpec {
                name: "read".into(),
                description: "d".into(),
                input_schema: json!({"type": "object"}),
            }],
            max_tokens: 16000,
        };
        let body = provider().request_body(&req);
        assert_eq!(body["system"], "sys");
        assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
        assert_eq!(body["output_config"]["effort"], "high");
        assert_eq!(body["fallbacks"], "default");
        let assistant = body["messages"][1]["content"].as_array().unwrap();
        assert_eq!(assistant.len(), 2, "foreign opaque block dropped");
        assert_eq!(assistant[0], thinking, "own opaque block sent back verbatim");
        assert_eq!(body["messages"][2]["content"][0]["type"], "tool_result");
        assert!(body.get("temperature").is_none());
    }

    // SPEC-0001 AC3
    #[test]
    fn parses_responses() {
        let v = json!({
            "content": [
                {"type": "thinking", "thinking": "", "signature": "s"},
                {"type": "text", "text": "ok"},
                {"type": "tool_use", "id": "t", "name": "bash", "input": {"command": "ls"}}
            ],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "cache_read_input_tokens": 5, "output_tokens": 3}
        });
        let c = parse_response(&v).unwrap();
        assert_eq!(c.stop, StopReason::ToolUse);
        assert!(matches!(c.content[0], Block::Opaque { .. }));
        assert_eq!(c.usage, Usage { input_tokens: 15, output_tokens: 3 });
        assert_eq!(
            parse_response(&json!({"content": [], "stop_reason": "refusal"})).unwrap().stop,
            StopReason::Refusal
        );
        assert!(parse_response(&json!({})).is_err());
    }

    #[test]
    fn fallbacks_only_for_supported_models() {
        let mut req = CompletionRequest {
            model: "claude-haiku-5-5".into(),
            system: String::new(),
            messages: vec![Message::user("x")],
            tools: vec![],
            max_tokens: 10,
        };
        assert!(provider().request_body(&req).get("fallbacks").is_none());
        req.model = DEFAULT_MODEL.into();
        assert!(provider().request_body(&req).get("fallbacks").is_some());
    }
}
