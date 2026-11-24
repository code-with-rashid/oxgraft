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
use crate::retry::{RetryPolicy, complete_with_retries};
use crate::session::Session;

/// Answer to an approval prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Yes,
    No,
    /// Yes, and don't ask again for this tool in this session.
    Always,
}

/// Asks the user about actions the approval policy marks `ask`.
pub trait Approver {
    fn approve(&self, tool: &str, action: &Action, input: &Value) -> Answer;
}

/// Headless approver: answers every `ask` the same way.
#[derive(Debug, Clone, Copy)]
pub struct Fixed(pub Answer);

impl Approver for Fixed {
    fn approve(&self, _: &str, _: &Action, _: &Value) -> Answer {
        self.0
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
const LOOP_LIMIT: usize = 3;
/// Tool calls executed per model turn; extra calls get an error result (stability).
const MAX_CALLS_PER_TURN: usize = 32;

pub struct Agent<'a> {
    pub toolkit: &'a Toolkit,
    pub provider: &'a dyn Provider,
    pub tools: Vec<Box<dyn Tool>>,
    pub approver: &'a dyn Approver,
    pub config: AgentConfig,
    /// Sleep used between retries; tests replace it.
    pub sleep: fn(Duration),
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
}

/// Tools a subagent may use: read-only, so its work never needs undo.
const SUBAGENT_TOOLS: &[&str] = &["read", "ls", "glob", "grep", "skill"];

impl<'a> Agent<'a> {
    pub fn new(
        toolkit: &'a Toolkit,
        provider: &'a dyn Provider,
        tools: Vec<Box<dyn Tool>>,
        approver: &'a dyn Approver,
        config: AgentConfig,
    ) -> Self {
        Self { toolkit, provider, tools, approver, config, sleep: std::thread::sleep }
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

    /// Run a read-only subagent (GRAFT-0026) and return its final report.
    fn subagent(&self, session: &mut Session, input: &Value) -> ToolOutput {
        let Some(prompt) = input.get("prompt").and_then(Value::as_str).filter(|p| !p.trim().is_empty()) else {
            return ToolOutput::error("missing required field `prompt`");
        };
        let mut sub = Session::new(&self.config.model, "subagent", &self.config.workspace);
        let label = input.get("description").and_then(Value::as_str).unwrap_or("subagent");
        sub.title = format!("[subagent] {label}");
        let task = format!(
            "{prompt}\n\nYou are a read-only subagent: investigate with the available tools, then reply with a concise, \
             specific report (file paths, line numbers, findings). Do not ask questions."
        );
        let outcome = self.run_at(&mut sub, &task, &mut |_| {}, 1);
        session.usage.add(outcome.usage);
        match outcome.status {
            Status::Done if !outcome.final_text.trim().is_empty() => ToolOutput::ok(outcome.final_text),
            Status::Done => ToolOutput::error("the subagent finished without a report"),
            Status::Stopped(r) | Status::Failed(r) => {
                ToolOutput::error(format!("the subagent stopped early ({r}). Partial report:\n{}", outcome.final_text))
            }
        }
    }

    pub fn system_prompt(&self) -> String {
        let mut p = crate::prompt::system_prompt(&self.config.workspace, &crate::prompt::today());
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
            model: Some(self.config.model.clone()),
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
        session.messages.push(Message::user(task));
        self.save(session, emit);

        let system = self.system_prompt();
        let specs = self.specs(depth);
        let mut state = RunState { always: HashSet::new(), recent: Vec::new(), depth, seen: HashMap::new() };
        let mut usage = Usage::default();
        let mut continuations = 0;
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
            let request = CompletionRequest {
                model: self.config.model.clone(),
                system: system.clone(),
                messages: session.messages.clone(),
                tools: specs.clone(),
                max_tokens: self.config.max_tokens,
            };
            let completion = complete_with_retries(
                self.provider,
                &request,
                &self.config.retry,
                &|d| (self.sleep)(d),
                &mut |attempt, wait, err| {
                    emit(&Event::Retrying { attempt, wait_ms: wait.as_millis() as u64, message: err.to_string() })
                },
            );
            let completion = match completion {
                Ok(c) => c,
                Err(e) => return self.finish(session, emit, Status::Failed(e.to_string()), turn, usage, final_text),
            };
            usage.add(completion.usage);
            session.usage.add(completion.usage);

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
                return self.finish(session, emit, Status::Done, turn, usage, final_text);
            }

            let truncated = completion.stop == StopReason::MaxTokens;
            let mut results = Vec::with_capacity(calls.len());
            for (index, (id, name, input)) in calls.into_iter().enumerate() {
                emit(&Event::ToolCall { id: id.clone(), tool: name.clone(), input: input.clone() });
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
                results.push(Block::ToolResult { tool_use_id: id, content: output.content, is_error: output.is_error });
            }
            session.messages.push(Message { role: Role::User, content: results });
            self.save(session, emit);

            let context_used = completion.usage.input_tokens + completion.usage.output_tokens;
            if let Some(window) = self.config.context_window
                && context_used > window * 8 / 10
            {
                self.compact(session, task, &system, &mut usage, emit);
            }
        }
        let turns = self.config.max_turns;
        self.finish(session, emit, Status::Stopped(format!("reached max turns ({turns})")), turns, usage, final_text)
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
    ) {
        let mut messages = session.messages.clone();
        messages.push(Message::user(
            "Context is nearly full. Write a handover summary for yourself so you can continue in a fresh context: \
             the task, what you have done (files changed and why), what you verified, the current state, open \
             problems, and the exact next steps. Do not call tools.",
        ));
        let request = CompletionRequest {
            model: self.config.model.clone(),
            system: system.to_string(),
            messages,
            tools: Vec::new(),
            max_tokens: self.config.max_tokens,
        };
        let summary =
            complete_with_retries(self.provider, &request, &self.config.retry, &|d| (self.sleep)(d), &mut |_, _, _| {})
                .map(|c| {
                    usage.add(c.usage);
                    session.usage.add(c.usage);
                    Message::assistant(c.content).text()
                });
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
            match hook.run(&self.config.workspace, HookEvent::PreTool, name, &input_json, None) {
                HookResult::Ok => {}
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
        let decision = match self.toolkit.approval.decide(&action) {
            Decision::Ask if state.always.contains(name) => Decision::Allow,
            d => d,
        };
        match decision {
            Decision::Deny => {
                emit(&Event::ApprovalResolved { id: id.into(), decision: "deny".into() });
                return ToolOutput::error(format!(
                    "Denied by the approval policy: `{name}` on `{}` is not allowed. Choose another approach.",
                    action.resource
                ));
            }
            Decision::Ask => {
                emit(&Event::ApprovalRequested { id: id.into(), tool: name.into(), resource: action.resource.clone() });
                match self.approver.approve(name, &action, input) {
                    Answer::No => {
                        emit(&Event::ApprovalResolved { id: id.into(), decision: "ask_declined".into() });
                        return ToolOutput::error(
                            "The user declined this action. Do not retry it; choose another approach or explain what you need.",
                        );
                    }
                    Answer::Always => {
                        state.always.insert(name.to_string());
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
            match hook.run(&self.config.workspace, HookEvent::PostTool, name, &input_json, Some(&output.content)) {
                HookResult::Ok => {}
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
            if let HookResult::Failed(msg) = hook.run(&self.config.workspace, HookEvent::SessionEnd, "", "{}", None) {
                emit(&Event::Warning { message: msg });
            }
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
