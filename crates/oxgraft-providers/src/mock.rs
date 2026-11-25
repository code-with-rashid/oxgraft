//! Scripted provider for deterministic tests and demos (SPEC-0001 AC7).
//!
//! Script format (JSON):
//! ```json
//! {"turns": [
//!   {"text": "Reading it", "tool_calls": [{"name": "read", "input": {"path": "a.txt"}}]},
//!   {"text": "Done"}
//! ]}
//! ```

use std::collections::VecDeque;
use std::sync::Mutex;

use oxgraft_seams::{Block, Completion, CompletionRequest, ErrorKind, Provider, ProviderError, StopReason, Usage};
use serde::Deserialize;
use serde_json::Value;

pub const ID: &str = "mock";

#[derive(Debug, Clone, Deserialize)]
pub struct Script {
    pub turns: Vec<Turn>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Turn {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub tool_calls: Vec<Call>,
    /// Optional explicit stop reason: end_turn, tool_use, max_tokens, refusal.
    #[serde(default)]
    pub stop: Option<String>,
    /// Simulate a provider error for this turn: "transient", "auth", "bad_request".
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Call {
    pub name: String,
    #[serde(default)]
    pub input: Value,
}

#[derive(Debug)]
pub struct Mock {
    turns: Mutex<VecDeque<Turn>>,
    served: Mutex<usize>,
}

impl Mock {
    pub fn new(script: Script) -> Self {
        Self { turns: Mutex::new(script.turns.into()), served: Mutex::new(0) }
    }

    pub fn from_file(path: &str) -> Result<Self, ProviderError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| ProviderError::new(ID, ErrorKind::Config, format!("cannot read mock script {path}: {e}")))?;
        let script: Script = serde_json::from_str(&text)
            .map_err(|e| ProviderError::new(ID, ErrorKind::Config, format!("invalid mock script {path}: {e}")))?;
        Ok(Self::new(script))
    }
}

impl Provider for Mock {
    fn id(&self) -> &str {
        ID
    }

    fn complete(&self, req: &CompletionRequest) -> Result<Completion, ProviderError> {
        let turn = self
            .turns
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
            .ok_or_else(|| ProviderError::new(ID, ErrorKind::Exhausted, "mock script has no more turns"))?;
        let mut served = self.served.lock().unwrap_or_else(|e| e.into_inner());
        *served += 1;
        if let Some(kind) = &turn.error {
            let kind = match kind.as_str() {
                "transient" => ErrorKind::Transient,
                "auth" => ErrorKind::Auth,
                _ => ErrorKind::BadRequest,
            };
            return Err(ProviderError::new(ID, kind, format!("scripted {kind:?} error")));
        }
        let mut content = Vec::new();
        if !turn.text.is_empty() {
            content.push(Block::text(&turn.text));
        }
        for (i, call) in turn.tool_calls.iter().enumerate() {
            content.push(Block::ToolUse {
                id: format!("mock_{}_{i}", *served),
                name: call.name.clone(),
                input: call.input.clone(),
            });
        }
        let stop = match turn.stop.as_deref() {
            Some("max_tokens") => StopReason::MaxTokens,
            Some("refusal") => StopReason::Refusal,
            Some("end_turn") => StopReason::EndTurn,
            Some("tool_use") => StopReason::ToolUse,
            _ if !turn.tool_calls.is_empty() => StopReason::ToolUse,
            _ => StopReason::EndTurn,
        };
        let input_chars: usize =
            req.messages.iter().map(|m| serde_json::to_string(m).map(|s| s.len()).unwrap_or(0)).sum();
        let usage = Usage { input_tokens: (input_chars / 4) as u64, output_tokens: (turn.text.len() / 4 + 1) as u64 };
        Ok(Completion { content, stop, usage })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxgraft_seams::Message;

    fn req() -> CompletionRequest {
        CompletionRequest {
            model: "m".into(),
            system: String::new(),
            messages: vec![Message::user("x")],
            tools: vec![],
            max_tokens: 1,
        }
    }

    // SPEC-0001 AC7
    #[test]
    fn replays_turns_then_errors() {
        let script: Script = serde_json::from_str(
            r#"{"turns":[{"text":"a","tool_calls":[{"name":"read","input":{"path":"f"}}]},{"text":"b"}]}"#,
        )
        .unwrap();
        let mock = Mock::new(script);
        let first = mock.complete(&req()).unwrap();
        assert_eq!(first.stop, StopReason::ToolUse);
        assert_eq!(first.content.len(), 2);
        assert_eq!(mock.complete(&req()).unwrap().stop, StopReason::EndTurn);
        assert_eq!(mock.complete(&req()).unwrap_err().kind, ErrorKind::Exhausted);
    }
}
