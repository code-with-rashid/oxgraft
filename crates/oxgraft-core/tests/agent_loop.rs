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

fn run_with(
    ws: &Ws,
    script: serde_json::Value,
    configure: impl FnOnce(&mut AgentConfig),
    session: Option<Session>,
) -> (Status, Vec<Event>, Session) {
    let script: Script = serde_json::from_value(script).unwrap();
    let provider = Mock::new(script);
    let toolkit = toolkit("allow_all");
    let approver = Fixed(Answer::Yes);
    let mut config = AgentConfig::new("mock", ws.0.clone());
    configure(&mut config);
    let mut agent = Agent::new(&toolkit, &provider, oxgraft_tools::builtin(), &approver, config);
    agent.sleep = |_| {};
    let mut session = session.unwrap_or_else(|| Session::new("mock", "test", &ws.0));
    let mut events = Vec::new();
    let outcome = agent.run(&mut session, "task", &mut |e| events.push(e.clone()));
    (outcome.status, events, session)
}

#[test]
fn cancellation_stops_before_the_next_turn() {
    let ws = Ws::new("cancel");
    let (status, _, _) = run_with(
        &ws,
        json!({"turns": [{"text": "never"}]}),
        |c| c.cancel.store(true, std::sync::atomic::Ordering::Relaxed),
        None,
    );
    assert_eq!(status, Status::Stopped("interrupted by the user".into()));
}

#[test]
fn token_budget_stops_runaway_sessions() {
    let ws = Ws::new("budget");
    let turns: Vec<_> =
        (0..10).map(|i| json!({"tool_calls": [{"name": "ls", "input": {"path": ".".repeat(1 + i % 2)}}]})).collect();
    let (status, _, _) = run_with(&ws, json!({ "turns": turns }), |c| c.max_session_tokens = Some(1), None);
    assert!(matches!(status, Status::Stopped(ref r) if r.contains("token budget")), "{status:?}");
}

#[test]
fn hooks_block_and_give_feedback() {
    let ws = Ws::new("hooks");
    let hooks: Vec<oxgraft_core::hooks::Hook> =
        toml::from_str::<std::collections::HashMap<String, Vec<oxgraft_core::hooks::Hook>>>(
            "[[hooks]]\nevent = \"pre_tool\"\ntools = [\"bash\"]\ncommand = \"echo no shell today 1>&2 && exit 2\"\n\
         [[hooks]]\nevent = \"post_tool\"\ntools = [\"write\"]\ncommand = \"echo formatted 1>&2 && exit 2\"\n",
        )
        .unwrap()
        .remove("hooks")
        .unwrap();
    let (_, events, _) = run_with(
        &ws,
        json!({"turns": [
            {"tool_calls": [{"name": "bash", "input": {"command": "echo hi"}}, {"name": "write", "input": {"path": "f.txt", "content": "x"}}]},
            {"text": "ok"}
        ]}),
        |c| c.hooks = hooks,
        None,
    );
    let results = tool_results(&events);
    assert!(results[0].1.contains("Blocked by a pre_tool hook: no shell today"), "{:?}", results[0]);
    assert!(results[1].0 && results[1].1.contains("Feedback from a post_tool hook:\nformatted"), "{:?}", results[1]);
}

#[test]
fn resume_repairs_dangling_tool_calls() {
    let ws = Ws::new("repair");
    let mut session = Session::new("mock", "test", &ws.0);
    session.messages.push(oxgraft_seams::Message::user("earlier"));
    session.messages.push(oxgraft_seams::Message::assistant(vec![oxgraft_seams::Block::ToolUse {
        id: "dangling".into(),
        name: "ls".into(),
        input: json!({}),
    }]));
    let (status, events, session) = run_with(&ws, json!({"turns": [{"text": "fine"}]}), |_| {}, Some(session));
    assert_eq!(status, Status::Done);
    assert!(events.iter().any(|e| matches!(e, Event::Warning { message } if message.contains("unanswered"))));
    assert!(
        matches!(&session.messages[2].content[0], oxgraft_seams::Block::ToolResult { tool_use_id, .. } if tool_use_id == "dangling")
    );
}

#[test]
fn compacts_context_and_continues() {
    let ws = Ws::new("compact");
    let (status, events, session) = run_with(
        &ws,
        json!({"turns": [
            {"tool_calls": [{"name": "ls", "input": {}}]},
            {"text": "SUMMARY: listed the workspace, nothing else yet"},
            {"text": "all done"}
        ]}),
        |c| c.context_window = Some(10),
        None,
    );
    assert_eq!(status, Status::Done);
    assert_eq!(session.compactions, 1);
    assert!(session.messages[0].text().contains("SUMMARY: listed the workspace"));
    assert!(session.messages[0].text().starts_with("task"));
    assert!(events.iter().any(|e| matches!(e, Event::Warning { message } if message.contains("compacted"))));
}

#[test]
fn subagent_explores_read_only_and_reports() {
    let ws = Ws::new("subagent");
    std::fs::write(ws.0.join("needle.txt"), "the answer is 42\n").unwrap();
    let (status, events, _) = run_with(
        &ws,
        json!({"turns": [
            {"tool_calls": [{"name": "task", "input": {"description": "find answer", "prompt": "find the answer"}}]},
            {"tool_calls": [{"name": "grep", "input": {"pattern": "answer"}}, {"name": "write", "input": {"path": "x", "content": "x"}}]},
            {"text": "REPORT: needle.txt:1 says 42"},
            {"text": "The answer is 42."}
        ]}),
        |_| {},
        None,
    );
    assert_eq!(status, Status::Done);
    let results = tool_results(&events);
    assert_eq!(results.len(), 1, "subagent events stay internal: {results:?}");
    assert!(results[0].0 && results[0].1.contains("REPORT: needle.txt:1"), "{:?}", results[0]);
    assert!(!ws.0.join("x").exists(), "subagents cannot write");
    assert_eq!(events.iter().filter(|e| matches!(e, Event::SessionFinished { .. })).count(), 1);
}

#[test]
fn subagent_sessions_are_not_listed() {
    let ws = Ws::new("sublist");
    let (_, _, session) = run_with(
        &ws,
        json!({"turns": [
            {"tool_calls": [{"name": "task", "input": {"prompt": "look"}}]},
            {"text": "REPORT"},
            {"text": "done"}
        ]}),
        |_| {},
        None,
    );
    let listed = Session::list(&ws.0);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, session.id);
}

#[test]
fn edits_require_a_fresh_read() {
    let ws = Ws::new("readfirst");
    std::fs::write(ws.0.join("a.txt"), "one\n").unwrap();
    let edit = "<<<<<<< SEARCH\none\n=======\ntwo\n>>>>>>> REPLACE";
    let edit2 = "<<<<<<< SEARCH\ntwo\n=======\nthree\n>>>>>>> REPLACE";
    let (_, events, _) = run_with(
        &ws,
        json!({"turns": [
            {"tool_calls": [{"name": "edit", "input": {"path": "a.txt", "edit": edit}}]},
            {"tool_calls": [{"name": "read", "input": {"path": "a.txt"}}]},
            {"tool_calls": [{"name": "edit", "input": {"path": "a.txt", "edit": edit}}]},
            {"tool_calls": [{"name": "edit", "input": {"path": "a.txt", "edit": edit2}}]},
            {"text": "done"}
        ]}),
        |_| {},
        None,
    );
    let r = tool_results(&events);
    assert!(!r[0].0 && r[0].1.contains("Read `a.txt` before"), "{:?}", r[0]);
    assert!(r[2].0, "edit after read works: {:?}", r[2]);
    assert!(r[3].0, "consecutive edits after our own write work: {:?}", r[3]);
    assert_eq!(std::fs::read_to_string(ws.0.join("a.txt")).unwrap(), "three\n");
}

#[test]
fn created_files_can_be_edited_and_paths_normalise() {
    let ws = Ws::new("normalise");
    let edit = "<<<<<<< SEARCH\nnew\n=======\nnewer\n>>>>>>> REPLACE";
    let (_, events, _) = run_with(
        &ws,
        json!({"turns": [
            {"tool_calls": [{"name": "write", "input": {"path": "./made.txt", "content": "new\n"}}]},
            {"tool_calls": [{"name": "edit", "input": {"path": "made.txt", "edit": edit}}]},
            {"text": "done"}
        ]}),
        |_| {},
        None,
    );
    let r = tool_results(&events);
    assert!(r[1].0, "{:?}", r[1]);
    assert_eq!(std::fs::read_to_string(ws.0.join("made.txt")).unwrap(), "newer\n");
}
