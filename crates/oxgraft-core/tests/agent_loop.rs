//! Agent loop behaviour against the scripted mock provider (SPEC-0003).

use std::path::PathBuf;

use oxgraft_core::{Agent, AgentConfig, Answer, Available, Fixed, Profile, Session, Status, Toolkit};
use oxgraft_protocol::Event;
use oxgraft_providers::mock::{Mock, Script};
use serde_json::json;

struct Ws(PathBuf);

impl Ws {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("oxgraft-agent-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Ws(dir.canonicalize().unwrap())
    }
}

impl Drop for Ws {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn toolkit(approval: &str) -> Toolkit {
    let profile = Profile::from_toml(&format!(
        "name = \"test\"\n[strategies]\nedit_format = \"search_replace\"\ncontext = \"pinned_then_in_order\"\napproval = \"{approval}\"\nsandbox = \"none\"\n"
    ))
    .unwrap();
    let available = Available {
        edit_formats: oxgraft_strategies::edit_formats(),
        contexts: oxgraft_strategies::contexts(),
        approvals: oxgraft_strategies::approvals(),
        sandboxes: oxgraft_strategies::sandboxes(),
    };
    Toolkit::assemble(profile, available).unwrap()
}

fn run(
    ws: &Ws,
    approval: &str,
    answer: Answer,
    script: serde_json::Value,
    max_turns: u32,
) -> (Status, Vec<Event>, Session) {
    let script: Script = serde_json::from_value(script).unwrap();
    let provider = Mock::new(script);
    let toolkit = toolkit(approval);
    let approver = Fixed(answer);
    let mut config = AgentConfig::new("mock", ws.0.clone());
    config.max_turns = max_turns;
    let mut agent = Agent::new(&toolkit, &provider, oxgraft_tools::builtin(), &approver, config);
    agent.sleep = |_| {};
    let mut session = Session::new("mock", "test", &ws.0);
    let mut events = Vec::new();
    let outcome = agent.run(&mut session, "do the task", &mut |e| events.push(e.clone()));
    (outcome.status, events, session)
}

fn tool_results(events: &[Event]) -> Vec<(bool, String)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::ToolResult { ok, output, .. } => Some((*ok, output.clone())),
            _ => None,
        })
        .collect()
}

// SPEC-0003 AC6 + SPEC-0004 AC6 (core half): read, edit, verify, finish.
#[test]
fn completes_a_read_edit_run_task() {
    let ws = Ws::new("happy");
    std::fs::write(ws.0.join("greet.txt"), "hello world\n").unwrap();
    let edit = "<<<<<<< SEARCH\nhello world\n=======\nhello oxgraft\n>>>>>>> REPLACE";
    let (status, events, session) = run(
        &ws,
        "allow_all",
        Answer::Yes,
        json!({"turns": [
            {"text": "Looking", "tool_calls": [{"name": "read", "input": {"path": "greet.txt"}}]},
            {"tool_calls": [{"name": "edit", "input": {"path": "greet.txt", "edit": edit}}]},
            {"tool_calls": [{"name": "grep", "input": {"pattern": "oxgraft"}}]},
            {"text": "Changed the greeting."}
        ]}),
        10,
    );
    assert_eq!(status, Status::Done);
    assert_eq!(std::fs::read_to_string(ws.0.join("greet.txt")).unwrap(), "hello oxgraft\n");
    assert!(matches!(events.first(), Some(Event::SessionStarted { .. })));
    assert_eq!(events.iter().filter(|e| matches!(e, Event::SessionFinished { .. })).count(), 1);
    assert!(matches!(events.last(), Some(Event::SessionFinished { ok: true, turns: 4, .. })));
    assert_eq!(events.iter().filter(|e| matches!(e, Event::StrategySelected { .. })).count(), 4);
    assert!(tool_results(&events).iter().all(|(ok, _)| *ok));
    // user, (assistant, results) x3, assistant
    assert_eq!(session.messages.len(), 8);
    assert_eq!(session.changed_files.len(), 1);
    let saved = Session::load(&ws.0, &session.id).unwrap();
    assert_eq!(saved.messages.len(), 8);
    saved.undo();
    assert_eq!(std::fs::read_to_string(ws.0.join("greet.txt")).unwrap(), "hello world\n");
}

// SPEC-0003 AC1
#[test]
fn stops_at_max_turns() {
    let ws = Ws::new("maxturns");
    let turns: Vec<_> = (0..5)
        .map(|i| json!({"tool_calls": [{"name": "ls", "input": {"path": format!("./{}", ".".repeat(i % 2))}}]}))
        .collect();
    let (status, events, _) = run(&ws, "allow_all", Answer::Yes, json!({ "turns": turns }), 2);
    assert_eq!(status, Status::Stopped("reached max turns (2)".into()));
    assert!(matches!(events.last(), Some(Event::SessionFinished { ok: false, .. })));
}

// SPEC-0003 AC2
#[test]
fn blocks_repeated_identical_calls() {
    let ws = Ws::new("loop");
    let call = json!({"tool_calls": [{"name": "ls", "input": {}}]});
    let (status, events, _) =
        run(&ws, "allow_all", Answer::Yes, json!({"turns": [call, call, call, call, {"text": "ok"}]}), 10);
    assert_eq!(status, Status::Done);
    let results = tool_results(&events);
    assert!(results[..3].iter().all(|(ok, _)| *ok));
    assert!(!results[3].0 && results[3].1.contains("Blocked"), "{:?}", results[3]);
}

// SPEC-0003 AC3, SPEC-0002 AC1
#[test]
fn unknown_tools_and_bad_input_become_errors() {
    let ws = Ws::new("unknown");
    let (status, events, _) = run(
        &ws,
        "allow_all",
        Answer::Yes,
        json!({"turns": [
            {"tool_calls": [{"name": "teleport", "input": {}}, {"name": "read", "input": "{broken"}]},
            {"text": "giving up"}
        ]}),
        5,
    );
    assert_eq!(status, Status::Done);
    let results = tool_results(&events);
    assert!(results[0].1.contains("available tools: read, write, edit"));
    assert!(results[1].1.contains("JSON object"));
}

// SPEC-0003 step 4: headless approver declines `ask`; deny rules always deny.
#[test]
fn approvals_are_enforced() {
    let ws = Ws::new("approve");
    let (_, events, _) = run(
        &ws,
        "ordered_rules",
        Answer::No,
        json!({"turns": [
            {"tool_calls": [
                {"name": "write", "input": {"path": "x.txt", "content": "x"}},
                {"name": "read", "input": {"path": "secret.env"}}
            ]},
            {"text": "ok"}
        ]}),
        5,
    );
    let results = tool_results(&events);
    assert!(results[0].1.contains("user declined"), "{:?}", results[0]);
    assert!(results[1].1.contains("Denied by the approval policy"), "{:?}", results[1]);
    assert!(!ws.0.join("x.txt").exists());
    assert!(events.iter().any(|e| matches!(e, Event::ApprovalRequested { tool, .. } if tool == "write")));
}

#[test]
fn always_answer_is_remembered_for_the_tool() {
    let ws = Ws::new("always");
    let (_, events, _) = run(
        &ws,
        "ordered_rules",
        Answer::Always,
        json!({"turns": [
            {"tool_calls": [{"name": "write", "input": {"path": "a.txt", "content": "a"}}]},
            {"tool_calls": [{"name": "write", "input": {"path": "b.txt", "content": "b"}}]},
            {"text": "ok"}
        ]}),
        5,
    );
    let asks = events.iter().filter(|e| matches!(e, Event::ApprovalRequested { .. })).count();
    assert_eq!(asks, 1);
    assert!(ws.0.join("b.txt").exists());
}

// SPEC-0003 AC5 and the truncated-tool-call guard.
#[test]
fn handles_max_tokens() {
    let ws = Ws::new("maxtok");
    let (status, events, _) = run(
        &ws,
        "allow_all",
        Answer::Yes,
        json!({"turns": [
            {"text": "part one", "stop": "max_tokens"},
            {"tool_calls": [{"name": "write", "input": {"path": "big.txt", "content": "trunc"}}], "stop": "max_tokens"},
            {"text": "done"}
        ]}),
        5,
    );
    assert_eq!(status, Status::Done);
    assert!(!ws.0.join("big.txt").exists(), "truncated tool call must not run");
    assert!(tool_results(&events)[0].1.contains("output token limit"));
}

#[test]
fn retries_transient_errors_and_fails_cleanly_on_fatal_ones() {
    let ws = Ws::new("errors");
    let (status, events, _) =
        run(&ws, "allow_all", Answer::Yes, json!({"turns": [{"error": "transient"}, {"text": "recovered"}]}), 5);
    assert_eq!(status, Status::Done);
    assert!(events.iter().any(|e| matches!(e, Event::Retrying { .. })));

    let (status, events, _) = run(&ws, "allow_all", Answer::Yes, json!({"turns": [{"error": "auth"}]}), 5);
    assert!(matches!(status, Status::Failed(_)));
    assert!(matches!(events.last(), Some(Event::SessionFinished { ok: false, .. })));
}

#[test]
fn refusal_stops_the_session() {
    let ws = Ws::new("refusal");
    let (status, _, _) = run(&ws, "allow_all", Answer::Yes, json!({"turns": [{"text": "no", "stop": "refusal"}]}), 5);
    assert_eq!(status, Status::Stopped("the model refused".into()));
}
