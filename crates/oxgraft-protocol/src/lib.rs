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
    SessionStarted {
        protocol: u32,
        profile: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    StrategySelected {
        seam: String,
        strategy: String,
    },
    Message {
        role: Role,
        text: String,
    },
    ToolCall {
        id: String,
        tool: String,
        input: serde_json::Value,
    },
    ToolResult {
        id: String,
        ok: bool,
        output: String,
    },
    /// A request to the model is about to be sent (GRAFT-0080); UIs can show that
    /// the model is working until text or a tool call arrives.
    TurnStarted {
        turn: u32,
    },
    /// Assistant text as it streams in (ADR 0007). The full text follows in `message`.
    TextDelta {
        text: String,
    },
    ApprovalRequested {
        id: String,
        tool: String,
        resource: String,
        /// Set when the action is risky and is asked about every time (GRAFT-0042).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        risk: Option<String>,
    },
    /// The approval decision for a tool call: `allow`, `deny`, or `ask_declined`.
    ApprovalResolved {
        id: String,
        decision: String,
    },
    /// A retryable provider error; the request will be retried after `wait_ms`.
    Retrying {
        attempt: u32,
        wait_ms: u64,
        message: String,
    },
    /// A non-fatal problem worth surfacing (guard tripped, config note).
    Warning {
        message: String,
    },
    /// Files the run changed, from its first to its last workspace checkpoint
    /// (GRAFT-0063). Emitted just before `session_finished` when checkpoints are on.
    FilesChanged {
        files: Vec<String>,
    },
    SessionFinished {
        ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(default)]
        turns: u32,
        #[serde(default)]
        input_tokens: u64,
        #[serde(default)]
        output_tokens: u64,
    },
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

    pub fn finished(ok: bool) -> Self {
        Event::SessionFinished { ok, reason: None, turns: 0, input_tokens: 0, output_tokens: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_round_trip_through_jsonl() {
        let events = [
            Event::SessionStarted {
                protocol: PROTOCOL_VERSION,
                profile: "default".into(),
                session_id: Some("s".into()),
                model: None,
            },
            Event::StrategySelected { seam: "edit_format".into(), strategy: "search_replace".into() },
            Event::Message { role: Role::Assistant, text: "hello".into() },
            Event::ToolCall { id: "1".into(), tool: "read".into(), input: serde_json::json!({"path": "a.rs"}) },
            Event::ToolResult { id: "1".into(), ok: true, output: "fn main() {}".into() },
            Event::ApprovalRequested {
                id: "2".into(),
                tool: "bash".into(),
                resource: "rm -rf target".into(),
                risk: Some("r".into()),
            },
            Event::ApprovalResolved { id: "2".into(), decision: "deny".into() },
            Event::Retrying { attempt: 1, wait_ms: 1000, message: "429".into() },
            Event::Warning { message: "w".into() },
            Event::TextDelta { text: "par".into() },
            Event::TurnStarted { turn: 1 },
            Event::FilesChanged { files: vec!["src/a.rs".into()] },
            Event::SessionFinished { ok: true, reason: None, turns: 2, input_tokens: 5, output_tokens: 6 },
        ];
        for event in events {
            let line = event.to_jsonl();
            assert!(!line.contains('\n'));
            assert_eq!(Event::from_jsonl(&line).unwrap(), event);
        }
    }

    #[test]
    fn version_1_lines_still_parse() {
        // Lines written before optional fields were added must keep parsing (append-only rule).
        assert_eq!(Event::from_jsonl(r#"{"type":"session_finished","ok":false}"#).unwrap(), Event::finished(false));
        assert!(Event::from_jsonl(r#"{"type":"session_started","protocol":1,"profile":"default"}"#).is_ok());
    }
}
