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
        // Malformed replies are re-asked twice (GRAFT-0044); the third runs and fails normally.
        json!({"turns": [
            {"tool_calls": [{"name": "teleport", "input": {}}, {"name": "read", "input": "{broken"}]},
            {"tool_calls": [{"name": "teleport", "input": {}}, {"name": "read", "input": "{broken"}]},
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

// SPEC-0003 AC8: malformed tool calls are re-asked privately and kept out of history.
#[test]
fn malformed_tool_calls_are_requeried_out_of_history() {
    let ws = Ws::new("requery");
    std::fs::write(ws.0.join("a.txt"), "alpha\n").unwrap();
    let (status, events, session) = run(
        &ws,
        "allow_all",
        Answer::Yes,
        json!({"turns": [
            {"tool_calls": [{"name": "reed", "input": {"path": "a.txt"}}]},
            {"tool_calls": [{"name": "read", "input": "{\"path\": \"a.txt\""}]},
            {"tool_calls": [{"name": "read", "input": {"path": "a.txt"}}]},
            {"text": "done"}
        ]}),
        5,
    );
    assert_eq!(status, Status::Done);
    let warnings =
        events.iter().filter(|e| matches!(e, Event::Warning { message } if message.contains("malformed"))).count();
    assert_eq!(warnings, 2);
    let results = tool_results(&events);
    assert_eq!(results.len(), 1, "only the corrected call runs: {results:?}");
    assert!(results[0].0 && results[0].1.contains("alpha"));
    let history = serde_json::to_string(&session.messages).unwrap();
    assert!(!history.contains("reed") && !history.contains("not a valid JSON"), "{history}");
    // task, assistant(read), results, assistant(done)
    assert_eq!(session.messages.len(), 4);

    // After two re-asks the third malformed reply fails normally and is recorded.
    let ws = Ws::new("requery2");
    let (_, events, session) = run(
        &ws,
        "allow_all",
        Answer::Yes,
        json!({"turns": [
            {"tool_calls": [{"name": "bash", "input": {}}]},
            {"tool_calls": [{"name": "bash", "input": {}}]},
            {"tool_calls": [{"name": "bash", "input": {}}]},
            {"text": "gave up"}
        ]}),
        5,
    );
    let results = tool_results(&events);
    assert_eq!(results.len(), 1);
    assert!(!results[0].0);
    assert_eq!(session.messages.len(), 4);
}

// SPEC-0006 AC6: risky shell commands are asked about every time, even after "always".
#[test]
fn risky_commands_ignore_always_answers() {
    let ws = Ws::new("rails");
    let (_, events, _) = run(
        &ws,
        "ordered_rules",
        Answer::Always,
        json!({"turns": [
            {"tool_calls": [{"name": "bash", "input": {"command": "echo hi"}}]},
            {"tool_calls": [{"name": "bash", "input": {"command": "echo again"}}]},
            {"tool_calls": [{"name": "bash", "input": {"command": "rm -rf not_there"}}]},
            {"tool_calls": [{"name": "bash", "input": {"command": "rm -rf not_there"}}]},
            {"text": "ok"}
        ]}),
        8,
    );
    let asks: Vec<Option<String>> = events
        .iter()
        .filter_map(|e| match e {
            Event::ApprovalRequested { risk, .. } => Some(risk.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(asks.len(), 3, "{asks:?}");
    assert!(asks[0].is_none());
    assert!(asks[1].as_deref().is_some_and(|r| r.contains("recursively")));
    assert!(asks[2].is_some());
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

#[test]
fn caps_tool_calls_per_turn() {
    let ws = Ws::new("callcap");
    let calls: Vec<_> = (0..40).map(|i| json!({"name": "glob", "input": {"pattern": format!("*.{i}")}})).collect();
    let (status, events, session) =
        run_with(&ws, json!({"turns": [{"tool_calls": calls}, {"text": "ok"}]}), |_| {}, None);
    assert_eq!(status, Status::Done);
    let r = tool_results(&events);
    assert_eq!(r.len(), 40, "every call still gets a result");
    assert!(r[..32].iter().all(|(ok, _)| *ok));
    assert!(r[32..].iter().all(|(ok, out)| !ok && out.contains("at most 32")));
    assert_eq!(session.messages[2].content.len(), 40);
}

#[test]
fn failing_pre_tool_hooks_fail_closed() {
    let ws = Ws::new("hookfail");
    let hooks = vec![oxgraft_core::hooks::Hook {
        event: oxgraft_core::hooks::HookEvent::PreTool,
        tools: vec!["write".into()],
        command: "exit 1".into(),
        timeout_secs: 5,
    }];
    let (_, events, _) = run_with(
        &ws,
        json!({"turns": [{"tool_calls": [{"name": "write", "input": {"path": "w.txt", "content": "x"}}]}, {"text": "ok"}]}),
        |c| c.hooks = hooks,
        None,
    );
    let r = tool_results(&events);
    assert!(!r[0].0 && r[0].1.contains("pre_tool hook failed"), "{:?}", r[0]);
    assert!(!ws.0.join("w.txt").exists());
}

#[test]
fn nested_instructions_are_shown_once_when_touched() {
    let ws = Ws::new("jit");
    std::fs::create_dir_all(ws.0.join("web/src")).unwrap();
    std::fs::write(ws.0.join("web/AGENTS.md"), "Use pnpm in web/.").unwrap();
    std::fs::write(ws.0.join("web/src/a.ts"), "export {}\n").unwrap();
    std::fs::write(ws.0.join("web/src/b.ts"), "export {}\n").unwrap();
    let (_, events, _) = run_with(
        &ws,
        json!({"turns": [
            {"tool_calls": [{"name": "read", "input": {"path": "web/src/a.ts"}}]},
            {"tool_calls": [{"name": "read", "input": {"path": "web/src/b.ts"}}]},
            {"text": "ok"}
        ]}),
        |_| {},
        None,
    );
    let r = tool_results(&events);
    assert!(r[0].1.contains("[Instructions from web/AGENTS.md") && r[0].1.contains("Use pnpm in web/."), "{}", r[0].1);
    assert!(!r[1].1.contains("Instructions from"), "shown only once");
}

#[cfg(unix)]
#[test]
fn verify_command_feeds_failures_back_after_edits() {
    let ws = Ws::new("verify");
    let check = "grep -q good status.txt || (echo status is not good; exit 1)";
    let (status, _, session) = run_with(
        &ws,
        json!({"turns": [
            {"tool_calls": [{"name": "write", "input": {"path": "status.txt", "content": "bad\n"}}]},
            {"tool_calls": [{"name": "read", "input": {"path": "status.txt"}}]},
            {"tool_calls": [{"name": "write", "input": {"path": "status.txt", "content": "good\n"}}]},
            {"text": "fixed"}
        ]}),
        |c| c.verify_command = Some(check.into()),
        None,
    );
    assert_eq!(status, Status::Done);
    let texts: Vec<String> = session.messages.iter().map(|m| m.text()).collect();
    assert!(texts[2].contains("Automatic check") && texts[2].contains("status is not good"), "{texts:?}");
    assert!(!texts[4].contains("Automatic check"), "read-only turns are not checked");
    assert!(!texts[6].contains("Automatic check"), "passing check adds nothing");
}

#[test]
fn protected_paths_always_ask_and_mistakes_trip_the_breaker() {
    let ws = Ws::new("protected");
    // allow_all would normally approve; protected paths still go to the approver (which says no here).
    let script: Script = serde_json::from_value(json!({"turns": [
        {"tool_calls": [{"name": "write", "input": {"path": ".github/workflows/ci.yml", "content": "x"}}]},
        {"text": "ok"}
    ]}))
    .unwrap();
    let provider = Mock::new(script);
    let toolkit = toolkit("allow_all");
    let approver = Fixed(Answer::No);
    let mut agent =
        Agent::new(&toolkit, &provider, oxgraft_tools::builtin(), &approver, AgentConfig::new("mock", ws.0.clone()));
    agent.sleep = |_| {};
    let mut session = Session::new("mock", "test", &ws.0);
    let mut events = Vec::new();
    agent.run(&mut session, "task", &mut |e| events.push(e.clone()));
    assert!(tool_results(&events)[0].1.contains("declined"));
    assert!(!ws.0.join(".github/workflows/ci.yml").exists());

    let bad: Vec<_> = (0..12)
        .map(|i| json!({"tool_calls": [{"name": "read", "input": {"path": format!("missing-{i}.txt")}}]}))
        .collect();
    let (status, _, _) = run_with(&ws, json!({ "turns": bad }), |_| {}, None);
    assert_eq!(status, Status::Stopped("12 consecutive tool errors".into()));
}

// SPEC-0003 AC9 (ADR 0007): replies stream as text_delta events, then the full message.
#[test]
fn replies_stream_as_text_deltas_then_full_messages() {
    let ws = Ws::new("stream");
    let (_, events, _) = run(&ws, "allow_all", Answer::Yes, json!({"turns": [{"text": "streamed reply"}]}), 2);
    let delta = events.iter().position(|e| matches!(e, Event::TextDelta { text } if text == "streamed reply"));
    let message = events.iter().position(|e| matches!(e, Event::Message { text, .. } if text == "streamed reply"));
    assert!(delta.is_some() && message.is_some() && delta < message, "{events:?}");

    let script: Script = serde_json::from_value(json!({"turns": [{"text": "plain"}]})).unwrap();
    let provider = Mock::new(script);
    let toolkit = toolkit("allow_all");
    let approver = Fixed(Answer::Yes);
    let mut config = AgentConfig::new("mock", ws.0.clone());
    config.stream = false;
    let agent = Agent::new(&toolkit, &provider, oxgraft_tools::builtin(), &approver, config);
    let mut session = Session::new("mock", "test", &ws.0);
    let mut events = Vec::new();
    agent.run(&mut session, "x", &mut |e| events.push(e.clone()));
    assert!(!events.iter().any(|e| matches!(e, Event::TextDelta { .. })));
    assert!(events.iter().any(|e| matches!(e, Event::Message { .. })));
}
