use std::io::Read as _;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use oxgraft_seams::{Action, Confinement, EditFormat, Tool, ToolContext, ToolOutput, ToolSpec};
use serde_json::{Value, json};
use wait_timeout::ChildExt;

use crate::util::{opt_u64, str_field, truncate};

pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const MAX_TIMEOUT_SECS: u64 = 600;

#[derive(Debug, Clone, Copy, Default)]
pub struct Bash;

/// Which shell runs commands. On Windows, models write POSIX shell far more often
/// than cmd syntax, so prefer Git Bash's `sh` when present, then PowerShell
/// (GRAFT-0030, as Codex uses PowerShell and Claude Code uses Git Bash on Windows).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellKind {
    Posix(std::path::PathBuf),
    PowerShell,
}

pub fn shell_kind() -> ShellKind {
    if !cfg!(windows) {
        return ShellKind::Posix("sh".into());
    }
    if let Some(sh) = std::env::var_os("OXGRAFT_SHELL").map(std::path::PathBuf::from) {
        return ShellKind::Posix(sh);
    }
    let in_path = std::env::var_os("PATH")
        .and_then(|p| std::env::split_paths(&p).map(|d| d.join("sh.exe")).find(|p| p.is_file()));
    let git_bash = ["C:\\Program Files\\Git\\bin\\sh.exe", "C:\\Program Files (x86)\\Git\\bin\\sh.exe"]
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.is_file());
    match in_path.or(git_bash) {
        Some(sh) => ShellKind::Posix(sh),
        None => ShellKind::PowerShell,
    }
}

/// The program and leading arguments that run one command line in this shell.
/// Hooks and `verify_command` use the same shell.
pub fn shell_program() -> (std::path::PathBuf, Vec<String>) {
    match shell_kind() {
        ShellKind::Posix(sh) => (sh, vec!["-c".into()]),
        ShellKind::PowerShell => {
            ("powershell".into(), vec!["-NoProfile".into(), "-NonInteractive".into(), "-Command".into()])
        }
    }
}

fn shell(command: &str) -> Command {
    let (program, args) = shell_program();
    let mut c = Command::new(program);
    c.args(args).arg(command);
    c
}

impl Tool for Bash {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        let shell = match shell_kind() {
            ShellKind::Posix(_) => "POSIX sh",
            ShellKind::PowerShell => "Windows PowerShell; use PowerShell syntax",
        };
        ToolSpec {
            name: "bash".into(),
            description: format!(
                "Run a shell command ({shell}) in the workspace directory. Returns the exit code, stdout and \
                 stderr. Non-interactive: no prompts, no pagers. Default timeout {DEFAULT_TIMEOUT_SECS}s, max \
                 {MAX_TIMEOUT_SECS}s. Prefer the read/grep/glob tools for looking at files."
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "The command line"},
                    "timeout_secs": {"type": "integer", "description": "Timeout in seconds", "minimum": 1}
                },
                "required": ["command"]
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        Action { tool: "bash".into(), resource: input.get("command").and_then(Value::as_str).unwrap_or("").into() }
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let command = match str_field(input, "command") {
            Ok(c) if c.trim().is_empty() => return ToolOutput::error("`command` is empty"),
            Ok(c) => c,
            Err(e) => return ToolOutput::error(e),
        };
        let timeout = match opt_u64(input, "timeout_secs") {
            Ok(t) => t.unwrap_or(DEFAULT_TIMEOUT_SECS).clamp(1, MAX_TIMEOUT_SECS),
            Err(e) => return ToolOutput::error(e),
        };
        let parts = oxgraft_seams::Command { program: "shell".into(), args: vec![command.into()] };
        let mut sandboxed = None;
        let mut cmd = match ctx.sandbox.confinement(&parts) {
            Confinement::None => shell(command),
            Confinement::Restricted { writable, network } => {
                sandboxed = Some(network);
                match crate::sandbox::confined(command, ctx.workspace, &writable, network) {
                    Ok(c) => c,
                    // Fail closed: never fall back to running unconfined.
                    Err(e) => return ToolOutput::error(format!("sandbox unavailable, command not run: {e}")),
                }
            }
        };
        cmd.current_dir(ctx.workspace)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Keep tools non-interactive and output plain (lesson shared by Codex, Claude Code, SWE-agent).
            .env("PAGER", "cat")
            .env("GIT_PAGER", "cat")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("NO_COLOR", "1")
            .env("TERM", "dumb")
            .env("OXGRAFT", "1");
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return ToolOutput::error(format!("could not start shell: {e}")),
        };

        // Read pipes on threads so a chatty process can't deadlock us.
        let (tx, rx) = mpsc::channel::<(u8, Vec<u8>)>();
        for (tag, pipe) in [
            (1u8, child.stdout.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>)),
            (2u8, child.stderr.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>)),
        ] {
            if let Some(mut pipe) = pipe {
                let tx = tx.clone();
                std::thread::spawn(move || {
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    let _ = tx.send((tag, buf));
                });
            }
        }
        drop(tx);

        // Wait in short slices so a user interrupt (Ctrl-C) stops the command promptly.
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout);
        let mut interrupted = false;
        let status = loop {
            match child.wait_timeout(Duration::from_millis(200)) {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {
                    let cancelled = ctx.cancel.load(std::sync::atomic::Ordering::Relaxed);
                    if cancelled || std::time::Instant::now() >= deadline {
                        interrupted = cancelled;
                        let _ = child.kill();
                        let _ = child.wait();
                        break None;
                    }
                }
                Err(e) => return ToolOutput::error(format!("waiting for command failed: {e}")),
            }
        };

        // Grandchildren may keep pipes open; don't wait on them forever.
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        for _ in 0..2 {
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok((1, b)) => stdout = b,
                Ok((_, b)) => stderr = b,
                Err(_) => break,
            }
        }
        let stdout = String::from_utf8_lossy(&stdout);
        let stderr = String::from_utf8_lossy(&stderr);
        let mut out = String::new();
        match status {
            Some(s) => out.push_str(&format!("exit code: {}\n", s.code().map_or("signal".into(), |c| c.to_string()))),
            None if interrupted => out.push_str("interrupted by the user; process killed\n"),
            None => out.push_str(&format!("timed out after {timeout}s; process killed\n")),
        }
        if !stdout.trim().is_empty() {
            out.push_str(&format!("--- stdout ---\n{}\n", stdout.trim_end()));
        }
        if !stderr.trim().is_empty() {
            out.push_str(&format!("--- stderr ---\n{}\n", stderr.trim_end()));
        }
        let failed = status.is_some_and(|s| !s.success());
        if let Some(network) = sandboxed
            && failed
            && let Some(hint) = sandbox_hint(&stderr, network)
        {
            out.push_str(hint);
        }
        let out = if out.chars().count() > ctx.max_output_chars {
            // Keep the whole output where the model can read or grep it (GRAFT-0060).
            let saved = save_full_output(ctx.workspace, &out);
            let mut short = truncate(&out, ctx.max_output_chars);
            if let Some(rel) = saved {
                short.push_str(&format!("\n[full output saved to {rel}; use `read` or `grep` on it]\n"));
            }
            short
        } else {
            out
        };
        if status.is_none() { ToolOutput::error(out) } else { ToolOutput::ok(out) }
    }
}

/// Write `text` to `.oxgraft/outputs/<n>.log` in the workspace, keeping the 20
/// newest files. Returns the workspace-relative path.
fn save_full_output(workspace: &std::path::Path, text: &str) -> Option<String> {
    const KEEP: usize = 20;
    let dir = workspace.join(".oxgraft").join("outputs");
    std::fs::create_dir_all(&dir).ok()?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
    let name = format!("bash-{stamp}.log");
    std::fs::write(dir.join(&name), text).ok()?;
    let mut logs: Vec<_> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "log"))
        .collect();
    logs.sort();
    for old in logs.iter().take(logs.len().saturating_sub(KEEP)) {
        let _ = std::fs::remove_file(old);
    }
    Some(format!(".oxgraft/outputs/{name}"))
}

/// When a sandboxed command fails in a way the sandbox causes, say so, so the
/// model stops retrying and tells the user instead (after Gemini CLI's sandbox
/// denial handling; here only a hint, never an automatic expansion).
pub fn sandbox_hint(stderr: &str, network: bool) -> Option<&'static str> {
    let s = stderr.to_ascii_lowercase();
    let write_denied =
        ["read-only file system", "permission denied", "operation not permitted"].iter().any(|p| s.contains(p));
    let net_denied = !network
        && [
            "could not resolve host",
            "temporary failure in name resolution",
            "network is unreachable",
            "name or service not known",
        ]
        .iter()
        .any(|p| s.contains(p));
    if net_denied {
        Some(
            "[note: this command ran in the sandbox, which has no network access; the failure above is likely the \
             sandbox. Don't retry it; tell the user what needs the network.]\n",
        )
    } else if write_denied {
        Some(
            "[note: this command ran in the sandbox, which only allows writes inside the workspace and /tmp; the \
             failure above is likely the sandbox. Don't retry it; tell the user what needs to write elsewhere.]\n",
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    // SPEC-0002 AC13 (GRAFT-0060): long output is cut, and saved in full.
    #[test]
    fn long_output_is_saved_in_full() {
        if !matches!(shell_kind(), ShellKind::Posix(_)) {
            return; // the loop below is POSIX sh
        }
        let f = Fixture::new("longout");
        let cmd = "i=0; while [ $i -lt 1500 ]; do echo line$i; i=$((i+1)); done";
        let out = f.run(&Bash, json!({"command": cmd}));
        assert!(
            out.content.contains("truncated") && out.content.contains("full output saved to .oxgraft/outputs/"),
            "{}",
            out.content
        );
        let rel = out.content.split("saved to ").nth(1).unwrap().split(';').next().unwrap();
        let full = f.read(rel);
        assert!(full.contains("line0") && full.contains("line777") && full.contains("line1499"));
        let short = f.run(&Bash, json!({"command": "echo short"}));
        assert!(!short.content.contains("saved to"));
    }

    // SPEC-0006 AC7
    #[test]
    fn sandbox_failures_get_a_hint() {
        assert!(sandbox_hint("touch: /etc/x: Read-only file system", false).unwrap().contains("writes inside"));
        assert!(sandbox_hint("curl: (6) Could not resolve host: x.dev", false).unwrap().contains("no network"));
        assert!(sandbox_hint("curl: (6) Could not resolve host: x.dev", true).is_none(), "network allowed");
        assert!(sandbox_hint("error[E0425]: cannot find value", false).is_none());
    }

    // SPEC-0002 AC4
    #[test]
    fn runs_in_workspace_and_reports_exit_codes() {
        let f = Fixture::new("bash");
        f.file("hello.txt", "hi");
        let cmd = match shell_kind() {
            ShellKind::Posix(_) => "cat hello.txt",
            ShellKind::PowerShell => "Get-Content hello.txt",
        };
        let out = f.run(&Bash, json!({"command": cmd}));
        assert!(out.content.contains("exit code: 0"), "{}", out.content);
        assert!(out.content.contains("hi"));
        let out = f.run(&Bash, json!({"command": "exit 3"}));
        assert!(out.content.contains("exit code: 3"));
        assert!(f.run(&Bash, json!({"command": "  "})).is_error);
    }

    #[cfg(unix)]
    #[test]
    fn times_out_without_hanging() {
        let f = Fixture::new("timeout");
        let start = std::time::Instant::now();
        let out = f.run(&Bash, json!({"command": "sleep 30", "timeout_secs": 1}));
        assert!(out.is_error && out.content.contains("timed out"), "{}", out.content);
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn output_is_truncated() {
        let f = Fixture::new("big");
        let cmd = match shell_kind() {
            ShellKind::Posix(_) => "i=0; while [ $i -lt 3000 ]; do echo line$i; i=$((i+1)); done",
            ShellKind::PowerShell => "1..3000 | ForEach-Object { \"line$_\" }",
        };
        let out = f.run(&Bash, json!({"command": cmd}));
        assert!(out.content.contains("characters truncated"), "{}", &out.content[..200.min(out.content.len())]);
    }
}
