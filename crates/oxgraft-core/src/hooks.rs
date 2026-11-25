//! User hooks around tool calls (GRAFT-0018, after Claude Code's hooks).
//!
//! ```toml
//! [[hooks]]
//! event = "post_tool"        # pre_tool | post_tool | session_end
//! tools = ["edit", "write"]  # empty = every tool
//! command = "cargo fmt --all"
//! ```
//!
//! The command runs in the workspace with `OXGRAFT_HOOK_EVENT`, `OXGRAFT_TOOL`,
//! `OXGRAFT_TOOL_INPUT` (JSON) and, for post_tool, `OXGRAFT_TOOL_OUTPUT`.
//! Exit code 2 is feedback for the model: a pre_tool hook blocks the call with its
//! stderr as the reason; a post_tool hook's stderr is appended to the tool result.
//! Other failures (non-zero exit, timeout, cannot start) of a pre_tool hook block the
//! call (fail closed, GRAFT-0033); for post_tool and session_end they are warnings.

use std::io::Read as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Deserialize;
use wait_timeout::ChildExt;

/// How to run a command line: `program args… <line>`. The CLI sets this to the
/// shell the `bash` tool uses (Git Bash `sh` or PowerShell on Windows), so hooks,
/// `verify_command` and model commands all behave the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    pub program: std::path::PathBuf,
    pub args: Vec<String>,
}

impl Default for Shell {
    fn default() -> Self {
        if cfg!(windows) {
            Self { program: "cmd".into(), args: vec!["/C".into()] }
        } else {
            Self { program: "sh".into(), args: vec!["-c".into()] }
        }
    }
}

impl Shell {
    pub fn command(&self, line: &str) -> Command {
        let mut c = Command::new(&self.program);
        c.args(&self.args).arg(line);
        c
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreTool,
    PostTool,
    SessionEnd,
    /// Before a task reaches the model (GRAFT-0062). Exit 2 blocks it; stdout is
    /// added to the task as context. `OXGRAFT_PROMPT` holds the task.
    UserPrompt,
    /// When a new session starts (GRAFT-0098): stdout is added to the first task as
    /// context (e.g. the current ticket or branch notes). Exit 2 stops the run.
    SessionStart,
    /// When the model wants to finish (GRAFT-0062). Exit 2 sends stderr back as
    /// feedback and the run continues. `OXGRAFT_FINAL_MESSAGE` holds its reply.
    Stop,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    pub event: HookEvent,
    #[serde(default)]
    pub tools: Vec<String>,
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookResult {
    Ok,
    /// `user_prompt` hooks only: text the hook printed, to add to the task.
    Context(String),
    /// Exit code 2: stderr is feedback for the model.
    Feedback(String),
    /// Hook failed to run or exited non-zero (not 2).
    Failed(String),
}

impl Hook {
    pub fn matches(&self, event: HookEvent, tool: &str) -> bool {
        self.event == event && (self.tools.is_empty() || self.tools.iter().any(|t| t == tool))
    }

    pub fn run(
        &self,
        shell: &Shell,
        workspace: &Path,
        event: HookEvent,
        tool: &str,
        input: &str,
        output: Option<&str>,
    ) -> HookResult {
        let mut cmd = shell.command(&self.command);
        let event_name = match event {
            HookEvent::PreTool => "pre_tool",
            HookEvent::PostTool => "post_tool",
            HookEvent::SessionEnd => "session_end",
            HookEvent::UserPrompt => "user_prompt",
            HookEvent::Stop => "stop",
            HookEvent::SessionStart => "session_start",
        };
        let capture = matches!(event, HookEvent::UserPrompt | HookEvent::SessionStart);
        cmd.current_dir(workspace)
            .stdin(Stdio::null())
            .stdout(if capture { Stdio::piped() } else { Stdio::null() })
            .stderr(Stdio::piped())
            .env("OXGRAFT_HOOK_EVENT", event_name)
            .env("OXGRAFT_TOOL", tool);
        let text: String = input.chars().take(100_000).collect();
        match event {
            HookEvent::UserPrompt => cmd.env("OXGRAFT_PROMPT", text),
            HookEvent::Stop => cmd.env("OXGRAFT_FINAL_MESSAGE", text),
            _ => cmd.env("OXGRAFT_TOOL_INPUT", text),
        };
        if let Some(out) = output {
            cmd.env("OXGRAFT_TOOL_OUTPUT", out.chars().take(100_000).collect::<String>());
        }
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return HookResult::Failed(format!("hook `{}` could not start: {e}", self.command)),
        };
        let mut stderr = child.stderr.take();
        let reader = std::thread::spawn(move || {
            let mut s = String::new();
            if let Some(e) = stderr.as_mut() {
                let _ = e.read_to_string(&mut s);
            }
            s
        });
        let mut stdout = child.stdout.take();
        let out_reader = std::thread::spawn(move || {
            let mut s = String::new();
            if let Some(o) = stdout.as_mut() {
                let _ = o.read_to_string(&mut s);
            }
            s
        });
        let status = match child.wait_timeout(Duration::from_secs(self.timeout_secs.max(1))) {
            Ok(Some(s)) => s,
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return HookResult::Failed(format!("hook `{}` timed out after {}s", self.command, self.timeout_secs));
            }
            Err(e) => return HookResult::Failed(format!("hook `{}`: {e}", self.command)),
        };
        // Hook output goes to the model: keep the end of very long output (GRAFT-0093,
        // after Claude Code 2.1.332 shortening hook text over ~50,000 characters).
        let err = keep_tail(reader.join().unwrap_or_default().trim(), MAX_HOOK_OUTPUT);
        let out: String = out_reader.join().unwrap_or_default().trim().chars().take(20_000).collect();
        match status.code() {
            Some(0) if capture && !out.is_empty() => HookResult::Context(out),
            Some(0) => HookResult::Ok,
            Some(2) => {
                HookResult::Feedback(if err.is_empty() { format!("blocked by hook `{}`", self.command) } else { err })
            }
            code => HookResult::Failed(format!("hook `{}` exited with {code:?}: {err}", self.command)),
        }
    }
}

/// Most hook output (stderr feedback) passed to the model.
const MAX_HOOK_OUTPUT: usize = 20_000;

/// The last `max` characters of `text`, with a note when something was cut.
fn keep_tail(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_string();
    }
    let tail: String = text.chars().skip(count - max).collect();
    format!(
        "[… {} earlier characters of hook output omitted]
{tail}",
        count - max
    )
}

/// Run a verification command (build, lint, tests) in the workspace. Returns whether it
/// passed and its combined output (tail-truncated), with a timeout.
pub fn run_check(shell: &Shell, command: &str, workspace: &Path, timeout_secs: u64) -> (bool, String) {
    let mut cmd = shell.command(command);
    cmd.current_dir(workspace).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return (false, format!("could not start `{command}`: {e}")),
    };
    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_string(&mut s);
        }
        if let Some(e) = err.as_mut() {
            let _ = e.read_to_string(&mut s);
        }
        s
    });
    let ok = match child.wait_timeout(Duration::from_secs(timeout_secs.max(1))) {
        Ok(Some(status)) => status.success(),
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            return (false, format!("`{command}` timed out after {timeout_secs}s"));
        }
        Err(e) => return (false, e.to_string()),
    };
    let text = reader.join().unwrap_or_default();
    let chars: Vec<char> = text.trim_end().chars().collect();
    let tail: String = chars[chars.len().saturating_sub(8_000)..].iter().collect();
    (ok, tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hook(command: &str) -> Hook {
        Hook { event: HookEvent::PreTool, tools: vec!["bash".into()], command: command.into(), timeout_secs: 5 }
    }

    #[test]
    fn matching_and_exit_codes() {
        let ws = std::env::temp_dir();
        assert!(hook("x").matches(HookEvent::PreTool, "bash"));
        assert!(!hook("x").matches(HookEvent::PreTool, "read"));
        assert!(!hook("x").matches(HookEvent::PostTool, "bash"));
        assert_eq!(hook("exit 0").run(&Shell::default(), &ws, HookEvent::PreTool, "bash", "{}", None), HookResult::Ok);
        let blocked = hook("echo no rm please 1>&2 && exit 2").run(
            &Shell::default(),
            &ws,
            HookEvent::PreTool,
            "bash",
            "{}",
            None,
        );
        assert_eq!(blocked, HookResult::Feedback("no rm please".into()));
        assert!(matches!(
            hook("exit 1").run(&Shell::default(), &ws, HookEvent::PreTool, "bash", "{}", None),
            HookResult::Failed(_)
        ));
    }

    #[test]
    fn parses_config() {
        #[derive(Deserialize)]
        struct C {
            hooks: Vec<Hook>,
        }
        let c: C = toml::from_str("[[hooks]]\nevent = \"post_tool\"\ncommand = \"fmt\"\n").unwrap();
        assert_eq!(c.hooks[0].event, HookEvent::PostTool);
        assert_eq!(c.hooks[0].timeout_secs, 60);
    }

    #[test]
    fn long_hook_output_is_shortened() {
        assert_eq!(keep_tail("short", 10), "short");
        let long = "x".repeat(30) + "END";
        let kept = keep_tail(&long, 5);
        assert!(kept.ends_with("xxEND") && kept.contains("28 earlier characters"), "{kept}");
    }
}
