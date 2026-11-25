//! Provider-neutral conversation model (SPEC-0001).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
}

/// One content block of a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
    /// A provider-specific block (e.g. Anthropic `thinking`) that must be sent
    /// back unchanged to the same provider and is dropped for others.
    Opaque {
        provider: String,
        raw: Value,
    },
}

impl Block {
    pub fn text(text: impl Into<String>) -> Self {
        Block::Text { text: text.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Block>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self { role: Role::User, content: vec![Block::text(text)] }
    }

    pub fn assistant(content: Vec<Block>) -> Self {
        Self { role: Role::Assistant, content }
    }

    /// Concatenated text blocks.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| match b {
                Block::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The tool calls in this message, in order.
    pub fn tool_uses(&self) -> Vec<(&str, &str, &Value)> {
        self.content
            .iter()
            .filter_map(|b| match b {
                Block::ToolUse { id, name, input } => Some((id.as_str(), name.as_str(), input)),
                _ => None,
            })
            .collect()
    }
}

/// A tool as advertised to the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema for the input object.
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    Refusal,
    Other(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl Usage {
    pub fn add(&mut self, other: Usage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
    }
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSpec>,
    pub max_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct Completion {
    pub content: Vec<Block>,
    pub stop: StopReason,
    pub usage: Usage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Missing or bad configuration (e.g. no API key). Not retryable.
    Config,
    /// Rejected credentials. Not retryable.
    Auth,
    /// The request was invalid (4xx other than auth/rate). Not retryable.
    BadRequest,
    /// Rate limited, overloaded, server error, timeout, network. Retryable.
    Transient,
    /// The response could not be understood.
    Protocol,
    /// A scripted/mock provider ran out of turns.
    Exhausted,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{provider}: {message}")]
pub struct ProviderError {
    pub provider: String,
    pub kind: ErrorKind,
    pub status: Option<u16>,
    pub message: String,
    /// Server-suggested wait before retrying, in milliseconds.
    pub retry_after_ms: Option<u64>,
}

impl ProviderError {
    pub fn new(provider: &str, kind: ErrorKind, message: impl Into<String>) -> Self {
        Self { provider: provider.into(), kind, status: None, message: message.into(), retry_after_ms: None }
    }

    pub fn retryable(&self) -> bool {
        self.kind == ErrorKind::Transient
    }
}

/// A model backend. Implementations live in `oxgraft-providers`.
pub trait Provider: Send + Sync {
    /// Provider id, e.g. `"anthropic"`; used to match `Block::Opaque`.
    fn id(&self) -> &str;
    /// One request/response round trip. Retries are the caller's job.
    fn complete(&self, request: &CompletionRequest) -> Result<Completion, ProviderError>;

    /// Like `complete`, calling `on_text` with assistant text as it arrives (ADR 0007).
    /// The returned completion is the same as `complete` would return. The default
    /// does not stream: it reports the whole text once at the end.
    fn stream(&self, request: &CompletionRequest, on_text: &mut dyn FnMut(&str)) -> Result<Completion, ProviderError> {
        let completion = self.complete(request)?;
        for block in &completion.content {
            if let Block::Text { text } = block
                && !text.is_empty()
            {
                on_text(text);
            }
        }
        Ok(completion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_round_trip_through_json() {
        let m = Message::assistant(vec![
            Block::text("hi"),
            Block::ToolUse { id: "t1".into(), name: "read".into(), input: serde_json::json!({"path": "a"}) },
            Block::Opaque { provider: "anthropic".into(), raw: serde_json::json!({"type": "thinking"}) },
        ]);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<Message>(&json).unwrap(), m);
        assert_eq!(m.text(), "hi");
        assert_eq!(m.tool_uses().len(), 1);
    }

    #[test]
    fn only_transient_errors_retry() {
        assert!(ProviderError::new("x", ErrorKind::Transient, "").retryable());
        assert!(!ProviderError::new("x", ErrorKind::Auth, "").retryable());
    }
}
