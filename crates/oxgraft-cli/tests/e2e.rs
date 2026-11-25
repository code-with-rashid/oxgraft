//! End-to-end tests of the `oxgraft` binary with the mock provider (SPEC-0004).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use oxgraft_protocol::Event;
use serde_json::json;

struct Ws(PathBuf);

impl Ws {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("oxgraft-e2e-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Ws(dir)
    }

    fn script(&self, turns: serde_json::Value) -> String {
        let p = self.0.join("script.json");
        std::fs::write(&p, json!({ "turns": turns }).to_string()).unwrap();
        format!("mock:{}", p.display())
    }

    fn oxgraft(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_oxgraft"))
            .args(args)
            .current_dir(&self.0)
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("ANTHROPIC_AUTH_TOKEN")
            .env_remove("OXGRAFT_MODEL")
            .env("XDG_CONFIG_HOME", self.0.join("no-user-config"))
            .env("APPDATA", self.0.join("no-user-config"))
            .env("OXGRAFT_DATA_DIR", self.data())
            .output()
            .unwrap()
    }

    /// Per-test data dir (checkpoint repos), outside the workspace.
    fn data(&self) -> PathBuf {
        self.0.with_extension("data")
    }

    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.0.join(rel)).unwrap()
    }
}

impl Drop for Ws {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        let _ = std::fs::remove_dir_all(self.data());
    }
}

fn events(out: &Output) -> Vec<Event> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| Event::from_jsonl(l).unwrap_or_else(|e| panic!("stdout line is not an event ({e}): {l}")))
        .collect()
}

fn list_cmd() -> &'static str {
    // `ls` works in POSIX sh (Git Bash on Windows) and as a PowerShell alias.
    "ls"
}

// SPEC-0004 AC1, AC2, AC6
#[test]
fn run_reads_edits_runs_and_reports() {
    let ws = Ws::new("run");
    std::fs::write(ws.0.join("app.txt"), "version = 1\n").unwrap();
    let edit = "<<<<<<< SEARCH\nversion = 1\n=======\nversion = 2\n>>>>>>> REPLACE";
    let model = ws.script(json!([
        {"text": "Let me look.", "tool_calls": [{"name": "read", "input": {"path": "app.txt"}}]},
        {"tool_calls": [{"name": "edit", "input": {"path": "app.txt", "edit": edit}}]},
        {"tool_calls": [{"name": "bash", "input": {"command": list_cmd()}}]},
        {"text": "Bumped the version to 2."}
    ]));
    let out = ws.oxgraft(&["run", "--json", "--yes", "--model", &model, "bump the version"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(ws.read("app.txt"), "version = 2\n");
    let ev = events(&out);
    assert!(matches!(ev.first(), Some(Event::SessionStarted { .. })));
    assert!(matches!(ev.last(), Some(Event::SessionFinished { ok: true, .. })));
    let bash_ok = ev.iter().any(|e| matches!(e, Event::ToolResult { output, .. } if output.contains("app.txt") && output.contains("exit code: 0")));
    assert!(bash_ok, "bash result missing: {ev:?}");

    // SPEC-0004 AC3, AC4: the session is saved and undo restores the file.
    let list = ws.oxgraft(&["sessions"]);
    assert!(String::from_utf8_lossy(&list.stdout).contains("bump the version"));
    let undo = ws.oxgraft(&["undo"]);
    assert_eq!(undo.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&undo.stdout).contains("restored"));
    assert_eq!(ws.read("app.txt"), "version = 1\n");
}

// SPEC-0004 AC3: resume continues the same history.
#[test]
fn resume_continues_a_session() {
    let ws = Ws::new("resume");
    let model = ws.script(json!([{"text": "first"}]));
    assert_eq!(ws.oxgraft(&["run", "--json", "--model", &model, "one"]).status.code(), Some(0));
    let model = ws.script(json!([{"text": "second"}]));
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "--resume", "last", "two"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let dir = ws.0.join(".oxgraft/sessions");
    let files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .collect();
    assert_eq!(files.len(), 1, "resume must not create a new session");
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(files[0].path()).unwrap()).unwrap();
    assert_eq!(saved["messages"].as_array().unwrap().len(), 4);
}

// SPEC-0004 AC2: usage errors exit 2, guard stops exit 3, failures exit 1.
#[test]
fn exit_codes() {
    let ws = Ws::new("codes");
    let out = ws.oxgraft(&["run", "hello"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no model configured"));

    assert_eq!(ws.oxgraft(&["run", "--model", "nope/x", "hello"]).status.code(), Some(2));

    let model = ws.script(json!([
        {"tool_calls": [{"name": "ls", "input": {}}]},
        {"tool_calls": [{"name": "ls", "input": {"path": "."}}]}
    ]));
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "--max-turns", "1", "loop"]);
    assert_eq!(out.status.code(), Some(3));

    let model = ws.script(json!([{"error": "auth"}]));
    assert_eq!(ws.oxgraft(&["run", "--json", "--model", &model, "x"]).status.code(), Some(1));
}

// Headless runs without --yes decline `ask` actions instead of hanging.
#[test]
fn headless_declines_writes_without_yes() {
    let ws = Ws::new("headless");
    let model = ws.script(json!([
        {"tool_calls": [{"name": "write", "input": {"path": "x.txt", "content": "x"}}]},
        {"text": "could not write"}
    ]));
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "write x"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(!Path::new(&ws.0.join("x.txt")).exists());
}

#[test]
fn workspace_config_and_profiles() {
    let ws = Ws::new("config");
    let model = ws.script(json!([{"text": "hi"}]));
    std::fs::create_dir_all(ws.0.join(".oxgraft")).unwrap();
    std::fs::write(ws.0.join(".oxgraft/config.toml"), format!("model = {:?}\nprofile = \"baseline\"\n", model))
        .unwrap();
    // GRAFT-0034: an untrusted workspace cannot choose the model.
    let out = ws.oxgraft(&["run", "--json", "hi"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("workspace not trusted"));
    let user_dir = ws.0.join("no-user-config").join("oxgraft");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(user_dir.join("config.toml"), format!("trusted_workspaces = [{:?}]\n", ws.0.display().to_string()))
        .unwrap();
    let out = ws.oxgraft(&["run", "--json", "hi"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let ev = events(&out);
    assert!(ev.iter().any(|e| matches!(e, Event::StrategySelected { strategy, .. } if strategy == "whole_file")));
    assert_eq!(ws.oxgraft(&["run", "--profile", "missing", "hi"]).status.code(), Some(2));
}

#[test]
fn introspection_commands() {
    let ws = Ws::new("intro");
    let out = ws.oxgraft(&["models"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("ollama"));
    let out = ws.oxgraft(&["strategies", "--json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v.as_array().unwrap().len() >= 5);
    let out = ws.oxgraft(&["--version"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("oxgraft"));
}

// GRAFT-0022: MCP servers from the user config become tools; workspace .mcp.json needs trust.
#[test]
fn mcp_tools_work_end_to_end() {
    let ws = Ws::new("mcp");
    let user_dir = ws.0.join("no-user-config").join("oxgraft");
    std::fs::create_dir_all(&user_dir).unwrap();
    let exe = env!("CARGO_BIN_EXE_oxgraft").replace('\\', "/");
    std::fs::write(
        user_dir.join("config.toml"),
        format!("[mcp_servers.fixture]\ncommand = \"{exe}\"\nargs = [\"__mcp-fixture\"]\n"),
    )
    .unwrap();
    let model = ws.script(json!([
        {"tool_calls": [
            {"name": "mcp__fixture__echo", "input": {"text": "hello"}},
            {"name": "mcp__fixture__fail", "input": {}}
        ]},
        {"text": "done"}
    ]));
    let out = ws.oxgraft(&["run", "--json", "--yes", "--model", &model, "use mcp"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let results: Vec<(bool, String)> = events(&out)
        .into_iter()
        .filter_map(|e| match e {
            Event::ToolResult { ok, output, .. } => Some((ok, output)),
            _ => None,
        })
        .collect();
    // MCP output is fenced as untrusted data (GRAFT-0055).
    assert!(
        results[0].0
            && results[0].1.contains("<untrusted source=\"MCP tool mcp__fixture__echo\">\necho: hello\n</untrusted>")
    );
    assert!(!results[1].0 && results[1].1.contains("it failed"));

    // Untrusted project-level .mcp.json is ignored with a warning.
    std::fs::write(user_dir.join("config.toml"), "").unwrap();
    std::fs::write(
        ws.0.join(".mcp.json"),
        format!("{{\"mcpServers\":{{\"p\":{{\"command\":\"{exe}\",\"args\":[\"__mcp-fixture\"]}}}}}}"),
    )
    .unwrap();
    let model = ws.script(json!([{"text": "hi"}]));
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "x"]);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("ignoring 1 MCP server"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn doctor_reports_problems_and_readiness() {
    let ws = Ws::new("doctor");
    let out = ws.oxgraft(&["doctor"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "{text}");
    assert!(text.contains("FAIL model"), "{text}");
    let model = ws.script(json!([]));
    let out = ws.oxgraft(&["doctor", "--model", &model]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("ready"));
    for expect in ["trust", "not trusted", "approvals", "mode `suggest`", "checkpoints", "streaming"] {
        assert!(text.contains(expect), "missing {expect}:\n{text}");
    }
    assert!(!ws.data().exists(), "doctor creates nothing");
    let out = ws.oxgraft(&["doctor", "--model", &model, "--mode", "nope"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("FAIL approvals"));
}

#[test]
fn init_writes_agents_md_through_the_loop() {
    let ws = Ws::new("init");
    let model = ws.script(json!([
        {"tool_calls": [{"name": "write", "input": {"path": "AGENTS.md", "content": "# AGENTS.md\n\nRun `make test`.\n"}}]},
        {"text": "Wrote AGENTS.md"}
    ]));
    let out = ws.oxgraft(&["init", "--json", "--yes", "--model", &model]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(ws.read("AGENTS.md").contains("make test"));
}

// GRAFT-0036: user rules with justification; deny wins over a broad allow; compound commands split.
#[test]
fn configured_rules_apply_with_reasons() {
    let ws = Ws::new("rules");
    let user_dir = ws.0.join("no-user-config").join("oxgraft");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(
        user_dir.join("config.toml"),
        "[[rules]]\ntool = \"bash\"\nresource = \"echo *\"\ndecision = \"allow\"\n\n\
         [[rules]]\ntool = \"bash\"\nresource = \"rm *\"\ndecision = \"deny\"\njustification = \"never delete files\"\n",
    )
    .unwrap();
    let model = ws.script(json!([
        {"tool_calls": [
            {"name": "bash", "input": {"command": "echo hello"}},
            {"name": "bash", "input": {"command": "echo hi && rm -rf nothing"}}
        ]},
        {"text": "done"}
    ]));
    // No --yes: `ask` would be declined, so the first call passing proves the allow rule applied.
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "x"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let results: Vec<(bool, String)> = events(&out)
        .into_iter()
        .filter_map(|e| match e {
            Event::ToolResult { ok, output, .. } => Some((ok, output)),
            _ => None,
        })
        .collect();
    assert!(results[0].0 && results[0].1.contains("hello"), "{:?}", results[0]);
    assert!(!results[1].0 && results[1].1.contains("never delete files"), "{:?}", results[1]);
}

// SPEC-0004 AC7 (GRAFT-0046): per-turn checkpoints capture shell changes too; restore
// puts the workspace back and can itself be undone.
#[test]
fn checkpoints_capture_shell_changes_and_restore() {
    let has_git = Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success());
    if !has_git {
        eprintln!("git not installed; skipping");
        return;
    }
    let ws = Ws::new("checkpoints");
    std::fs::write(ws.0.join("keep.txt"), "original\n").unwrap();
    let model = ws.script(json!([
        {"tool_calls": [{"name": "write", "input": {"path": "new.txt", "content": "from write\n"}}]},
        {"tool_calls": [{"name": "bash", "input": {"command": "echo shell > made_by_shell.txt"}}]},
        {"text": "done"}
    ]));
    let out = ws.oxgraft(&["run", "--json", "--yes", "--model", &model, "make files"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(ws.0.join("made_by_shell.txt").exists());
    assert!(!ws.0.join(".git").exists(), "the user's repository is never created");
    assert!(!ws.0.join(".oxgraft/checkpoints.git").exists(), "the checkpoint repo lives outside the workspace");
    assert!(ws.data().join("checkpoints").is_dir());

    let list = ws.oxgraft(&["checkpoints"]);
    let listing = String::from_utf8_lossy(&list.stdout).to_string();
    assert!(listing.contains("start") && listing.contains("turn 1") && listing.contains("turn 2"), "{listing}");

    let restore = ws.oxgraft(&["restore", "0"]);
    assert_eq!(restore.status.code(), Some(0), "{}", String::from_utf8_lossy(&restore.stderr));
    assert!(!ws.0.join("new.txt").exists());
    assert!(!ws.0.join("made_by_shell.txt").exists(), "changes made by shell commands are rolled back too");
    assert_eq!(ws.read("keep.txt"), "original\n");

    // Undo the restore with the pre-restore checkpoint it recorded (number 3).
    let undo = ws.oxgraft(&["restore", "3"]);
    assert_eq!(undo.status.code(), Some(0), "{}", String::from_utf8_lossy(&undo.stderr));
    assert!(ws.0.join("made_by_shell.txt").exists());
    assert_eq!(ws.oxgraft(&["restore", "99"]).status.code(), Some(2));
}

// SPEC-0004 AC8 (GRAFT-0048): a human-mode run ends with what changed and how to undo it.
#[test]
fn human_runs_end_with_a_change_summary() {
    if !Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("git not installed; skipping");
        return;
    }
    let ws = Ws::new("summary");
    let model = ws.script(json!([
        {"tool_calls": [{"name": "write", "input": {"path": "notes.txt", "content": "n\n"}}]},
        {"text": "done"}
    ]));
    let out = ws.oxgraft(&["run", "--yes", "--model", &model, "write notes"]);
    assert_eq!(out.status.code(), Some(0));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("changed in this run") && err.contains("notes.txt"), "{err}");
    assert!(err.contains("oxgraft restore 0"), "{err}");

    // A run that changes nothing prints no summary.
    let model = ws.script(json!([{"text": "nothing to do"}]));
    let out = ws.oxgraft(&["run", "--yes", "--model", &model, "look"]);
    assert!(!String::from_utf8_lossy(&out.stderr).contains("changed in this run"));
}

// SPEC-0004 AC9 (GRAFT-0049): fork a session; the original is unchanged.
#[test]
fn sessions_can_be_forked() {
    let ws = Ws::new("fork");
    let model = ws.script(json!([{"text": "first answer"}]));
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "question one"]);
    assert_eq!(out.status.code(), Some(0));
    let original = events(&out)
        .into_iter()
        .find_map(|e| match e {
            Event::SessionStarted { session_id, .. } => session_id,
            _ => None,
        })
        .unwrap();

    let model = ws.script(json!([{"text": "second answer"}]));
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "--fork", &format!("{original}@2"), "question two"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("forked session"));
    let sessions = ws.oxgraft(&["sessions"]);
    let listing = String::from_utf8_lossy(&sessions.stdout).to_string();
    assert!(listing.contains(&format!("fork of {original}")), "{listing}");
    let saved: serde_json::Value =
        serde_json::from_str(&ws.read(&format!(".oxgraft/sessions/{original}.json"))).unwrap();
    assert_eq!(saved["messages"].as_array().unwrap().len(), 2, "original untouched");

    let bad = ws.oxgraft(&["run", "--model", &model, "--fork", &format!("{original}@1"), "x"]);
    assert_eq!(bad.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("fork points"));
}

// SPEC-0004 AC10 (GRAFT-0052): approval modes; rails still ask; repos can't pick a mode.
#[test]
fn approval_modes_change_what_runs_without_asking() {
    let ws = Ws::new("modes");
    let results = |out: &Output| -> Vec<(bool, String)> {
        events(out)
            .into_iter()
            .filter_map(|e| match e {
                Event::ToolResult { ok, output, .. } => Some((ok, output)),
                _ => None,
            })
            .collect()
    };
    let turns = json!([
        {"tool_calls": [{"name": "write", "input": {"path": "a.txt", "content": "a\n"}}]},
        {"tool_calls": [{"name": "bash", "input": {"command": "echo ran"}}]},
        {"tool_calls": [{"name": "bash", "input": {"command": "rm -rf nothing_here"}}]},
        {"text": "done"}
    ]);
    // No --yes: every `ask` is declined, so an ok result means the mode allowed it.
    let model = ws.script(turns.clone());
    let out = ws.oxgraft(&["run", "--json", "--mode", "full-auto", "--model", &model, "x"]);
    let r = results(&out);
    assert!(r[0].0 && r[1].0 && r[1].1.contains("ran"), "{r:?}");
    assert!(!r[2].0 && r[2].1.contains("declined"), "risky commands still ask: {r:?}");

    let _ = std::fs::remove_file(ws.0.join("a.txt"));
    let model = ws.script(turns.clone());
    let out = ws.oxgraft(&["run", "--json", "--mode", "auto-edit", "--model", &model, "x"]);
    let r = results(&out);
    assert!(r[0].0 && !r[1].0, "auto-edit allows edits, asks for commands: {r:?}");

    let _ = std::fs::remove_file(ws.0.join("a.txt"));
    let model = ws.script(turns.clone());
    let out = ws.oxgraft(&["run", "--json", "--yes", "--mode", "read-only", "--model", &model, "x"]);
    let r = results(&out);
    assert!(r.iter().all(|(ok, _)| !ok) && r[0].1.contains("read-only mode"), "{r:?}");

    // A workspace can't switch itself to full-auto.
    std::fs::create_dir_all(ws.0.join(".oxgraft")).unwrap();
    std::fs::write(ws.0.join(".oxgraft/config.toml"), "approval_mode = \"full-auto\"\n").unwrap();
    let _ = std::fs::remove_file(ws.0.join("a.txt"));
    let model = ws.script(turns);
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "x"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("ignoring `approval_mode"));
    let r = results(&out);
    assert!(!r[0].0, "untrusted workspace mode ignored: {r:?}");

    assert_eq!(ws.oxgraft(&["run", "--mode", "yolo", "--model", "mock:x.json", "x"]).status.code(), Some(2));
}

// SPEC-0003 AC11 via the CLI: `run --until` keeps going until the check passes.
#[test]
fn run_until_a_check_passes() {
    let ws = Ws::new("until");
    let model = ws.script(json!([
        {"text": "done already?"},
        {"tool_calls": [{"name": "write", "input": {"path": "ok.txt", "content": "ok\n"}}]},
        {"text": "now done"}
    ]));
    let out = ws.oxgraft(&["run", "--json", "--yes", "--until", "cat ok.txt", "--model", &model, "make ok.txt"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(events(&out).iter().any(|e| matches!(e, Event::Warning { message } if message.contains("finish check"))));
    assert!(ws.0.join("ok.txt").exists());
}

// Chat: a task, then /checkpoints and /restore 0 roll the workspace back.
#[test]
fn chat_lists_and_restores_checkpoints() {
    if !Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("git not installed; skipping");
        return;
    }
    use std::io::Write as _;
    let ws = Ws::new("chatcp");
    let model = ws.script(json!([
        {"tool_calls": [{"name": "write", "input": {"path": "made.txt", "content": "x\n"}}]},
        {"text": "made it"}
    ]));
    let mut child = Command::new(env!("CARGO_BIN_EXE_oxgraft"))
        .args(["chat", "--yes", "--model", &model])
        .current_dir(&ws.0)
        .env_remove("ANTHROPIC_API_KEY")
        .env("XDG_CONFIG_HOME", ws.0.join("no-user-config"))
        .env("APPDATA", ws.0.join("no-user-config"))
        .env("OXGRAFT_DATA_DIR", ws.data())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"make a file\n/checkpoints\n/restore 0\n/restore x\n/exit\n").unwrap();
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(err.contains("checkpoints of session") && err.contains("turn 1"), "{err}");
    assert!(err.contains("workspace restored to checkpoint 0"), "{err}");
    assert!(err.contains("usage: /restore N"), "{err}");
    assert!(!ws.0.join("made.txt").exists(), "restored to before the task");
}

// `doctor --live` sends a plain request and a tool-call request to the model.
#[test]
fn doctor_live_checks_reply_and_tool_calling() {
    let ws = Ws::new("doctorlive");
    let model = ws.script(json!([
        {"text": "ready"},
        {"tool_calls": [{"name": "echo", "input": {"text": "ping"}}]}
    ]));
    let out = ws.oxgraft(&["doctor", "--live", "--model", &model]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("ok   live reply") && text.contains("\"ready\""), "{text}");
    assert!(text.contains("ok   live tools") && text.contains("tool call works"), "{text}");

    // A model that answers in prose instead of calling the tool is reported.
    let model = ws.script(json!([{"text": "ready"}, {"text": "ping"}]));
    let out = ws.oxgraft(&["doctor", "--live", "--model", &model]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "{text}");
    assert!(text.contains("FAIL live tools") && text.contains("may not support tool calling"), "{text}");
}

// SPEC-0004 AC11 (GRAFT-0057): custom commands from .agents/commands.
#[test]
fn custom_commands_expand_in_run() {
    let ws = Ws::new("commands");
    std::fs::create_dir_all(ws.0.join(".agents/commands")).unwrap();
    std::fs::write(ws.0.join(".agents/commands/explain.md"), "Explain $ARGUMENTS in two sentences.\n").unwrap();
    let model = ws.script(json!([{"text": "It greets."}]));
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "/explain", "main.rs"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let id = events(&out)
        .into_iter()
        .find_map(|e| match e {
            Event::SessionStarted { session_id, .. } => session_id,
            _ => None,
        })
        .unwrap();
    let saved = ws.read(&format!(".oxgraft/sessions/{id}.json"));
    assert!(saved.contains("Explain main.rs in two sentences."), "{saved}");

    let out = ws.oxgraft(&["run", "--model", &model, "/nope"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("available: /explain"));
}

// Chat `!command` (GRAFT-0058): runs without the model; output goes with the next message.
#[test]
fn chat_bang_runs_a_command_and_passes_its_output_on() {
    use std::io::Write as _;
    let ws = Ws::new("bang");
    let model = ws.script(json!([{"text": "noted"}]));
    let mut child = Command::new(env!("CARGO_BIN_EXE_oxgraft"))
        .args(["chat", "--model", &model])
        .current_dir(&ws.0)
        .env_remove("ANTHROPIC_API_KEY")
        .env("XDG_CONFIG_HOME", ws.0.join("no-user-config"))
        .env("APPDATA", ws.0.join("no-user-config"))
        .env("OXGRAFT_DATA_DIR", ws.data())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"!echo bang-output\nwhat did it print?\n/exit\n").unwrap();
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("bang-output"), "{err}");
    let sessions = std::fs::read_dir(ws.0.join(".oxgraft/sessions")).unwrap();
    let file = sessions.flatten().find(|e| e.path().extension().is_some_and(|x| x == "json")).unwrap();
    let saved = std::fs::read_to_string(file.path()).unwrap();
    assert!(saved.contains("I ran `echo bang-output` myself") && saved.contains("what did it print?"), "{saved}");
}

// SPEC-0004 AC12 (GRAFT-0063): headless runs report the files they changed.
#[test]
fn json_runs_report_changed_files() {
    if !Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success()) {
        return;
    }
    let ws = Ws::new("fileschanged");
    let model = ws.script(json!([
        {"tool_calls": [{"name": "write", "input": {"path": "src/new.rs", "content": "fn x() {}\n"}}]},
        {"tool_calls": [{"name": "bash", "input": {"command": "echo hi > from_shell.txt"}}]},
        {"text": "done"}
    ]));
    let out = ws.oxgraft(&["run", "--json", "--yes", "--model", &model, "x"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let events = events(&out);
    let files = events
        .iter()
        .find_map(|e| match e {
            Event::FilesChanged { files } => Some(files.clone()),
            _ => None,
        })
        .expect("a files_changed event");
    assert!(files.contains(&"src/new.rs".to_string()) && files.contains(&"from_shell.txt".to_string()), "{files:?}");
    let pos = |f: fn(&Event) -> bool| events.iter().position(f).unwrap();
    assert!(pos(|e| matches!(e, Event::FilesChanged { .. })) < pos(|e| matches!(e, Event::SessionFinished { .. })));

    // Nothing changed: no event.
    let model = ws.script(json!([{"text": "nothing to do"}]));
    let out = ws.oxgraft(&["run", "--json", "--model", &model, "look"]);
    assert!(!self::events(&out).iter().any(|e| matches!(e, Event::FilesChanged { .. })));
}

// GRAFT-0064: doctor warns about Ollama's small default context.
#[test]
fn doctor_warns_about_ollama_context() {
    let ws = Ws::new("ollamawarn");
    let out = ws.oxgraft(&["doctor", "--model", "ollama/qwen3"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("OLLAMA_CONTEXT_LENGTH"), "{text}");
}

// SPEC-0004 AC13 (GRAFT-0066): export a session as Markdown.
#[test]
fn sessions_export_as_markdown() {
    let ws = Ws::new("export");
    std::fs::write(ws.0.join("a.txt"), "alpha\n").unwrap();
    let model = ws.script(json!([
        {"tool_calls": [{"name": "read", "input": {"path": "a.txt"}}]},
        {"text": "The file says alpha."}
    ]));
    assert_eq!(ws.oxgraft(&["run", "--json", "--model", &model, "what is in a.txt?"]).status.code(), Some(0));
    let out = ws.oxgraft(&["export"]);
    let md = String::from_utf8_lossy(&out.stdout);
    assert!(md.starts_with("# what is in a.txt?"), "{md}");
    for expect in ["## User", "**→ read**", "alpha", "## Oxgraft", "The file says alpha."] {
        assert!(md.contains(expect), "missing {expect}:\n{md}");
    }
    let file = ws.0.join("t.md");
    assert_eq!(ws.oxgraft(&["export", "-o", file.to_str().unwrap()]).status.code(), Some(0));
    assert!(std::fs::read_to_string(file).unwrap().contains("The file says alpha."));
}

// SPEC-0004 AC15 (GRAFT-0070): --plan is read-only.
#[test]
fn plan_mode_changes_nothing() {
    let ws = Ws::new("plan");
    let model = ws.script(json!([
        {"tool_calls": [{"name": "write", "input": {"path": "x.txt", "content": "x"}}]},
        {"text": "1. Create x.txt."}
    ]));
    let out = ws.oxgraft(&["run", "--json", "--yes", "--plan", "--model", &model, "add x.txt"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!ws.0.join("x.txt").exists());
    assert!(
        events(&out)
            .iter()
            .any(|e| matches!(e, Event::ToolResult { ok: false, output, .. } if output.contains("read-only")))
    );
    assert_eq!(ws.oxgraft(&["run", "--plan", "--mode", "full-auto", "--model", &model, "x"]).status.code(), Some(2));
}

// Review finding: a task that is just an absolute path is a task, not a custom command.
#[test]
fn absolute_path_tasks_are_not_commands() {
    let ws = Ws::new("abspath");
    let model = ws.script(json!([{"text": "looked"}]));
    let out = ws.oxgraft(&["run", "--model", &model, "/tmp/notes.txt"]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
}

// `sessions --json`: one object per session, for scripts.
#[test]
fn sessions_list_as_json() {
    let ws = Ws::new("sessionsjson");
    let model = ws.script(json!([{"text": "hi"}]));
    assert_eq!(ws.oxgraft(&["run", "--json", "--model", &model, "say hi"]).status.code(), Some(0));
    let out = ws.oxgraft(&["sessions", "--json"]);
    let line = String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or_default().to_string();
    let v: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v["title"], "say hi");
    assert_eq!(v["messages"], 2);
}

// GRAFT-0075: --quiet prints only the final answer; --output-last-message saves it.
#[test]
fn quiet_runs_print_only_the_answer() {
    let ws = Ws::new("quiet");
    let model = ws.script(json!([
        {"tool_calls": [{"name": "ls", "input": {}}]},
        {"text": "The answer is 42."}
    ]));
    let file = ws.0.join("answer.txt");
    let out = ws.oxgraft(&["run", "-q", "--output-last-message", file.to_str().unwrap(), "--model", &model, "x"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "The answer is 42.\n");
    assert!(!String::from_utf8_lossy(&out.stderr).contains("→ ls"));
    assert_eq!(std::fs::read_to_string(file).unwrap(), "The answer is 42.");
    assert_eq!(ws.oxgraft(&["run", "-q", "--json", "--model", &model, "x"]).status.code(), Some(2));
}

// GRAFT-0077: /mode switches approval mode inside chat.
#[test]
fn chat_mode_switches_approvals() {
    use std::io::Write as _;
    let ws = Ws::new("chatmode");
    let model = ws.script(json!([
        {"tool_calls": [{"name": "write", "input": {"path": "made.txt", "content": "x"}}]},
        {"text": "written"}
    ]));
    let mut child = Command::new(env!("CARGO_BIN_EXE_oxgraft"))
        .args(["chat", "--model", &model])
        .current_dir(&ws.0)
        .env_remove("ANTHROPIC_API_KEY")
        .env("XDG_CONFIG_HOME", ws.0.join("no-user-config"))
        .env("APPDATA", ws.0.join("no-user-config"))
        .env("OXGRAFT_DATA_DIR", ws.data())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"/mode\n/mode yolo\n/mode auto-edit\nmake the file\n/exit\n").unwrap();
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("mode: suggest") && err.contains("unknown mode `yolo`") && err.contains("mode: auto-edit"),
        "{err}"
    );
    assert!(ws.0.join("made.txt").exists(), "auto-edit allowed the write without a prompt: {err}");
}

// GRAFT-0083: /model switches the model mid-chat.
#[test]
fn chat_model_switches_provider() {
    use std::io::Write as _;
    let ws = Ws::new("chatmodel");
    let first = ws.script(json!([{"text": "from the first model"}]));
    let second_path = ws.0.join("second.json");
    std::fs::write(&second_path, json!({"turns": [{"text": "from the second model"}]}).to_string()).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_oxgraft"))
        .args(["chat", "--model", &first])
        .current_dir(&ws.0)
        .env_remove("ANTHROPIC_API_KEY")
        .env("XDG_CONFIG_HOME", ws.0.join("no-user-config"))
        .env("APPDATA", ws.0.join("no-user-config"))
        .env("OXGRAFT_DATA_DIR", ws.data())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let script = format!("hello\n/model nope/x\n/model mock:{}\nhello again\n/exit\n", second_path.display());
    child.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    let (stdout, stderr) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("from the first model") && stdout.contains("from the second model"), "{stdout}\n{stderr}");
    assert!(stderr.contains("cannot switch to `nope/x`"), "{stderr}");
}

// GRAFT-0087: token use per model across sessions.
#[test]
fn stats_sum_usage_per_model() {
    let ws = Ws::new("stats");
    let model = ws.script(json!([{"text": "a"}]));
    for _ in 0..2 {
        assert_eq!(ws.oxgraft(&["run", "--json", "--model", &model, "x"]).status.code(), Some(0));
    }
    let out = ws.oxgraft(&["stats", "--json"]);
    let line = String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or_default().to_string();
    let v: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v["sessions"], 2);
    assert!(v["input_tokens"].as_u64().unwrap() > 0);
    assert!(String::from_utf8_lossy(&ws.oxgraft(&["stats"]).stdout).contains("SESSIONS"));
}
