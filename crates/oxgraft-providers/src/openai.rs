//! OpenAI-compatible Chat Completions: OpenAI, OpenRouter, DeepSeek, Groq,
//! Gemini's OpenAI endpoint, xAI, Mistral, Ollama, LM Studio, or any base URL.

use std::time::Duration;

use oxgraft_seams::{
    Block, Completion, CompletionRequest, ErrorKind, Provider, ProviderError, Role, StopReason, Usage,
};
use serde_json::{Value, json};

use crate::http::post_json;

/// A known OpenAI-compatible endpoint.
#[derive(Debug, Clone, Copy)]
pub struct Preset {
    pub id: &'static str,
    pub base_url: &'static str,
    /// Env var holding the API key; `None` for local servers.
    pub key_env: Option<&'static str>,
    /// Env var that overrides the base URL.
    pub base_env: &'static str,
    pub example: &'static str,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "openai",
        base_url: "https://api.openai.com/v1",
        key_env: Some("OPENAI_API_KEY"),
        base_env: "OPENAI_BASE_URL",
        example: "openai/<model>",
    },
    Preset {
        id: "openrouter",
        base_url: "https://openrouter.ai/api/v1",
        key_env: Some("OPENROUTER_API_KEY"),
        base_env: "OPENROUTER_BASE_URL",
        example: "openrouter/<vendor>/<model>",
    },
    Preset {
        id: "deepseek",
        base_url: "https://api.deepseek.com/v1",
        key_env: Some("DEEPSEEK_API_KEY"),
        base_env: "DEEPSEEK_BASE_URL",
        example: "deepseek/<model>",
    },
    Preset {
        id: "groq",
        base_url: "https://api.groq.com/openai/v1",
        key_env: Some("GROQ_API_KEY"),
        base_env: "GROQ_BASE_URL",
        example: "groq/<model>",
    },
    Preset {
        id: "gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        key_env: Some("GEMINI_API_KEY"),
        base_env: "GEMINI_BASE_URL",
        example: "gemini/<model>",
    },
    Preset {
        id: "xai",
        base_url: "https://api.x.ai/v1",
        key_env: Some("XAI_API_KEY"),
        base_env: "XAI_BASE_URL",
        example: "xai/<model>",
    },
    Preset {
        id: "mistral",
        base_url: "https://api.mistral.ai/v1",
        key_env: Some("MISTRAL_API_KEY"),
        base_env: "MISTRAL_BASE_URL",
        example: "mistral/<model>",
    },
    Preset {
        id: "ollama",
        base_url: "http://localhost:11434/v1",
        key_env: None,
        base_env: "OLLAMA_BASE_URL",
        example: "ollama/<model>",
    },
    Preset {
        id: "lmstudio",
        base_url: "http://localhost:1234/v1",
        key_env: None,
        base_env: "LMSTUDIO_BASE_URL",
        example: "lmstudio/<model>",
    },
    Preset {
        id: "compatible",
        base_url: "http://localhost:8000/v1",
        key_env: Some("OXGRAFT_COMPATIBLE_API_KEY"),
        base_env: "OXGRAFT_COMPATIBLE_BASE_URL",
        example: "compatible/<model> (any OpenAI-compatible server)",
    },
];

pub struct OpenAiCompatible {
    preset: Preset,
    base_url: String,
    api_key: Option<String>,
}

impl std::fmt::Debug for OpenAiCompatible {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatible").field("id", &self.preset.id).field("base_url", &self.base_url).finish()
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

impl OpenAiCompatible {
    pub fn from_env(preset: Preset) -> Result<Self, ProviderError> {
        let base_url = env(preset.base_env).unwrap_or_else(|| preset.base_url.into()).trim_end_matches('/').to_string();
        let api_key = preset.key_env.and_then(env);
        let local = preset.key_env.is_none() || preset.id == "compatible";
        if api_key.is_none() && !local {
            return Err(ProviderError::new(
                preset.id,
                ErrorKind::Config,
                format!("no credentials: set {} or pick another model with --model", preset.key_env.unwrap_or("")),
            ));
        }
        Ok(Self { preset, base_url, api_key })
    }

    /// Explicit construction (tests, embedding).
    pub fn new(preset: Preset, base_url: &str, api_key: Option<String>) -> Self {
        Self { preset, base_url: base_url.trim_end_matches('/').into(), api_key }
    }

    pub fn request_body(&self, req: &CompletionRequest) -> Value {
        let mut body = json!({ "model": req.model, "messages": messages_json(req) });
        // OpenAI's own API wants max_completion_tokens; most compatible servers accept max_tokens.
        let key = if self.preset.id == "openai" { "max_completion_tokens" } else { "max_tokens" };
        body[key] = json!(req.max_tokens);
        if !req.tools.is_empty() {
            body["tools"] = req
                .tools
                .iter()
                .map(|t| {
                    json!({"type": "function", "function": {
                        "name": t.name, "description": t.description, "parameters": t.input_schema
                    }})
                })
                .collect();
        }
        body
    }
}

fn messages_json(req: &CompletionRequest) -> Vec<Value> {
    let mut out = Vec::new();
    if !req.system.is_empty() {
        out.push(json!({"role": "system", "content": req.system}));
    }
    for m in &req.messages {
        match m.role {
            Role::User => {
                // Tool results must directly follow the assistant tool_calls message.
                for b in &m.content {
                    if let Block::ToolResult { tool_use_id, content, is_error } = b {
                        let content = if *is_error { format!("ERROR: {content}") } else { content.clone() };
                        out.push(json!({"role": "tool", "tool_call_id": tool_use_id, "content": content}));
                    }
                }
                let text = m.text();
                if !text.is_empty() {
                    out.push(json!({"role": "user", "content": text}));
                }
            }
            Role::Assistant => {
                let calls: Vec<Value> = m
                    .tool_uses()
                    .into_iter()
                    .map(|(id, name, input)| {
                        let arguments = match input {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        json!({"id": id, "type": "function", "function": {"name": name, "arguments": arguments}})
                    })
                    .collect();
                let text = m.text();
                let mut msg =
                    json!({"role": "assistant", "content": if text.is_empty() { Value::Null } else { json!(text) }});
                if !calls.is_empty() {
                    msg["tool_calls"] = json!(calls);
                } else if text.is_empty() {
                    msg["content"] = json!("(empty)");
                }
                out.push(msg);
            }
        }
    }
    out
}

pub fn parse_response(provider: &str, v: &Value) -> Result<Completion, ProviderError> {
    let choice = v
        .pointer("/choices/0")
        .ok_or_else(|| ProviderError::new(provider, ErrorKind::Protocol, "response has no choices"))?;
    let message = &choice["message"];
    let mut content = Vec::new();
    if let Some(text) = message.get("content").and_then(Value::as_str)
        && !text.is_empty()
    {
        content.push(Block::Text { text: text.to_string() });
    }
    let mut has_calls = false;
    if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
        for (i, call) in calls.iter().enumerate() {
            has_calls = true;
            let raw_args = call.pointer("/function/arguments");
            let input = match raw_args {
                Some(Value::String(s)) if s.trim().is_empty() => json!({}),
                // Malformed JSON stays a string; the tool's validation reports it (SPEC-0001 AC4).
                Some(Value::String(s)) => serde_json::from_str(s).unwrap_or_else(|_| Value::String(s.clone())),
                Some(other) => other.clone(),
                None => json!({}),
            };
            content.push(Block::ToolUse {
                id: call.get("id").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("call_{i}")),
                name: call.pointer("/function/name").and_then(Value::as_str).unwrap_or_default().to_string(),
                input,
            });
        }
    }
    let stop = match choice.get("finish_reason").and_then(Value::as_str) {
        _ if has_calls => StopReason::ToolUse,
        Some("stop") | None => StopReason::EndTurn,
        Some("tool_calls") | Some("function_call") => StopReason::ToolUse,
        Some("length") => StopReason::MaxTokens,
        Some("content_filter") => StopReason::Refusal,
        Some(other) => StopReason::Other(other.to_string()),
    };
    let n = |p: &str| v.pointer(p).and_then(Value::as_u64).unwrap_or(0);
    let usage = Usage { input_tokens: n("/usage/prompt_tokens"), output_tokens: n("/usage/completion_tokens") };
    Ok(Completion { content, stop, usage })
}

impl Provider for OpenAiCompatible {
    fn id(&self) -> &str {
        self.preset.id
    }

    fn complete(&self, req: &CompletionRequest) -> Result<Completion, ProviderError> {
        let mut headers = vec![("content-type", "application/json".to_string())];
        if let Some(key) = &self.api_key {
            headers.push(("authorization", format!("Bearer {key}")));
        }
        let url = format!("{}/chat/completions", self.base_url);
        let v = post_json(self.preset.id, &url, &headers, &self.request_body(req), Duration::from_secs(600))?;
        parse_response(self.preset.id, &v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxgraft_seams::{Message, ToolSpec};

    fn provider(id: &str) -> OpenAiCompatible {
        let preset = *PRESETS.iter().find(|p| p.id == id).unwrap();
        OpenAiCompatible { preset, base_url: preset.base_url.into(), api_key: Some("k".into()) }
    }

    // SPEC-0001 AC4
    #[test]
    fn maps_requests() {
        let req = CompletionRequest {
            model: "m".into(),
            system: "sys".into(),
            messages: vec![
                Message::user("hi"),
                Message::assistant(vec![
                    Block::text("calling"),
                    Block::ToolUse { id: "c1".into(), name: "read".into(), input: json!({"path": "a"}) },
                ]),
                Message {
                    role: Role::User,
                    content: vec![
                        Block::ToolResult { tool_use_id: "c1".into(), content: "boom".into(), is_error: true },
                        Block::text("and also"),
                    ],
                },
            ],
            tools: vec![ToolSpec { name: "read".into(), description: "d".into(), input_schema: json!({}) }],
            max_tokens: 100,
        };
        let body = provider("openrouter").request_body(&req);
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[2]["tool_calls"][0]["function"]["arguments"], r#"{"path":"a"}"#);
        assert_eq!(msgs[3]["role"], "tool");
        assert_eq!(msgs[3]["content"], "ERROR: boom");
        assert_eq!(msgs[4]["role"], "user");
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["max_tokens"], 100);
        assert_eq!(provider("openai").request_body(&req)["max_completion_tokens"], 100);
    }

    #[test]
    fn parses_responses_including_malformed_arguments() {
        let v = json!({
            "choices": [{"message": {"content": null, "tool_calls": [
                {"id": "a", "function": {"name": "read", "arguments": "{\"path\":\"x\"}"}},
                {"function": {"name": "bash", "arguments": "{not json"}}
            ]}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 7, "completion_tokens": 2}
        });
        let c = parse_response("x", &v).unwrap();
        assert_eq!(c.stop, StopReason::ToolUse);
        assert_eq!(c.content.len(), 2);
        match &c.content[1] {
            Block::ToolUse { id, input, .. } => {
                assert_eq!(id, "call_1");
                assert!(input.is_string());
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(c.usage.input_tokens, 7);
        let end = json!({"choices": [{"message": {"content": "done"}, "finish_reason": "length"}]});
        assert_eq!(parse_response("x", &end).unwrap().stop, StopReason::MaxTokens);
        assert!(parse_response("x", &json!({})).is_err());
    }
}
