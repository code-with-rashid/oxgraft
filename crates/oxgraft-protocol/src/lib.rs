//! The headless event stream: Oxgraft's contract with agentic-arena, CI, and
//! any future UI (TUI, IDE extension, web console).
//!
//! Rules (see `docs/adr/0004-versioned-event-protocol.md`):
//! - Events are append-only. Never remove or rename a variant or field.
//! - New optional fields are fine; anything breaking bumps [`PROTOCOL_VERSION`].
//! - One event per line (JSONL), tagged by `"type"`.

use serde::{Deserialize, Serialize};

/// Bump only for breaking changes to the event schema.
pub const PROTOCOL_VERSION: u32 = 1;

/// Who produced a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// One line of the headless event stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    SessionStarted { protocol: u32, profile: String },
    StrategySelected { seam: String, strategy: String },
    Message { role: Role, text: String },
    ToolCall { id: String, tool: String, input: serde_json::Value },
    ToolResult { id: String, ok: bool, output: String },
    ApprovalRequested { id: String, tool: String, resource: String },
    SessionFinished { ok: bool },
}

impl Event {
    /// Serialize as a single JSONL line (no trailing newline).
    pub fn to_jsonl(&self) -> String {
        serde_json::to_string(self).expect("protocol events always serialize")
    }

    /// Parse one JSONL line.
    pub fn from_jsonl(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_round_trip_through_jsonl() {
        let events = [
            Event::SessionStarted { protocol: PROTOCOL_VERSION, profile: "default".into() },
            Event::StrategySelected { seam: "edit_format".into(), strategy: "search_replace".into() },
            Event::Message { role: Role::Assistant, text: "hello".into() },
            Event::ToolCall { id: "1".into(), tool: "read".into(), input: serde_json::json!({"path": "a.rs"}) },
            Event::ToolResult { id: "1".into(), ok: true, output: "fn main() {}".into() },
            Event::ApprovalRequested { id: "2".into(), tool: "bash".into(), resource: "rm -rf target".into() },
            Event::SessionFinished { ok: true },
        ];
        for event in events {
            let line = event.to_jsonl();
            assert!(!line.contains('\n'));
            assert_eq!(Event::from_jsonl(&line).unwrap(), event);
        }
    }

    #[test]
    fn events_are_tagged_by_snake_case_type() {
        let line = Event::SessionFinished { ok: false }.to_jsonl();
        assert_eq!(line, r#"{"type":"session_finished","ok":false}"#);
    }
}
