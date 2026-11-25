//! Background shell commands (GRAFT-0104, after Claude Code's `run_in_background`
//! with BashOutput/KillShell): dev servers and watchers keep running while the
//! agent works; `bash_output` reads what they printed since the last look, or
//! stops them. Everything still running is stopped when the run ends.

use std::collections::BTreeMap;
use std::io::Read;
use std::process::Child;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use oxgraft_seams::{Action, EditFormat, Tool, ToolContext, ToolOutput, ToolSpec};
use serde_json::{Value, json};

/// Output kept per process; older output is dropped.
const MAX_BUFFER: usize = 1_000_000;
const MAX_PROCESSES: usize = 8;

struct Process {
    child: Child,
    output: Arc<Mutex<Vec<u8>>>,
    /// Bytes of `output` already shown to the model (adjusted when trimmed).
    shown: usize,
    command: String,
}

fn registry() -> &'static Mutex<BTreeMap<u32, Process>> {
    static R: OnceLock<Mutex<BTreeMap<u32, Process>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(BTreeMap::new()))
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// Register a spawned child whose stdout/stderr are piped; returns its id.
pub fn start(mut child: Child, command: &str) -> Result<u32, String> {
    let mut reg = registry().lock().map_err(|_| "background registry poisoned")?;
    let running = reg.values_mut().filter_map(|p| p.child.try_wait().ok()).filter(Option::is_none).count();
    if running >= MAX_PROCESSES {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("at most {MAX_PROCESSES} background processes; stop one with bash_output kill"));
    }
    let output = Arc::new(Mutex::new(Vec::new()));
    let pipes: Vec<Box<dyn Read + Send>> = [
        child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>),
        child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .collect();
    for mut pipe in pipes {
        let out = output.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = pipe.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if let Ok(mut o) = out.lock() {
                    o.extend_from_slice(&buf[..n]);
                }
            }
        });
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    reg.insert(id, Process { child, output, shown: 0, command: command.to_string() });
    Ok(id)
}

/// Stop a background process and, on Unix, its whole process group.
fn stop(child: &mut Child) {
    #[cfg(unix)]
    if matches!(child.try_wait(), Ok(None)) {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", "--", &format!("-{}", child.id())])
            .stderr(std::process::Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Stop every background process (called when a run or chat ends).
pub fn kill_all() {
    if let Ok(mut reg) = registry().lock() {
        for p in reg.values_mut() {
            stop(&mut p.child);
        }
        reg.clear();
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BashOutput;

impl Tool for BashOutput {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "bash_output".into(),
            description: "Read new output from a background command started with `bash` and `background: true`, \
                or stop it with `kill: true`. Without `id`, lists background commands."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": {"type": "integer", "description": "Background command id"},
                    "kill": {"type": "boolean", "description": "Stop the command"}
                },
                "required": []
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        Action { tool: "bash_output".into(), resource: input.get("id").map(Value::to_string).unwrap_or_default() }
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let Ok(mut reg) = registry().lock() else { return ToolOutput::error("background registry poisoned") };
        let Some(id) = input.get("id").and_then(Value::as_u64) else {
            if reg.is_empty() {
                return ToolOutput::ok("no background commands");
            }
            let lines: Vec<String> =
                reg.iter_mut().map(|(id, p)| format!("{id}: {} ({})", p.command, status(&mut p.child))).collect();
            return ToolOutput::ok(lines.join("\n"));
        };
        let Some(p) = reg.get_mut(&(id as u32)) else {
            return ToolOutput::error(format!("no background command {id}"));
        };
        if input.get("kill").and_then(Value::as_bool).unwrap_or(false) {
            stop(&mut p.child);
        }
        let state = status(&mut p.child);
        let new = {
            let Ok(mut out) = p.output.lock() else { return ToolOutput::error("output buffer poisoned") };
            if out.len() > MAX_BUFFER {
                let drop = out.len() - MAX_BUFFER;
                out.drain(..drop);
                p.shown = p.shown.saturating_sub(drop);
            }
            let text = String::from_utf8_lossy(&out[p.shown.min(out.len())..]).into_owned();
            p.shown = out.len();
            text
        };
        let body = if new.trim().is_empty() { "(no new output)".to_string() } else { new };
        ToolOutput::ok(crate::util::truncate(&format!("[{id}: {state}]\n{body}"), ctx.max_output_chars))
    }
}

fn status(child: &mut Child) -> String {
    match child.try_wait() {
        Ok(None) => "running".into(),
        Ok(Some(s)) => format!("exited with {}", s.code().map_or("a signal".into(), |c| c.to_string())),
        Err(e) => format!("unknown ({e})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    // GRAFT-0104
    #[test]
    fn background_commands_run_report_and_stop() {
        if !matches!(crate::shell_kind(), crate::ShellKind::Posix(_)) {
            return;
        }
        let f = Fixture::new("background");
        let out = f.run(&crate::Bash, json!({"command": "echo started; sleep 30", "background": true}));
        assert!(!out.is_error && out.content.contains("started background command"), "{}", out.content);
        let id: u64 = out.content.split_whitespace().nth(3).unwrap().trim_end_matches(';').parse().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let first = f.run(&BashOutput, json!({"id": id}));
        assert!(first.content.contains("running") && first.content.contains("started"), "{}", first.content);
        let again = f.run(&BashOutput, json!({"id": id}));
        assert!(again.content.contains("(no new output)"), "{}", again.content);
        let stopped = f.run(&BashOutput, json!({"id": id, "kill": true}));
        assert!(stopped.content.contains("exited"), "{}", stopped.content);
        assert!(f.run(&BashOutput, json!({"id": 999_999})).is_error);
        assert!(f.run(&BashOutput, json!({})).content.contains("echo started"));
    }
}
