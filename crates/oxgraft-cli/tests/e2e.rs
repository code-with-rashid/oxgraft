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
            .output()
            .unwrap()
    }

    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.0.join(rel)).unwrap()
    }
}

impl Drop for Ws {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
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
    assert_eq!(results[0], (true, "echo: hello".to_string()));
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
