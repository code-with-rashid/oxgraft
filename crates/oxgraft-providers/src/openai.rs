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
    /// Send `stream_options.include_usage` when streaming (servers known to accept it).
    pub stream_usage: bool,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "openai",
        base_url: "https://api.openai.com/v1",
        key_env: Some("OPENAI_API_KEY"),
        base_env: "OPENAI_BASE_URL",
        stream_usage: true,
        example: "openai/<model>",
    },
    Preset {
        id: "openrouter",
        base_url: "https://openrouter.ai/api/v1",
        key_env: Some("OPENROUTER_API_KEY"),
        base_env: "OPENROUTER_BASE_URL",
        stream_usage: true,
        example: "openrouter/<vendor>/<model>",
    },
    Preset {
        id: "deepseek",
        base_url: "https://api.deepseek.com/v1",
        key_env: Some("DEEPSEEK_API_KEY"),
        base_env: "DEEPSEEK_BASE_URL",
        stream_usage: true,
        example: "deepseek/<model>",
    },
    Preset {
        id: "groq",
        base_url: "https://api.groq.com/openai/v1",
        key_env: Some("GROQ_API_KEY"),
        base_env: "GROQ_BASE_URL",
        stream_usage: false,
        example: "groq/<model>",
    },
    Preset {
        id: "gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        key_env: Some("GEMINI_API_KEY"),
        base_env: "GEMINI_BASE_URL",
        stream_usage: false,
        example: "gemini/<model>",
    },
    Preset {
        id: "xai",
        base_url: "https://api.x.ai/v1",
        key_env: Some("XAI_API_KEY"),
        base_env: "XAI_BASE_URL",
        stream_usage: false,
        example: "xai/<model>",
    },
    Preset {
        id: "mistral",
        base_url: "https://api.mistral.ai/v1",
        key_env: Some("MISTRAL_API_KEY"),
        base_env: "MISTRAL_BASE_URL",
        stream_usage: false,
        example: "mistral/<model>",
    },
    Preset {
        id: "ollama",
        base_url: "http://localhost:11434/v1",
        key_env: None,
        base_env: "OLLAMA_BASE_URL",
        stream_usage: false,
        example: "ollama/<model>",
    },
    Preset {
        id: "lmstudio",
        base_url: "http://localhost:1234/v1",
        key_env: None,
        base_env: "LMSTUDIO_BASE_URL",
        stream_usage: false,
        example: "lmstudio/<model>",
    },
    Preset {
        id: "compatible",
        base_url: "http://localhost:8000/v1",
        key_env: Some("OXGRAFT_COMPATIBLE_API_KEY"),
        base_env: "OXGRAFT_COMPATIBLE_BASE_URL",
        stream_usage: false,
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

impl OpenAiCompatible {
    fn headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![("content-type", "application/json".to_string())];
        if let Some(key) = &self.api_key {
            headers.push(("authorization", format!("Bearer {key}")));
        }
        headers
    }
}

/// Rebuilds a chat completion from its streamed chunks (ADR 0007), ending in the
/// same `parse_response` as the blocking path.
#[derive(Debug, Default)]
pub struct StreamState {
    text: String,
    calls: Vec<Value>,
    finish_reason: Value,
    usage: Value,
    done: bool,
}

impl StreamState {
    pub fn apply(&mut self, provider: &str, data: &str, on_text: &mut dyn FnMut(&str)) -> Result<(), ProviderError> {
        if data.trim() == "[DONE]" {
            self.done = true;
            return Ok(());
        }
        let v: Value = serde_json::from_str(data)
            .map_err(|e| ProviderError::new(provider, ErrorKind::Protocol, format!("bad stream chunk: {e}")))?;
        if let Some(err) = v.get("error") {
            let message = err.get("message").and_then(Value::as_str).unwrap_or("stream error").to_string();
            return Err(ProviderError::new(provider, ErrorKind::Transient, format!("stream error: {message}")));
        }
        if v.get("usage").is_some_and(Value::is_object) {
            self.usage = v["usage"].clone();
        }
        let Some(choice) = v.pointer("/choices/0") else { return Ok(()) };
        let delta = &choice["delta"];
        if let Some(t) = delta.get("content").and_then(Value::as_str)
            && !t.is_empty()
        {
            self.text.push_str(t);
            on_text(t);
        }
        for (pos, call) in delta.get("tool_calls").and_then(Value::as_array).into_iter().flatten().enumerate() {
            let index = call.get("index").and_then(Value::as_u64).map_or(pos, |i| i as usize);
            while self.calls.len() <= index {
                self.calls.push(json!({"function": {"name": "", "arguments": ""}}));
            }
            let slot = &mut self.calls[index];
            if let Some(id) = call.get("id").and_then(Value::as_str) {
                slot["id"] = json!(id);
            }
            for key in ["name", "arguments"] {
                if let Some(piece) = call.pointer(&format!("/function/{key}")).and_then(Value::as_str) {
                    let current = slot["function"][key].as_str().unwrap_or_default().to_string();
                    slot["function"][key] = json!(current + piece);
                }
            }
        }
        if let Some(reason) = choice.get("finish_reason").filter(|r| !r.is_null()) {
            self.finish_reason = reason.clone();
        }
        Ok(())
    }

    pub fn finish(self, provider: &str) -> Result<Completion, ProviderError> {
        // Some servers close the stream without `[DONE]`; a finish reason is enough.
        if !self.done && self.finish_reason.is_null() {
            return Err(ProviderError::new(provider, ErrorKind::Transient, "stream ended before the reply finished"));
        }
        let mut message = json!({"role": "assistant", "content": self.text});
        if !self.calls.is_empty() {
            message["tool_calls"] = Value::Array(self.calls);
        }
        let mut response = json!({"choices": [{"message": message, "finish_reason": self.finish_reason}]});
        if self.usage.is_object() {
            response["usage"] = self.usage;
        }
        parse_response(provider, &response)
    }
}

impl Provider for OpenAiCompatible {
    fn id(&self) -> &str {
        self.preset.id
    }

    fn complete(&self, req: &CompletionRequest) -> Result<Completion, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let v = post_json(self.preset.id, &url, &self.headers(), &self.request_body(req), Duration::from_secs(600))?;
        parse_response(self.preset.id, &v)
    }

    fn stream(&self, req: &CompletionRequest, on_text: &mut dyn FnMut(&str)) -> Result<Completion, ProviderError> {
        let mut body = self.request_body(req);
        body["stream"] = json!(true);
        if self.preset.stream_usage {
            body["stream_options"] = json!({"include_usage": true});
        }
        let url = format!("{}/chat/completions", self.base_url);
        let mut state = StreamState::default();
        let mut shown = false;
        let result = crate::http::post_sse(
            self.preset.id,
            &url,
            &self.headers(),
            &body,
            Duration::from_secs(600),
            &mut |_, data| {
                state.apply(self.preset.id, data, &mut |t| {
                    shown = true;
                    on_text(t)
                })
            },
        );
        match result {
            // A server that rejects streaming (or its options) gets a plain request instead.
            Err(e) if e.kind == ErrorKind::BadRequest && e.status.is_some() && !shown => {
                let completion = self.complete(req)?;
                let text: String = completion
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        Block::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                if !text.is_empty() {
                    on_text(&text);
                }
                Ok(completion)
            }
            Err(e) => Err(e),
            Ok(()) => state.finish(self.preset.id),
        }
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

    // SPEC-0001 AC7
    #[test]
    fn stream_chunks_rebuild_the_response() {
        let chunks = [
            r#"{"choices":[{"index":0,"delta":{"role":"assistant","content":""}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"content":"Hi "}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"content":"there"}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"c1","type":"function","function":{"name":"read","arguments":""}}]}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":"}}]}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"a\"}"}}]}}]}"#,
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
            r#"{"choices":[],"usage":{"prompt_tokens":7,"completion_tokens":5}}"#,
            "[DONE]",
        ];
        let mut state = StreamState::default();
        let mut text = String::new();
        for c in chunks {
            state.apply("openai", c, &mut |t| text.push_str(t)).unwrap();
        }
        assert_eq!(text, "Hi there");
        let c = state.finish("openai").unwrap();
        assert_eq!(c.stop, StopReason::ToolUse);
        assert_eq!(c.usage, Usage { input_tokens: 7, output_tokens: 5 });
        assert_eq!(c.content[0], Block::Text { text: "Hi there".into() });
        assert_eq!(c.content[1], Block::ToolUse { id: "c1".into(), name: "read".into(), input: json!({"path": "a"}) });

        let mut state = StreamState::default();
        state.apply("x", r#"{"choices":[{"delta":{"content":"cut"}}]}"#, &mut |_| {}).unwrap();
        assert_eq!(state.finish("x").unwrap_err().kind, ErrorKind::Transient);
        let mut state = StreamState::default();
        assert!(state.apply("x", r#"{"error":{"message":"overloaded"}}"#, &mut |_| {}).is_err());
    }
}
