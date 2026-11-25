//! The agent loop (SPEC-0003).

use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::hooks::{Hook, HookEvent, HookResult};

use oxgraft_protocol::{Event, PROTOCOL_VERSION, Role as EventRole};
use oxgraft_seams::{
    Action, Block, CompletionRequest, Decision, FileSnapshot, Message, Provider, Role, StopReason, Tool, ToolContext,
    ToolOutput, ToolSpec, Usage,
};
use serde_json::Value;

use crate::Toolkit;
use crate::retry::{RetryPolicy, complete_with_retries, stream_with_retries};
use crate::session::Session;

/// Answer to an approval prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Yes,
    No,
    /// Yes, and don't ask again for this tool in this session.
    Always,
    /// No, and here is what to do instead (GRAFT-0078).
    NoWith(String),
}

/// Asks the user about actions the approval policy marks `ask`.
pub trait Approver {
    fn approve(&self, tool: &str, action: &Action, input: &Value) -> Answer;
}

/// Headless approver: answers every `ask` the same way.
#[derive(Debug, Clone)]
pub struct Fixed(pub Answer);

impl Approver for Fixed {
    fn approve(&self, _: &str, _: &Action, _: &Value) -> Answer {
        self.0.clone()
    }
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub model: String,
    pub workspace: PathBuf,
    pub max_turns: u32,
    pub max_tokens: u32,
    pub max_output_chars: usize,
    pub retry: RetryPolicy,
    /// Extra system prompt text appended after the generated one.
    pub append_system: Option<String>,
    /// Set by the UI (e.g. Ctrl-C) to interrupt the current run.
    pub cancel: Arc<AtomicBool>,
    /// Stop when input + output tokens for one run exceed this.
    pub max_session_tokens: Option<u64>,
    pub hooks: Vec<Hook>,
    /// Model context window in tokens; compaction runs above 80% of it. `None` disables it.
    pub context_window: Option<u64>,
    /// Offer the read-only `task` subagent tool.
    pub subagents: bool,
    /// Command run after any turn that changed files; failures go back to the model.
    pub verify_command: Option<String>,
    /// Stream model output as `text_delta` events (ADR 0007).
    pub stream: bool,
    /// Snapshot the workspace into a private git repo after turns that may have
    /// changed files (GRAFT-0046). Off in core by default; the CLI turns it on.
    pub checkpoints: bool,
    /// Shell for hooks and `verify_command`.
    pub shell: crate::hooks::Shell,
    /// Command that must pass before the run may finish (`oxgraft run --until`).
    pub finish_check: Option<String>,
    /// Remind the model of unfinished todo items before finishing (off in plan mode).
    pub todo_reminder: bool,
    /// JSON schema the final answer must match (`run --output-schema`, GRAFT-0101).
    pub output_schema: Option<Value>,
}

impl AgentConfig {
    pub fn new(model: &str, workspace: PathBuf) -> Self {
        Self {
            model: model.into(),
            workspace,
            max_turns: 50,
            max_tokens: 16_000,
            max_output_chars: 30_000,
            retry: RetryPolicy::default(),
            append_system: None,
            cancel: Arc::new(AtomicBool::new(false)),
            max_session_tokens: None,
            hooks: Vec::new(),
            context_window: Some(200_000),
            subagents: true,
            verify_command: None,
            stream: true,
            checkpoints: false,
            shell: crate::hooks::Shell::default(),
            finish_check: None,
            todo_reminder: true,
            output_schema: None,
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

const INTERRUPTED: &str = "Not executed: the user interrupted the session.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The model finished its turn without asking for more tools.
    Done,
    /// A guard stopped the session (max turns, refusal, …).
    Stopped(String),
    /// A provider or internal error ended the session.
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub status: Status,
    pub turns: u32,
    pub usage: Usage,
    /// The final assistant text, if any.
    pub final_text: String,
}

const MAX_CONTINUATIONS: u32 = 3;
/// What an "always" answer approves (GRAFT-0069, after Claude Code's per-command
/// "don't ask again"): the whole tool, except for `bash`, where it is the command
/// prefix of each part (`cargo test`, `npm run build`), so approving `cargo test`
/// never approves `rm`. Lines with command substitution give no keys.
pub fn always_keys(tool: &str, resource: &str) -> Vec<String> {
    if tool == "web_fetch" {
        // Per host: approving docs.rs doesn't approve every site.
        return url_host(resource).map(|h| vec![format!("web_fetch:{h}")]).unwrap_or_default();
    }
    if tool != "bash" {
        return vec![tool.to_string()];
    }
    // Anything that can write or run more than the visible command gives no key, so
    // "always" is just "yes" for it: substitution, redirection, process substitution,
    // PowerShell subexpressions, escapes and quotes (GRAFT-0069 revision).
    if resource.contains(['$', '`', '>', '<', '(', ')', '{', '}', '\\', '"', '\'', '@']) {
        return Vec::new();
    }
    let parts = shell_parts(resource);
    let keys: Vec<String> = parts.iter().filter_map(|p| command_prefix(p)).map(|p| format!("bash:{p}")).collect();
    // All or nothing: a part without a key makes the whole line keyless.
    if keys.len() == parts.len() { keys } else { Vec::new() }
}

/// Whether earlier "always" answers cover this action: for `bash`, every part.
fn always_covers(always: &HashSet<String>, tool: &str, resource: &str) -> bool {
    if tool == "web_fetch" {
        let keys = always_keys(tool, resource);
        return !keys.is_empty() && keys.iter().all(|k| always.contains(k));
    }
    if tool != "bash" {
        return always.contains(tool);
    }
    let keys = always_keys(tool, resource);
    let parts = shell_parts(resource).len();
    !keys.is_empty() && keys.len() == parts && keys.iter().all(|k| always.contains(k))
}

/// Lowercased host of an http(s) URL, parsed the way the HTTP client parses it
/// (so `https://evil.com\\@docs.rs/` is evil.com, not docs.rs).
fn url_host(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let host = parsed.host_str()?.trim_end_matches('.').to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// Split on `;`, `&`, `|` and newlines (conservative: quotes are not honoured).
fn shell_parts(command: &str) -> Vec<&str> {
    command.split([';', '&', '|', '\n']).map(str::trim).filter(|p| !p.is_empty()).collect()
}

/// `cargo test --release` → `cargo test`; `ls -la` → `ls`; `FOO=1 make` → none.
fn command_prefix(part: &str) -> Option<String> {
    // Commands that run other commands or code: approving them approves anything.
    const WRAPPERS: &[&str] = &[
        "sh",
        "bash",
        "zsh",
        "dash",
        "fish",
        "pwsh",
        "powershell",
        "cmd",
        "python",
        "python3",
        "node",
        "deno",
        "perl",
        "ruby",
        "php",
        "env",
        "xargs",
        "find",
        "timeout",
        "nohup",
        "nice",
        "time",
        "watch",
        "exec",
        "eval",
        "source",
        ".",
        "command",
        "builtin",
        "sudo",
        "doas",
        "su",
        "ssh",
        "npx",
        "bunx",
        "iex",
        "invoke-expression",
        "start-process",
        "awk",
        "sed",
    ];
    let mut words = part.split_whitespace();
    let first = words.next()?;
    if first.contains('=') || first.contains(['(', ')', '{', '}', '<', '>']) {
        return None;
    }
    let name = first.rsplit(['/', '\\']).next().unwrap_or(first).to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    if WRAPPERS.contains(&name) {
        return None;
    }
    // Tools whose second word picks a subcommand: `git status` and `git push` differ.
    const SUBCOMMANDS: &[&str] = &[
        "git",
        "cargo",
        "npm",
        "pnpm",
        "yarn",
        "bun",
        "npx",
        "go",
        "docker",
        "kubectl",
        "uv",
        "poetry",
        "pip",
        "pip3",
        "dotnet",
        "gradle",
        "mvn",
        "make",
        "rustup",
        "brew",
        "apt",
        "gh",
        "terraform",
        "helm",
    ];
    match words.next() {
        Some(second) if SUBCOMMANDS.contains(&first) => {
            let plain = !second.starts_with(['-', '+'])
                && second.chars().all(|c| c.is_ascii_alphanumeric() || "-_:.".contains(c));
            plain.then(|| format!("{first} {second}"))
        }
        None if SUBCOMMANDS.contains(&first) => None,
        _ => Some(first.to_string()),
    }
}

/// Private re-asks for malformed tool calls per turn (GRAFT-0044).
const MAX_REQUERIES: u32 = 2;

/// Tool calls in `reply` that cannot run at all: unknown tool, input that is not a
/// JSON object (e.g. unparseable arguments), or a missing required field.
fn malformed_calls(reply: &Message, specs: &[ToolSpec]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (id, name, input) in reply.tool_uses() {
        let Some(spec) = specs.iter().find(|s| s.name == name) else {
            let names: Vec<&str> = specs.iter().map(|s| s.name.as_str()).collect();
            out.push((id.to_string(), format!("unknown tool `{name}`; available tools: {}", names.join(", "))));
            continue;
        };
        let Some(object) = input.as_object() else {
            out.push((id.to_string(), format!("the input for `{name}` is not a valid JSON object")));
            continue;
        };
        // `edit` also accepts old_string/new_string in place of `edit` (GRAFT-0035).
        let alternative = name == "edit" && (object.contains_key("old_string") || object.contains_key("edits"));
        let missing: Vec<&str> = spec.input_schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|field| !object.contains_key(*field))
            .filter(|field| !(alternative && *field == "edit"))
            .collect();
        if !missing.is_empty() {
            out.push((id.to_string(), format!("`{name}` is missing required field(s): {}", missing.join(", "))));
        }
    }
    out
}
const LOOP_LIMIT: usize = 3;
/// Tool calls executed per model turn; extra calls get an error result (stability).
const MAX_CALLS_PER_TURN: usize = 32;
/// Consecutive failing verify rounds before automatic checks pause (Aider uses 3).
const MAX_VERIFY_ROUNDS: u32 = 3;
/// Times a failing finish check is sent back before the run stops (GRAFT-0054).
const MAX_FINISH_ROUNDS: u32 = 3;
/// Times stop hooks may send the model back before finishing is accepted (GRAFT-0062).
const MAX_STOP_ROUNDS: u32 = 3;
const MISTAKE_NUDGE: u32 = 6;
const MISTAKE_STOP: u32 = 12;

pub struct Agent<'a> {
    pub toolkit: &'a Toolkit,
    pub provider: &'a dyn Provider,
    pub tools: Vec<Box<dyn Tool>>,
    pub approver: &'a dyn Approver,
    pub config: AgentConfig,
    /// Sleep used between retries; tests replace it.
    pub sleep: fn(Duration),
    /// Provider and model for the read-only `task` subagent (GRAFT-0081); the main
    /// ones when `None`.
    pub subagent: Option<(&'a dyn Provider, String)>,
}

impl std::fmt::Debug for Agent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent").field("config", &self.config).finish_non_exhaustive()
    }
}

struct RunState {
    always: HashSet<String>,
    recent: Vec<(String, Value)>,
    depth: u32,
    /// Files the model has read (or written) this run, with the mtime it saw.
    seen: HashMap<PathBuf, Option<std::time::SystemTime>>,
    /// Subdirectory instruction files already shown this run.
    instructions: HashSet<PathBuf>,
    /// Checkpoint repo, when enabled and working.
    checkpoints: Option<crate::checkpoint::Checkpoints>,
    /// The model's latest `todo` list, for the unfinished-plan reminder (GRAFT-0059).
    todo: Vec<(String, String)>,
    todo_reminded: bool,
}

/// Tools that can't change the workspace, so a turn using only these needs no checkpoint.
const READ_ONLY_TOOLS: &[&str] = &["read", "ls", "glob", "grep", "map", "skill", "todo", "task", "web_fetch"];

/// Tools a subagent may use: read-only, so its work never needs undo.
const SUBAGENT_TOOLS: &[&str] = &["read", "ls", "glob", "grep", "map", "skill"];

impl<'a> Agent<'a> {
    pub fn new(
        toolkit: &'a Toolkit,
        provider: &'a dyn Provider,
        tools: Vec<Box<dyn Tool>>,
        approver: &'a dyn Approver,
        config: AgentConfig,
    ) -> Self {
        Self { toolkit, provider, tools, approver, config, sleep: std::thread::sleep, subagent: None }
    }

    /// Tools visible at a nesting depth: everything at the top level (plus `task`
    /// when subagents are enabled); read-only tools inside a subagent.
    fn specs(&self, depth: u32) -> Vec<ToolSpec> {
        let mut specs: Vec<ToolSpec> = self
            .tools
            .iter()
            .map(|t| t.spec(self.toolkit.edit_format.as_ref()))
            .filter(|s| depth == 0 || SUBAGENT_TOOLS.contains(&s.name.as_str()))
            .collect();
        if depth == 0 && self.config.subagents {
            specs.push(ToolSpec {
                name: "task".into(),
                description: "Delegate a self-contained exploration or research question to a subagent with a fresh \
                    context. It can read, list, glob and grep (no edits or commands) and returns a concise report. \
                    Use it to search large codebases without filling your own context; give it a precise prompt."
                    .into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "description": {"type": "string", "description": "3-5 word label"},
                        "prompt": {"type": "string", "description": "What to find out, and what to report back"}
                    },
                    "required": ["prompt"]
                }),
            });
        }
        specs
    }

    /// Self-modification and persistence paths (after Qwen Code's protected list).
    fn is_protected(&self, path: &std::path::Path) -> bool {
        let root = self.config.workspace.canonicalize().unwrap_or_else(|_| self.config.workspace.clone());
        let rel = path.strip_prefix(&root).unwrap_or(path).to_string_lossy().replace('\\', "/");
        const DIRS: &[&str] = &[".oxgraft/", ".git/", ".github/workflows/", ".agents/", ".claude/", ".gemini/"];
        const FILES: &[&str] = &[".mcp.json", "AGENTS.md", "CLAUDE.md", "GEMINI.md"];
        DIRS.iter().any(|d| rel.starts_with(d)) || FILES.iter().any(|f| rel == *f || rel.ends_with(&format!("/{f}")))
    }

    /// Run a read-only subagent (GRAFT-0026) and return its final report.
    fn subagent(&self, session: &mut Session, input: &Value) -> ToolOutput {
        let Some(prompt) = input.get("prompt").and_then(Value::as_str).filter(|p| !p.trim().is_empty()) else {
            return ToolOutput::error("missing required field `prompt`");
        };
        let mut sub = Session::new(self.backend(1).1, "subagent", &self.config.workspace);
        let label = input.get("description").and_then(Value::as_str).unwrap_or("subagent");
        sub.title = format!("[subagent] {label}");
        let task = format!(
            "{prompt}\n\nYou are a read-only subagent: investigate with the available tools, then reply with a concise, \
             specific report (file paths, line numbers, findings). Do not ask questions."
        );
        let outcome = self.run_at(&mut sub, &task, &mut |_| {}, 1);
        // Book the subagent's tokens to the model(s) it actually used.
        for (model, usage) in &sub.usage_by_model {
            session.add_usage(model, *usage);
        }
        match outcome.status {
            Status::Done if !outcome.final_text.trim().is_empty() => ToolOutput::ok(outcome.final_text),
            Status::Done => ToolOutput::error("the subagent finished without a report"),
            Status::Stopped(r) | Status::Failed(r) => {
                ToolOutput::error(format!("the subagent stopped early ({r}). Partial report:\n{}", outcome.final_text))
            }
        }
    }

    /// Provider and model for a run at `depth` (subagents may use a cheaper model).
    fn backend(&self, depth: u32) -> (&dyn Provider, &str) {
        match (&self.subagent, depth) {
            (Some((provider, model)), d) if d > 0 => (*provider, model.as_str()),
            _ => (self.provider, self.config.model.as_str()),
        }
    }

    pub fn system_prompt(&self) -> String {
        let mut p = crate::prompt::system_prompt(&self.config.workspace, &crate::prompt::today());
        if let Some(schema) = &self.config.output_schema {
            p.push_str(&format!(
                "\n\nOUTPUT FORMAT: when you finish, reply with only a JSON object (no prose, no code fence) matching \
                 this JSON schema:\n{}",
                serde_json::to_string_pretty(schema).unwrap_or_default()
            ));
        }
        if let Some(extra) = &self.config.append_system {
            p.push_str("\n\n");
            p.push_str(extra);
        }
        p
    }

    /// Run one user task to completion. `emit` receives every protocol event;
    /// `session` is saved after every turn.
    pub fn run(&self, session: &mut Session, task: &str, emit: &mut dyn FnMut(&Event)) -> Outcome {
        self.run_at(session, task, emit, 0)
    }

    fn run_at(&self, session: &mut Session, task: &str, emit: &mut dyn FnMut(&Event), depth: u32) -> Outcome {
        emit(&Event::SessionStarted {
            protocol: PROTOCOL_VERSION,
            profile: self.toolkit.profile.name.clone(),
            session_id: Some(session.id.clone()),
            model: Some(self.backend(depth).1.to_string()),
        });
        for e in self.toolkit.session_preamble().into_iter().skip(1) {
            emit(&e);
        }
        if session.title.is_empty() {
            session.title = task.lines().next().unwrap_or("").chars().take(80).collect();
        }
        let repaired = session.repair();
        if repaired > 0 {
            emit(&Event::Warning {
                message: format!("closed {repaired} unanswered tool call(s) left by an interrupted run"),
            });
        }
        let (mut task_text, attached) = if depth == 0 {
            let policy = &self.toolkit.approval;
            let may_read =
                |rel: &str| policy.decide(&Action { tool: "read".into(), resource: rel.into() }) != Decision::Deny;
            crate::prompt::expand_mentions(&self.config.workspace, task, &may_read)
        } else {
            (task.to_string(), Vec::new())
        };
        // session_start hooks (GRAFT-0098) for a new session, then user_prompt hooks
        // (GRAFT-0062): add context, or block the task.
        if depth == 0 {
            let new_session = session.messages.is_empty();
            let events: &[HookEvent] =
                if new_session { &[HookEvent::SessionStart, HookEvent::UserPrompt] } else { &[HookEvent::UserPrompt] };
            for hook in self.config.hooks.iter().filter(|h| events.contains(&h.event)) {
                let event = hook.event;
                let name = if event == HookEvent::SessionStart { "session_start" } else { "user_prompt" };
                match hook.run(&self.config.shell, &self.config.workspace, event, "", task, None) {
                    HookResult::Ok => {}
                    HookResult::Context(extra) => {
                        task_text.push_str(&format!("\n\n[Context added by a {name} hook:]\n{extra}"));
                    }
                    HookResult::Feedback(reason) => {
                        let status = Status::Stopped(format!("blocked by a {name} hook: {reason}"));
                        return self.finish(session, emit, status, 0, Usage::default(), String::new());
                    }
                    HookResult::Failed(msg) => emit(&Event::Warning { message: msg }),
                }
            }
        }
        session.messages.push(Message::user(task_text));
        self.save(session, emit);

        let system = self.system_prompt();
        let specs = self.specs(depth);
        let mut state = RunState {
            always: HashSet::new(),
            recent: Vec::new(),
            depth,
            seen: HashMap::new(),
            instructions: HashSet::new(),
            checkpoints: None,
            todo: Vec::new(),
            todo_reminded: false,
        };
        // Attached files count as read, so the model can edit them straight away.
        for path in attached {
            let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            state.seen.insert(path, mtime);
        }
        if self.config.checkpoints && depth == 0 {
            match crate::checkpoint::Checkpoints::open(&self.config.workspace) {
                Ok(cp) => {
                    state.checkpoints = Some(cp);
                    self.checkpoint(session, &mut state, "start", emit);
                    session.run_start = session.checkpoints.len().checked_sub(1);
                }
                Err(e) => emit(&Event::Warning { message: format!("checkpoints off: {e}") }),
            }
        }
        let mut usage = Usage::default();
        let mut continuations = 0;
        let mut changed_files = false;
        let mut verify_failures = 0;
        let mut finish_failures = 0;
        let mut stop_rounds = 0;
        let mut consecutive_errors = 0u32;
        let mut final_text = String::new();

        for turn in 1..=self.config.max_turns {
            if self.config.cancelled() {
                return self.finish(
                    session,
                    emit,
                    Status::Stopped("interrupted by the user".into()),
                    turn - 1,
                    usage,
                    final_text,
                );
            }
            if let Some(budget) = self.config.max_session_tokens
                && usage.input_tokens + usage.output_tokens > budget
            {
                let reason = format!("token budget exhausted ({budget} tokens)");
                return self.finish(session, emit, Status::Stopped(reason), turn - 1, usage, final_text);
            }
            if depth == 0 {
                emit(&Event::TurnStarted { turn });
            }
            let request = CompletionRequest {
                model: self.backend(depth).1.to_string(),
                system: system.clone(),
                messages: session.messages.clone(),
                tools: specs.clone(),
                max_tokens: self.config.max_tokens,
            };
            let completion = if self.config.stream {
                // Both closures need `emit`; route retries through a cell (ADR 0007).
                let emit_cell = std::cell::RefCell::new(&mut *emit);
                stream_with_retries(
                    self.backend(depth).0,
                    &request,
                    &self.config.retry,
                    &|d| (self.sleep)(d),
                    &mut |attempt, wait, err| {
                        (emit_cell.borrow_mut())(&Event::Retrying {
                            attempt,
                            wait_ms: wait.as_millis() as u64,
                            message: err.to_string(),
                        })
                    },
                    &mut |text| (emit_cell.borrow_mut())(&Event::TextDelta { text: text.to_string() }),
                )
            } else {
                complete_with_retries(
                    self.backend(depth).0,
                    &request,
                    &self.config.retry,
                    &|d| (self.sleep)(d),
                    &mut |attempt, wait, err| {
                        emit(&Event::Retrying { attempt, wait_ms: wait.as_millis() as u64, message: err.to_string() })
                    },
                )
            };
            let mut completion = match completion {
                Ok(c) => c,
                Err(e) => return self.finish(session, emit, Status::Failed(e.to_string()), turn, usage, final_text),
            };
            usage.add(completion.usage);
            session.add_usage(self.backend(depth).1, completion.usage);
            if depth == 0 {
                session.context_tokens = completion.usage.input_tokens + completion.usage.output_tokens;
            }

            // Re-query malformed tool calls (GRAFT-0044, after SWE-agent's format-error
            // requery): the bad attempt and its error stay out of the history, so later
            // turns don't learn from it. If the corrected reply carries provider-signed
            // blocks (thinking), they were produced after the bad attempt, so the whole
            // trail is kept instead: history stays truthful and append-only.
            let mut trail: Vec<Message> = Vec::new();
            let mut requeries = 0;
            while requeries < MAX_REQUERIES && completion.stop != StopReason::MaxTokens {
                let bad = Message::assistant(completion.content.clone());
                let errors = malformed_calls(&bad, &specs);
                if errors.is_empty() {
                    break;
                }
                requeries += 1;
                emit(&Event::Warning {
                    message: format!("malformed tool call ({}); asking the model again", errors[0].1),
                });
                let results: Vec<Block> = bad
                    .tool_uses()
                    .into_iter()
                    .map(|(id, _, _)| {
                        let why = errors.iter().find(|(e, _)| e == id).map_or(
                            "Not executed: another call in this reply was malformed. Send all calls again.".to_string(),
                            |(_, w)| format!("Not executed: {w}. Send a corrected call."),
                        );
                        Block::ToolResult { tool_use_id: id.to_string(), content: why, is_error: true }
                    })
                    .collect();
                trail.push(bad);
                trail.push(Message { role: Role::User, content: results });
                let mut messages = session.messages.clone();
                messages.extend(trail.iter().cloned());
                let retry = CompletionRequest { messages, ..request.clone() };
                match complete_with_retries(
                    self.backend(depth).0,
                    &retry,
                    &self.config.retry,
                    &|d| (self.sleep)(d),
                    &mut |_, _, _| {},
                ) {
                    Ok(c) => {
                        usage.add(c.usage);
                        session.add_usage(self.backend(depth).1, c.usage);
                        completion = c;
                    }
                    Err(_) => {
                        // Keep the last malformed reply; its calls fail normally below.
                        trail.truncate(trail.len() - 2);
                        break;
                    }
                }
            }
            if completion.content.iter().any(|b| matches!(b, Block::Opaque { .. })) {
                session.messages.append(&mut trail);
            }

            let assistant = Message::assistant(completion.content);
            let text = assistant.text();
            if !text.trim().is_empty() {
                final_text = text.clone();
                emit(&Event::Message { role: EventRole::Assistant, text });
            }
            let calls: Vec<(String, String, Value)> =
                assistant.tool_uses().into_iter().map(|(i, n, v)| (i.to_string(), n.to_string(), v.clone())).collect();
            session.messages.push(assistant);
            self.save(session, emit);

            if completion.stop == StopReason::Refusal {
                return self.finish(
                    session,
                    emit,
                    Status::Stopped("the model refused".into()),
                    turn,
                    usage,
                    final_text,
                );
            }

            if calls.is_empty() {
                if completion.stop == StopReason::MaxTokens && continuations < MAX_CONTINUATIONS {
                    continuations += 1;
                    emit(&Event::Warning { message: "output hit max_tokens; asking the model to continue".into() });
                    session.messages.push(Message::user(
                        "Your reply was cut off by the output limit. Continue exactly where you stopped.",
                    ));
                    continue;
                }
                // A reply with no text and no tool calls (only thinking, or nothing) is
                // not an answer: ask once more instead of ending silently (GRAFT-0090,
                // after goose 1.61's recovery from thinking-only turns).
                if session.messages.last().is_some_and(|m| m.text().trim().is_empty())
                    && completion.stop != StopReason::Refusal
                    && continuations < MAX_CONTINUATIONS
                {
                    continuations += 1;
                    emit(&Event::Warning {
                        message: "the model replied without text or tool calls; asking it to continue".into(),
                    });
                    session.messages.push(Message::user(
                        "Your last reply had no visible text and no tool calls. Continue the task: call a tool, or give your final answer.",
                    ));
                    continue;
                }
                // Unfinished plan (GRAFT-0059, after OpenCode's and Claude Code's todo
                // reminders): once per run, point out items not marked done.
                let open: Vec<&str> =
                    state.todo.iter().filter(|(_, st)| st != "done").map(|(t, _)| t.as_str()).collect();
                let last_turn = turn == self.config.max_turns;
                if !open.is_empty() && !state.todo_reminded && self.config.todo_reminder && !last_turn {
                    state.todo_reminded = true;
                    emit(&Event::Warning {
                        message: format!("{} todo item(s) not done; reminding the model", open.len()),
                    });
                    session.messages.push(Message::user(format!(
                        "[Your todo list still has {} item(s) not marked done: {}. Finish them, or update the list \
                         (mark them done, or remove ones that no longer apply) and say why, then finish.]",
                        open.len(),
                        open.join("; ")
                    )));
                    self.save(session, emit);
                    continue;
                }
                // Structured output (GRAFT-0101, after Codex's `exec --output-schema`):
                // the final answer must be a JSON object matching the schema.
                if let Some(schema) = &self.config.output_schema
                    && depth == 0
                {
                    let problems = crate::schema::check_answer(&final_text, schema);
                    if !problems.is_empty() && !last_turn && finish_failures < MAX_FINISH_ROUNDS {
                        finish_failures += 1;
                        emit(&Event::Warning {
                            message: "final answer doesn't match the output schema; asking again".into(),
                        });
                        session.messages.push(Message::user(format!(
                            "[Your final answer must be only a JSON object matching the schema. Problems: {}]",
                            problems.join("; ")
                        )));
                        self.save(session, emit);
                        continue;
                    }
                }
                // Finish check (GRAFT-0054, after Qwen Code's /goal verifier): "done" only
                // counts once an independent check passes; failures go back to the model.
                if let Some(check) = &self.config.finish_check
                    && depth == 0
                {
                    let (ok, out) = crate::hooks::run_check(&self.config.shell, check, &self.config.workspace, 600);
                    if !ok {
                        if finish_failures >= MAX_FINISH_ROUNDS || last_turn {
                            let reason = if last_turn {
                                format!("finish check `{check}` failed and no turns are left")
                            } else {
                                format!("finish check `{check}` still failing after {MAX_FINISH_ROUNDS} rounds")
                            };
                            return self.finish(session, emit, Status::Stopped(reason), turn, usage, final_text);
                        }
                        finish_failures += 1;
                        emit(&Event::Warning { message: format!("finish check `{check}` failed; continuing") });
                        session.messages.push(Message::user(format!(
                            "[The task is not done: the check `{check}` failed. Fix the problems below, then finish \
                             again.]\n{out}"
                        )));
                        self.save(session, emit);
                        continue;
                    }
                }
                // stop hooks (GRAFT-0062): exit 2 sends feedback and the run goes on.
                if depth == 0 && stop_rounds < MAX_STOP_ROUNDS && !last_turn {
                    let mut feedback = Vec::new();
                    for hook in self.config.hooks.iter().filter(|h| h.event == HookEvent::Stop) {
                        match hook.run(
                            &self.config.shell,
                            &self.config.workspace,
                            HookEvent::Stop,
                            "",
                            &final_text,
                            None,
                        ) {
                            HookResult::Feedback(msg) => feedback.push(msg),
                            HookResult::Failed(msg) => emit(&Event::Warning { message: msg }),
                            HookResult::Ok | HookResult::Context(_) => {}
                        }
                    }
                    if !feedback.is_empty() {
                        stop_rounds += 1;
                        emit(&Event::Warning { message: "a stop hook asked the model to continue".into() });
                        session.messages.push(Message::user(format!(
                            "[A stop hook did not accept finishing yet:]\n{}",
                            feedback.join("\n")
                        )));
                        self.save(session, emit);
                        continue;
                    }
                }
                return self.finish(session, emit, Status::Done, turn, usage, final_text);
            }

            let truncated = completion.stop == StopReason::MaxTokens;
            let may_change = calls.iter().any(|(_, name, _)| !READ_ONLY_TOOLS.contains(&name.as_str()));
            let mut results = Vec::with_capacity(calls.len());
            for (index, (id, name, input)) in calls.into_iter().enumerate() {
                emit(&Event::ToolCall { id: id.clone(), tool: name.clone(), input: input.clone() });
                if name == "todo"
                    && let Some(items) = input["items"].as_array()
                {
                    state.todo = items
                        .iter()
                        .map(|i| {
                            let text = i["text"].as_str().unwrap_or("").to_string();
                            (text, i["status"].as_str().unwrap_or("pending").to_string())
                        })
                        .collect();
                }
                let output = if index >= MAX_CALLS_PER_TURN {
                    ToolOutput::error(format!(
                        "Not executed: at most {MAX_CALLS_PER_TURN} tool calls run per turn. Repeat the remaining calls in your next turn."
                    ))
                } else if self.config.cancelled() {
                    ToolOutput::error(INTERRUPTED)
                } else if truncated {
                    ToolOutput::error(
                        "Not executed: your response hit the output token limit, so this tool call may be incomplete. \
                         Retry with a smaller step (e.g. several smaller edits instead of one large write).",
                    )
                } else {
                    self.execute(session, &mut state, &id, &name, &input, emit)
                };
                emit(&Event::ToolResult { id: id.clone(), ok: !output.is_error, output: output.content.clone() });
                if matches!(name.as_str(), "edit" | "write") && !output.is_error {
                    changed_files = true;
                }
                consecutive_errors = if output.is_error { consecutive_errors + 1 } else { 0 };
                results.push(Block::ToolResult { tool_use_id: id, content: output.content, is_error: output.is_error });
            }
            // Automatic verification after edits (GRAFT-0039, Aider's lint/test-and-fix loop).
            if changed_files
                && let Some(command) = &self.config.verify_command
                && verify_failures < MAX_VERIFY_ROUNDS
            {
                let (ok, out) = crate::hooks::run_check(&self.config.shell, command, &self.config.workspace, 300);
                if ok {
                    verify_failures = 0;
                } else {
                    verify_failures += 1;
                    emit(&Event::Warning { message: format!("verify command `{command}` failed") });
                    results.push(Block::text(format!(
                        "[Automatic check `{command}` failed after your changes. Fix the problems below.]\n{out}"
                    )));
                }
            }
            changed_files = false;
            // Consecutive-mistake breaker (GRAFT-0041, after Cline's mistake tracker):
            // nudge at 6 failed tool calls in a row, stop at 12.
            if consecutive_errors == MISTAKE_NUDGE {
                results.push(Block::text(format!(
                    "[{MISTAKE_NUDGE} tool calls in a row have failed. Stop and reconsider: re-read the relevant files, \
                     check your assumptions, and try a different approach.]"
                )));
            }
            let stop = consecutive_errors >= MISTAKE_STOP;
            session.messages.push(Message { role: Role::User, content: results });
            if may_change {
                self.checkpoint(session, &mut state, &format!("turn {turn}"), emit);
            }
            self.save(session, emit);
            if stop {
                let reason = format!("{MISTAKE_STOP} consecutive tool errors");
                return self.finish(session, emit, Status::Stopped(reason), turn, usage, final_text);
            }

            let context_used = completion.usage.input_tokens + completion.usage.output_tokens;
            if let Some(window) = self.config.context_window
                && context_used > window * 8 / 10
            {
                self.compact(session, task, &system, &mut usage, emit, depth);
            }
        }
        let turns = self.config.max_turns;
        self.finish(session, emit, Status::Stopped(format!("reached max turns ({turns})")), turns, usage, final_text)
    }

    /// Take a checkpoint and record it if the workspace changed. A failing checkpoint
    /// repo is reported once and switched off; it never stops the session.
    fn checkpoint(&self, session: &mut Session, state: &mut RunState, label: &str, emit: &mut dyn FnMut(&Event)) {
        let Some(cp) = &state.checkpoints else { return };
        match cp.snapshot(&format!("{} {label}", session.id)) {
            Ok(commit) => {
                if session.checkpoints.last().is_none_or(|c| c.commit != commit) {
                    session.checkpoints.push(crate::session::CheckpointRecord { label: label.into(), commit });
                }
            }
            Err(e) => {
                emit(&Event::Warning { message: format!("checkpoints off for this run: {e}") });
                state.checkpoints = None;
            }
        }
    }

    /// Compact now, at the user's request (GRAFT-0076, after Claude Code's `/compact`).
    /// Returns false if there was nothing to compact or the summary failed.
    pub fn compact_session(&self, session: &mut Session, emit: &mut dyn FnMut(&Event)) -> bool {
        if session.messages.len() < 2 {
            return false;
        }
        let task = session.messages.iter().find(|m| m.role == Role::User).map(Message::text).unwrap_or_default();
        let before = session.compactions;
        let mut usage = Usage::default();
        self.compact(session, &task, &self.system_prompt(), &mut usage, emit, 0);
        session.compactions > before
    }

    /// Context compaction (GRAFT-0023, after Pi, OpenCode and Claude Code): ask the model
    /// for a handover summary, archive the old messages, and continue from the summary.
    /// The history is never edited in place, so provider-specific blocks stay valid.
    fn compact(
        &self,
        session: &mut Session,
        task: &str,
        system: &str,
        usage: &mut Usage,
        emit: &mut dyn FnMut(&Event),
        depth: u32,
    ) {
        // Summarise with the model that produced this history (review fix).
        let (provider, model) = self.backend(depth);
        let mut messages = session.messages.clone();
        messages.push(Message::user(
            "Context is nearly full. Write a handover summary for yourself so you can continue in a fresh context: \
             the task, what you have done (files changed and why), what you verified, the current state, open \
             problems, and the exact next steps. Do not call tools.",
        ));
        let request = CompletionRequest {
            model: model.to_string(),
            system: system.to_string(),
            messages,
            tools: Vec::new(),
            max_tokens: self.config.max_tokens,
        };
        let summary =
            complete_with_retries(provider, &request, &self.config.retry, &|d| (self.sleep)(d), &mut |_, _, _| {}).map(
                |c| {
                    usage.add(c.usage);
                    session.add_usage(model, c.usage);
                    Message::assistant(c.content).text()
                },
            );
        let summary = match summary {
            Ok(s) if !s.trim().is_empty() => s,
            Ok(_) | Err(_) => {
                emit(&Event::Warning { message: "context compaction failed; continuing with the full history".into() });
                return;
            }
        };
        let old = std::mem::take(&mut session.messages);
        session.archived_messages += old.len();
        session.compactions += 1;
        session.messages.push(Message::user(format!(
            "{task}\n\n[Context was compacted. Your handover summary of the work so far:]\n{summary}\n\n\
             Continue the task from where you left off."
        )));
        emit(&Event::Warning { message: format!("context compacted ({} messages summarised)", old.len()) });
        self.save(session, emit);
    }

    fn execute(
        &self,
        session: &mut Session,
        state: &mut RunState,
        id: &str,
        name: &str,
        input: &Value,
        emit: &mut dyn FnMut(&Event),
    ) -> ToolOutput {
        let edit = self.toolkit.edit_format.as_ref();
        if name == "task" && state.depth == 0 && self.config.subagents {
            return self.subagent(session, input);
        }
        let visible = state.depth == 0 || SUBAGENT_TOOLS.contains(&name);
        let Some(tool) = self.tools.iter().find(|t| visible && t.spec(edit).name == name) else {
            let names: Vec<String> = self.specs(state.depth).into_iter().map(|s| s.name).collect();
            return ToolOutput::error(format!("unknown tool `{name}`; available tools: {}", names.join(", ")));
        };

        // Loop guard (SPEC-0003 AC2): block a 4th identical consecutive call.
        let key = (name.to_string(), input.clone());
        let repeats = state.recent.iter().rev().take_while(|k| **k == key).count();
        if repeats >= LOOP_LIMIT {
            emit(&Event::Warning { message: format!("blocked repeated `{name}` call (loop guard)") });
            return ToolOutput::error(format!(
                "Blocked: you have made this exact `{name}` call {repeats} times in a row with the same input. \
                 It will not change the result. Try a different approach."
            ));
        }
        state.recent.push(key);
        if state.recent.len() > 20 {
            state.recent.remove(0);
        }

        let input_json = input.to_string();
        for hook in self.config.hooks.iter().filter(|h| h.matches(HookEvent::PreTool, name)) {
            match hook.run(&self.config.shell, &self.config.workspace, HookEvent::PreTool, name, &input_json, None) {
                HookResult::Ok | HookResult::Context(_) => {}
                HookResult::Feedback(reason) => {
                    emit(&Event::ApprovalResolved { id: id.into(), decision: "deny".into() });
                    return ToolOutput::error(format!("Blocked by a pre_tool hook: {reason}"));
                }
                // Fail closed (GRAFT-0033): a guard that cannot run must not wave the call through.
                HookResult::Failed(msg) => {
                    emit(&Event::Warning { message: msg.clone() });
                    emit(&Event::ApprovalResolved { id: id.into(), decision: "deny".into() });
                    return ToolOutput::error(format!(
                        "Blocked: a pre_tool hook failed ({msg}), so the call was not run."
                    ));
                }
            }
        }

        // Read-before-edit and stale-file guards (GRAFT-0028).
        let file = matches!(name, "read" | "edit" | "write")
            .then(|| input.get("path").and_then(Value::as_str))
            .flatten()
            .map(|p| {
                let joined = self.config.workspace.join(p);
                joined.canonicalize().unwrap_or(joined)
            });
        if let Some(path) = &file
            && matches!(name, "edit" | "write")
            && path.is_file()
        {
            let current = std::fs::metadata(path).and_then(|m| m.modified()).ok();
            match state.seen.get(path) {
                None => {
                    return ToolOutput::error(format!(
                        "Read `{}` before changing it, so your change is based on its current content.",
                        input["path"].as_str().unwrap_or("")
                    ));
                }
                Some(seen) if *seen != current => {
                    return ToolOutput::error(format!(
                        "`{}` changed on disk since you last read it. Read it again before changing it.",
                        input["path"].as_str().unwrap_or("")
                    ));
                }
                Some(_) => {}
            }
        }

        let action = tool.action(input);
        let protected = file.as_ref().is_some_and(|p| matches!(name, "edit" | "write") && self.is_protected(p));
        let fresh = self.toolkit.approval.needs_fresh_yes(&action);
        let decision = match self.toolkit.approval.decide(&action) {
            // Protected paths (GRAFT-0040) always need an explicit yes: no rule or
            // "always" answer can fast-path changes to agent config, CI or git internals.
            Decision::Allow | Decision::Ask if protected => Decision::Ask,
            // Risky commands (GRAFT-0042) are asked about every time.
            Decision::Ask if fresh.is_none() && always_covers(&state.always, name, &action.resource) => Decision::Allow,
            d => d,
        };
        match decision {
            Decision::Deny => {
                emit(&Event::ApprovalResolved { id: id.into(), decision: "deny".into() });
                let why = self.toolkit.approval.explain(&action).map(|w| format!(" Reason: {w}.")).unwrap_or_default();
                return ToolOutput::error(format!(
                    "Denied by the approval policy: `{name}` on `{}` is not allowed.{why} Choose another approach.",
                    action.resource
                ));
            }
            Decision::Ask => {
                emit(&Event::ApprovalRequested {
                    id: id.into(),
                    tool: name.into(),
                    resource: action.resource.clone(),
                    risk: fresh.clone(),
                });
                match self.approver.approve(name, &action, input) {
                    Answer::No => {
                        emit(&Event::ApprovalResolved { id: id.into(), decision: "ask_declined".into() });
                        return ToolOutput::error(
                            "The user declined this action. Do not retry it; choose another approach or explain what you need.",
                        );
                    }
                    Answer::NoWith(feedback) => {
                        emit(&Event::ApprovalResolved { id: id.into(), decision: "ask_declined".into() });
                        return ToolOutput::error(format!(
                            "The user declined this action and said: {feedback}\nFollow their guidance; do not retry the same action."
                        ));
                    }
                    Answer::Always => {
                        state.always.extend(always_keys(name, &action.resource));
                    }
                    Answer::Yes => {}
                }
                emit(&Event::ApprovalResolved { id: id.into(), decision: "allow".into() });
            }
            Decision::Allow => {}
        }

        let snapshots = Mutex::new(Vec::<FileSnapshot>::new());
        let ctx = ToolContext {
            workspace: &self.config.workspace,
            edit_format: edit,
            sandbox: self.toolkit.sandbox.as_ref(),
            snapshots: &snapshots,
            max_output_chars: self.config.max_output_chars,
            cancel: &self.config.cancel,
        };
        let mut output = catch_unwind(AssertUnwindSafe(|| tool.run(input, &ctx))).unwrap_or_else(|_| {
            ToolOutput::error(format!("internal error: the `{name}` tool panicked; the session continues"))
        });
        for hook in self.config.hooks.iter().filter(|h| h.matches(HookEvent::PostTool, name)) {
            match hook.run(
                &self.config.shell,
                &self.config.workspace,
                HookEvent::PostTool,
                name,
                &input_json,
                Some(&output.content),
            ) {
                HookResult::Ok | HookResult::Context(_) => {}
                HookResult::Feedback(feedback) => {
                    output.content.push_str(&format!("\n\nFeedback from a post_tool hook:\n{feedback}"));
                }
                HookResult::Failed(msg) => emit(&Event::Warning { message: msg }),
            }
        }
        if let Some(path) = file
            && !output.is_error
        {
            // After a read, or after our own write, the model's view matches the disk.
            let path = path.canonicalize().unwrap_or(path);
            let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            if let Some(extra) =
                crate::prompt::nested_instructions(&self.config.workspace, &path, &mut state.instructions)
            {
                output.content.push_str(&extra);
            }
            state.seen.insert(path, mtime);
        }
        for snap in snapshots.into_inner().unwrap_or_else(|e| e.into_inner()) {
            if let Err(e) = session.record_original(&snap.path, snap.original.as_deref()) {
                emit(&Event::Warning { message: format!("could not back up {} for undo: {e}", snap.path.display()) });
            }
        }
        output
    }

    fn save(&self, session: &Session, emit: &mut dyn FnMut(&Event)) {
        if let Err(e) = session.save() {
            emit(&Event::Warning { message: format!("could not save session: {e}") });
        }
    }

    fn finish(
        &self,
        session: &mut Session,
        emit: &mut dyn FnMut(&Event),
        status: Status,
        turns: u32,
        usage: Usage,
        final_text: String,
    ) -> Outcome {
        self.save(session, emit);
        for hook in self.config.hooks.iter().filter(|h| h.event == HookEvent::SessionEnd) {
            if let HookResult::Failed(msg) =
                hook.run(&self.config.shell, &self.config.workspace, HookEvent::SessionEnd, "", "{}", None)
            {
                emit(&Event::Warning { message: msg });
            }
        }
        // What the run changed (GRAFT-0063), for headless consumers and evaluations.
        if let Some(start) = session.run_start.take()
            && let (Some(first), Some(last)) = (session.checkpoints.get(start), session.checkpoints.last())
            && first.commit != last.commit
            && let Ok(cp) = crate::checkpoint::Checkpoints::open(&self.config.workspace)
            && let Ok(files) = cp.changed_files(&first.commit, &last.commit)
        {
            emit(&Event::FilesChanged { files });
        }
        let (ok, reason) = match &status {
            Status::Done => (true, None),
            Status::Stopped(r) | Status::Failed(r) => (false, Some(r.clone())),
        };
        emit(&Event::SessionFinished {
            ok,
            reason,
            turns,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
        });
        Outcome { status, turns, usage, final_text }
    }
}
