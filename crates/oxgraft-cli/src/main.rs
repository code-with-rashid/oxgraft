//! The `oxgraft` command (SPEC-0004).

mod commands;
mod config;
mod profiles;
mod render;

use std::io::{BufRead, IsTerminal, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oxgraft_core::{Agent, AgentConfig, Answer, Approver, Available, Fixed, Profile, Session, Status, Toolkit};
use oxgraft_protocol::Event;
use oxgraft_seams::Action;
use serde_json::Value;

/// Oxgraft: the best of every coding agent, grafted into one.
#[derive(Debug, Parser)]
#[command(name = "oxgraft", version, about, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    common: Common,
}

#[derive(Debug, Clone, clap::Args)]
struct Common {
    /// Model as provider/model (e.g. anthropic/claude-opus-5-5, openai/<model>, ollama/<model>) or mock:<script.json>.
    #[arg(long, short = 'm', global = true)]
    model: Option<String>,
    /// Profile name (default, baseline, ci, or profiles/<name>.toml) or path to a .toml file.
    #[arg(long, short = 'p', global = true)]
    profile: Option<String>,
    /// Anthropic effort level: low, medium, high, xhigh, max (or none).
    #[arg(long, global = true)]
    effort: Option<String>,
    /// Maximum model turns per task.
    #[arg(long, global = true)]
    max_turns: Option<u32>,
    /// Approve every `ask` action without prompting (deny rules still apply).
    #[arg(long, short = 'y', global = true)]
    yes: bool,
    /// Approval mode: read-only, suggest (default), auto-edit, full-auto.
    #[arg(long, global = true)]
    mode: Option<String>,
    /// Extra instructions added to the system prompt for this run.
    #[arg(long, global = true, value_name = "TEXT")]
    append_system_prompt: Option<String>,
    /// Workspace directory (default: current directory).
    #[arg(long, global = true)]
    cwd: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run one task and exit.
    Run {
        /// The task. Reads stdin if omitted.
        task: Vec<String>,
        /// Stream protocol events as JSON lines on stdout.
        #[arg(long)]
        json: bool,
        /// Continue a saved session (id from `oxgraft sessions`, or `last`).
        #[arg(long)]
        resume: Option<String>,
        /// Don't accept "done" until this command passes (e.g. "cargo test"); its
        /// failures go back to the model, up to 3 times.
        #[arg(long)]
        until: Option<String>,
        /// Start a new session from a copy of another's history: `ID`, or `ID@N` to
        /// keep its first N messages (N must follow a finished reply).
        #[arg(long, conflicts_with = "resume")]
        fork: Option<String>,
    },
    /// Interactive session (the default when no command is given).
    Chat {
        /// Continue a saved session (id or `last`).
        #[arg(long)]
        resume: Option<String>,
        /// Start a new session from a copy of another's history: `ID`, or `ID@N` to
        /// keep its first N messages (N must follow a finished reply).
        #[arg(long, conflicts_with = "resume")]
        fork: Option<String>,
    },
    /// List saved sessions in this workspace.
    Sessions,
    /// Restore files changed by a session (default: the latest).
    Undo { id: Option<String> },
    /// Write a session as a Markdown transcript (default: the latest session).
    Export {
        id: Option<String>,
        /// Write to this file instead of stdout.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// List a session's workspace checkpoints (default: the latest session).
    Checkpoints { id: Option<String> },
    /// Put the workspace back to checkpoint N of a session (see `oxgraft checkpoints`).
    Restore {
        /// Checkpoint number; 0 is the state when the session started.
        number: usize,
        #[arg(long, default_value = "last")]
        session: String,
    },
    /// Check the setup: model and credentials, shell, sandbox, git, skills, MCP, config.
    Doctor {
        /// Also send two tiny requests to the model: a plain reply (credentials,
        /// streaming) and a tool call (whether the model can use tools at all).
        #[arg(long)]
        live: bool,
    },
    /// Have the agent study this repository and write (or improve) AGENTS.md.
    Init {
        #[arg(long)]
        json: bool,
    },
    /// Show supported providers and how to configure them.
    Models,
    /// List every strategy and where it was learned from.
    Strategies {
        #[arg(long)]
        json: bool,
    },
    /// Validate a profile and print the session preamble it would produce.
    Profile {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Run a tiny MCP server over stdio (for tests and MCP setup checks).
    #[command(name = "__mcp-fixture", hide = true)]
    McpFixture,
}

const INIT_TASK: &str = "Study this repository and write AGENTS.md at its root: concise instructions for coding \
agents working here. Cover: what the project is (2-3 sentences); the layout (key directories and what lives in them); \
exact commands to install, build, test, lint and run (verify each one you can by running it); code conventions you can \
see in the code; and anything surprising a newcomer would get wrong. If AGENTS.md already exists, read it first and \
improve it rather than replacing good content. Keep it under 150 lines; no generic advice.";

/// Providers that run on the user's machine, with small default context windows.
const LOCAL_PROVIDERS: &[&str] = &["ollama", "lmstudio"];
/// Compaction window assumed for local providers unless `context_window` is set.
const LOCAL_CONTEXT_WINDOW: u64 = 32_768;

/// Exit codes (SPEC-0004 AC2).
const EXIT_ERROR: u8 = 1;
const EXIT_USAGE: u8 = 2;
const EXIT_STOPPED: u8 = 3;

fn available() -> Available {
    available_with(Vec::new())
}

/// Rules for an approval mode (GRAFT-0052, after Codex's suggest / auto-edit /
/// full-auto and read-only modes). They sit between the built-in defaults and the
/// user's own rules, so explicit rules still win. Protected paths and risk rails
/// apply in every mode.
fn mode_rules(mode: &str) -> Result<Vec<oxgraft_strategies::approval::Rule>, String> {
    use oxgraft_seams::Decision::{Allow, Deny};
    use oxgraft_strategies::approval::Rule;
    let why = "read-only mode: describe the change instead, or rerun with --mode suggest";
    Ok(match mode {
        "suggest" => Vec::new(),
        "read-only" => {
            ["edit", "write", "bash", "mcp__*"].iter().map(|t| Rule::new(t, "*", Deny).because(why)).collect()
        }
        "auto-edit" => vec![Rule::new("edit", "*", Allow), Rule::new("write", "*", Allow)],
        "full-auto" => {
            vec![Rule::new("edit", "*", Allow), Rule::new("write", "*", Allow), Rule::new("bash", "*", Allow)]
        }
        other => {
            return Err(format!("unknown approval mode `{other}`: use read-only, suggest, auto-edit or full-auto"));
        }
    })
}

/// Strategies, with user-configured approval rules appended to the defaults.
fn available_with(rules: Vec<oxgraft_strategies::approval::Rule>) -> Available {
    Available {
        edit_formats: oxgraft_strategies::edit_formats(),
        contexts: oxgraft_strategies::contexts(),
        approvals: vec![
            Box::new(oxgraft_strategies::approval::OrderedRules::with_extra(rules.clone())),
            Box::new(oxgraft_strategies::approval::InertReads::new(
                oxgraft_strategies::approval::OrderedRules::with_extra(rules),
            )),
            Box::new(oxgraft_strategies::approval::AllowAll),
        ],
        sandboxes: oxgraft_strategies::sandboxes(),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(code) => ExitCode::from(code),
        Err((code, message)) => {
            eprintln!("oxgraft: {message}");
            ExitCode::from(code)
        }
    }
}

type CliResult = Result<u8, (u8, String)>;

fn usage(e: impl std::fmt::Display) -> (u8, String) {
    (EXIT_USAGE, e.to_string())
}

fn workspace(common: &Common) -> Result<PathBuf, (u8, String)> {
    let dir = match &common.cwd {
        Some(d) => d.clone(),
        None => std::env::current_dir().map_err(usage)?,
    };
    dir.canonicalize().map_err(|e| usage(format!("workspace {}: {e}", dir.display())))
}

fn dispatch(cli: Cli) -> CliResult {
    let common = cli.common.clone();
    match cli.command {
        None => chat(&common, None, None),
        Some(Command::Chat { resume, fork }) => chat(&common, resume, fork),
        Some(Command::Run { task, json, resume, fork, until }) => run(&common, task, json, resume, fork, until),
        Some(Command::Sessions) => {
            let ws = workspace(&common)?;
            let sessions = Session::list(&ws);
            if sessions.is_empty() {
                println!("no sessions in {}", ws.display());
            }
            for s in sessions {
                println!(
                    "{}  {:<28} {:>3} msgs  {:>6} files changed  {}",
                    s.id,
                    s.model,
                    s.messages.len(),
                    s.changed_files.len(),
                    s.title
                );
            }
            Ok(0)
        }
        Some(Command::Undo { id }) => {
            let ws = workspace(&common)?;
            let session = load_session(&ws, id.as_deref().unwrap_or("last"))?;
            if session.changed_files.is_empty() {
                println!("session {} changed no files", session.id);
            }
            for line in session.undo() {
                println!("{line}");
            }
            Ok(0)
        }
        Some(Command::Export { id, output }) => {
            let ws = workspace(&common)?;
            let session = load_session(&ws, id.as_deref().unwrap_or("last"))?;
            let text = session.to_markdown();
            match output {
                Some(path) => {
                    std::fs::write(&path, text).map_err(|e| (EXIT_ERROR, format!("{}: {e}", path.display())))?;
                    eprintln!("wrote {}", path.display());
                }
                None => print!("{text}"),
            }
            Ok(0)
        }
        Some(Command::Checkpoints { id }) => {
            let ws = workspace(&common)?;
            let session = load_session(&ws, id.as_deref().unwrap_or("last"))?;
            for line in checkpoint_lines(&ws, &session).map_err(|e| (EXIT_ERROR, e))? {
                println!("{line}");
            }
            Ok(0)
        }
        Some(Command::Restore { number, session }) => {
            let ws = workspace(&common)?;
            let mut session = load_session(&ws, &session)?;
            let message = restore_checkpoint(&ws, &mut session, number)?;
            println!("{message}");
            Ok(0)
        }
        Some(Command::McpFixture) => {
            oxgraft_tools::mcp::serve_fixture();
            Ok(0)
        }
        Some(Command::Doctor { live }) => doctor(&common, live),
        // GRAFT-0032: Claude Code's and Codex's /init, as a preset task through the normal loop.
        Some(Command::Init { json }) => run(&common, vec![INIT_TASK.to_string()], json, None, None, None),
        Some(Command::Models) => {
            render::models();
            Ok(0)
        }
        Some(Command::Strategies { json }) => {
            render::strategies(json).map_err(|e| (EXIT_ERROR, e))?;
            Ok(0)
        }
        Some(Command::Profile { path, json }) => {
            let text = std::fs::read_to_string(&path).map_err(|e| usage(format!("{}: {e}", path.display())))?;
            let profile = Profile::from_toml(&text).map_err(|e| usage(format!("{}: {e}", path.display())))?;
            let toolkit = Toolkit::assemble(profile, available()).map_err(usage)?;
            if json {
                for event in toolkit.session_preamble() {
                    println!("{}", event.to_jsonl());
                }
            } else {
                println!("profile ok: {toolkit:?}");
            }
            Ok(0)
        }
    }
}

fn load_session(ws: &Path, id: &str) -> Result<Session, (u8, String)> {
    if id == "last" {
        return Session::list(ws).into_iter().next().ok_or_else(|| usage("no saved sessions in this workspace"));
    }
    Session::load(ws, id).map_err(usage)
}

struct Setup {
    ws: PathBuf,
    toolkit: Toolkit,
    provider: Box<dyn oxgraft_seams::Provider>,
    model: String,
    config: AgentConfig,
    tools: Vec<Box<dyn oxgraft_seams::Tool>>,
}

fn setup(common: &Common) -> Result<Setup, (u8, String)> {
    let ws = workspace(common)?;
    let cfg = config::resolve(
        &ws,
        &config::Flags {
            model: common.model.as_deref(),
            profile: common.profile.as_deref(),
            effort: common.effort.as_deref(),
            max_turns: common.max_turns,
            approval_mode: common.mode.as_deref(),
            instructions: common.append_system_prompt.as_deref(),
        },
    )
    .map_err(usage)?;
    for warning in &cfg.warnings {
        eprintln!("oxgraft: warning: {warning}");
    }
    let model_spec = cfg.model.clone().ok_or_else(|| {
        usage(
            "no model configured. Set ANTHROPIC_API_KEY to use anthropic/claude-opus-5-5, or pass --model \
             provider/model (see `oxgraft models`), or set `model` in .oxgraft/config.toml",
        )
    })?;
    let model_ref = oxgraft_providers::parse_model(&model_spec).map_err(usage)?;
    let provider = oxgraft_providers::build(&model_ref, &oxgraft_providers::Options { effort: cfg.effort.clone() })
        .map_err(usage)?;
    let (_, profile_text) =
        profiles::load(&ws, &profiles::resolve_auto(&cfg.profile, &model_ref.model), cfg.trusted).map_err(usage)?;
    let profile = Profile::from_toml(&profile_text).map_err(usage)?;
    let mut rules = mode_rules(&cfg.approval_mode).map_err(usage)?;
    rules.extend(cfg.rules.clone());
    let toolkit = Toolkit::assemble(profile, available_with(rules)).map_err(usage)?;
    let mut config = AgentConfig::new(&model_ref.model, ws.clone());
    config.max_turns = cfg.max_turns;
    config.max_tokens = cfg.max_tokens;
    config.max_session_tokens = cfg.max_session_tokens;
    config.context_window = cfg.context_window;
    // Local servers usually run far smaller windows than hosted models; compact
    // early rather than overflow (GRAFT-0064). An explicit setting always wins.
    if cfg.context_window_default && LOCAL_PROVIDERS.contains(&model_ref.provider.as_str()) {
        config.context_window = Some(LOCAL_CONTEXT_WINDOW);
    }
    config.verify_command = cfg.verify_command.clone();
    config.stream = cfg.stream;
    config.checkpoints = cfg.checkpoints;
    let (program, args) = oxgraft_tools::shell_program();
    config.shell = oxgraft_core::hooks::Shell { program, args };
    config.hooks = cfg.hooks.clone();
    let (mut tools, skills) = oxgraft_tools::for_workspace(&ws);
    for (name, server) in &cfg.mcp_servers {
        match oxgraft_tools::mcp::connect(name, server, &ws) {
            Ok(found) => tools.extend(found),
            // One broken server must not stop the session (stability over completeness).
            Err(e) => eprintln!("oxgraft: warning: {e}"),
        }
    }
    // Standing instructions (GRAFT-0067) go before the skills list.
    let mut extra: Vec<String> = cfg.instructions.clone();
    if !extra.is_empty() {
        extra.insert(0, "Additional instructions from the user's configuration:".into());
    }
    extra.extend(skills);
    config.append_system = if extra.is_empty() { None } else { Some(extra.join("\n\n")) };
    // First Ctrl-C interrupts the current run (tools and model turns stop promptly);
    // a second one exits (GRAFT-0020).
    let cancel = config.cancel.clone();
    let _ = ctrlc::set_handler(move || {
        if cancel.swap(true, std::sync::atomic::Ordering::SeqCst) {
            eprintln!("\noxgraft: exiting");
            std::process::exit(130);
        }
        eprintln!("\noxgraft: interrupting… (press Ctrl-C again to quit)");
    });
    Ok(Setup { ws, toolkit, provider, model: model_spec, config, tools })
}

fn status_code(status: &Status) -> u8 {
    match status {
        Status::Done => 0,
        Status::Stopped(_) => EXIT_STOPPED,
        Status::Failed(_) => EXIT_ERROR,
    }
}

/// The session to work in: resumed, forked (GRAFT-0049), or new.
fn open_session(s: &Setup, resume: Option<String>, fork: Option<String>) -> Result<Session, (u8, String)> {
    if let Some(spec) = fork {
        let (id, keep) = match spec.rsplit_once('@') {
            Some((id, n)) => {
                (id.to_string(), Some(n.parse::<usize>().map_err(|_| usage(format!("bad fork point `{n}`")))?))
            }
            None => (spec, None),
        };
        let source = load_session(&s.ws, &id)?;
        let fork = source.fork(keep).map_err(usage)?;
        eprintln!("forked session {} from {} ({} messages)", fork.id, source.id, fork.messages.len());
        return Ok(fork);
    }
    match resume {
        Some(id) => load_session(&s.ws, &id),
        None => Ok(Session::new(&s.model, &s.toolkit.profile.name, &s.ws)),
    }
}

fn run(
    common: &Common,
    task: Vec<String>,
    json: bool,
    resume: Option<String>,
    fork: Option<String>,
    until: Option<String>,
) -> CliResult {
    let mut task = task.join(" ");
    if task.trim().is_empty() {
        if std::io::stdin().is_terminal() {
            return Err(usage("no task given: oxgraft run \"<task>\" (or pipe it on stdin)"));
        }
        std::io::stdin().lock().read_line(&mut task).map_err(usage)?;
        let mut rest = String::new();
        while std::io::stdin().lock().read_line(&mut rest).map_err(usage)? > 0 {
            task.push_str(&rest);
            rest.clear();
        }
    }
    if task.trim().is_empty() {
        return Err(usage("the task is empty"));
    }
    if task.trim_start().starts_with('/') {
        let ws = workspace(common)?;
        match commands::expand(&ws, &task) {
            Some(expanded) => task = expanded,
            None if !task.trim().contains(char::is_whitespace) => {
                let names: Vec<String> = commands::discover(&ws).into_iter().map(|c| format!("/{}", c.name)).collect();
                return Err(usage(format!(
                    "unknown command `{}` (available: {})",
                    task.trim(),
                    if names.is_empty() { "none; add .agents/commands/<name>.md".into() } else { names.join(", ") }
                )));
            }
            None => {}
        }
    }
    let mut s = setup(common)?;
    s.config.finish_check = until;
    let mut session = open_session(&s, resume, fork)?;
    let approver: Box<dyn Approver> = if common.yes || json || !std::io::stdin().is_terminal() {
        Box::new(Fixed(if common.yes { Answer::Yes } else { Answer::No }))
    } else {
        Box::new(TerminalApprover)
    };
    let agent = Agent::new(&s.toolkit, s.provider.as_ref(), s.tools, approver.as_ref(), s.config);
    let mut renderer = render::Human::default();
    let before = session.checkpoints.len();
    let outcome = agent.run(&mut session, task.trim(), &mut |e: &Event| {
        if json {
            println!("{}", e.to_jsonl());
            let _ = std::io::stdout().flush();
        } else {
            renderer.event(e);
        }
    });
    if !json {
        change_summary(&s.ws, &session, before);
        eprintln!("session {} · resume with: oxgraft run --resume {} \"...\"", session.id, session.id);
    }
    Ok(status_code(&outcome.status))
}

/// Run a command the user typed with `!` in chat (GRAFT-0058): the user's own
/// action, so no approval; same shell as the bash tool; 5-minute limit.
fn run_user_command(ws: &Path, command: &str) -> (bool, String) {
    let (program, args) = oxgraft_tools::shell_program();
    oxgraft_core::hooks::run_check(&oxgraft_core::hooks::Shell { program, args }, command, ws, 300)
}

/// Lines describing a session's checkpoints (GRAFT-0046), for the CLI and chat.
fn checkpoint_lines(ws: &Path, session: &Session) -> Result<Vec<String>, String> {
    if session.checkpoints.is_empty() {
        return Ok(vec![format!("session {} has no checkpoints", session.id)]);
    }
    let cp = oxgraft_core::checkpoint::Checkpoints::open(ws)?;
    let mut out = vec![format!(
        "checkpoints of session {} (restore with: oxgraft restore N --session {}):",
        session.id, session.id
    )];
    let mut previous: Option<&str> = None;
    for (n, c) in session.checkpoints.iter().enumerate() {
        let stat = previous
            .and_then(|p| cp.diff_stat(p, &c.commit).ok())
            .and_then(|s| s.lines().last().map(|l| l.trim().to_string()))
            .unwrap_or_default();
        out.push(format!("  {n:>3}  {:<10} {}  {stat}", c.label, &c.commit[..c.commit.len().min(10)]));
        previous = Some(&c.commit);
    }
    Ok(out)
}

/// Put the workspace back to checkpoint `number`, recording a pre-restore
/// checkpoint so the restore itself can be undone. Returns a message for the user.
fn restore_checkpoint(ws: &Path, session: &mut Session, number: usize) -> Result<String, (u8, String)> {
    let Some(target) = session.checkpoints.get(number).cloned() else {
        return Err(usage(format!(
            "session {} has {} checkpoint(s); run `oxgraft checkpoints {}`",
            session.id,
            session.checkpoints.len(),
            session.id
        )));
    };
    let cp = oxgraft_core::checkpoint::Checkpoints::open(ws).map_err(|e| (EXIT_ERROR, e))?;
    let safety = cp.restore(&target.commit).map_err(|e| (EXIT_ERROR, e))?;
    session.checkpoints.push(oxgraft_core::session::CheckpointRecord { label: "pre-restore".into(), commit: safety });
    let _ = session.save();
    Ok(format!(
        "workspace restored to checkpoint {number} ({}) of session {}; undo with: oxgraft restore {} --session {}",
        target.label,
        session.id,
        session.checkpoints.len() - 1,
        session.id
    ))
}

/// After a run, list what changed in the workspace and how to roll it back
/// (GRAFT-0048, after SWE-agent's review-on-submit). Uses the run's checkpoints.
fn change_summary(ws: &Path, session: &Session, before: usize) {
    let all = &session.checkpoints;
    if all.len() <= before {
        return; // no new checkpoint: nothing changed (or checkpoints are off)
    }
    // The run's starting state: its own "start" checkpoint, or the previous last one.
    let start = if all[before].label == "start" { before } else { before.saturating_sub(1) };
    let last = all.len() - 1;
    if start >= last {
        return;
    }
    let Ok(cp) = oxgraft_core::checkpoint::Checkpoints::open(ws) else { return };
    let Ok(stat) = cp.diff_stat(&all[start].commit, &all[last].commit) else { return };
    if stat.trim().is_empty() {
        return;
    }
    eprintln!("\nchanged in this run:");
    for line in stat.lines() {
        eprintln!("  {}", line.trim());
    }
    eprintln!("undo all of it with: oxgraft restore {start} --session {}", session.id);
}

fn chat(common: &Common, resume: Option<String>, fork: Option<String>) -> CliResult {
    let s = setup(common)?;
    let mut session = open_session(&s, resume, fork)?;
    let approver: Box<dyn Approver> =
        if common.yes { Box::new(Fixed(Answer::Yes)) } else { Box::new(TerminalApprover) };
    let agent = Agent::new(&s.toolkit, s.provider.as_ref(), s.tools, approver.as_ref(), s.config);
    eprintln!(
        "oxgraft · {} · profile {} · {}\nType a task, or /help for commands.",
        s.model,
        s.toolkit.profile.name,
        s.ws.display()
    );
    let stdin = std::io::stdin();
    let mut last = 0;
    // Output of `!command` lines, passed to the model with the next message.
    let mut shell_context = String::new();
    loop {
        eprint!("\n> ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).map_err(usage)? == 0 {
            break;
        }
        let line = line.trim();
        match line {
            "" => continue,
            bang if bang.starts_with('!') => {
                let command = bang[1..].trim();
                if command.is_empty() {
                    eprintln!(
                        "usage: !<command> (runs it in the workspace; the output goes to the model with your next message)"
                    );
                    continue;
                }
                let (ok, output) = run_user_command(&s.ws, command);
                eprintln!("{output}");
                if !ok {
                    eprintln!("(command failed)");
                }
                let tail: String = {
                    let chars: Vec<char> = output.chars().collect();
                    chars[chars.len().saturating_sub(8_000)..].iter().collect()
                };
                shell_context.push_str(&format!("[I ran `{command}` myself; its output:]\n{tail}\n\n"));
                continue;
            }
            "/exit" | "/quit" => break,
            "/undo" => {
                for l in session.undo() {
                    eprintln!("{l}");
                }
                session.changed_files.clear();
                let _ = session.save();
                continue;
            }
            "/sessions" => {
                for s in Session::list(&session.workspace) {
                    eprintln!("{}  {}", s.id, s.title);
                }
                continue;
            }
            "/cost" | "/usage" => {
                eprintln!(
                    "session {}: {} input + {} output tokens, {} messages, {} files changed, {} compactions",
                    session.id,
                    session.usage.input_tokens,
                    session.usage.output_tokens,
                    session.messages.len(),
                    session.changed_files.len(),
                    session.compactions
                );
                continue;
            }
            "/checkpoints" => {
                match checkpoint_lines(&s.ws, &session) {
                    Ok(lines) => lines.iter().for_each(|l| eprintln!("{l}")),
                    Err(e) => eprintln!("checkpoints: {e}"),
                }
                continue;
            }
            restore if restore.starts_with("/restore") => {
                match restore.trim_start_matches("/restore").trim().parse::<usize>() {
                    Ok(n) => match restore_checkpoint(&s.ws, &mut session, n) {
                        Ok(message) => eprintln!("{message}"),
                        Err((_, e)) => eprintln!("restore: {e}"),
                    },
                    Err(_) => eprintln!("usage: /restore N (see /checkpoints)"),
                }
                continue;
            }
            "/clear" | "/new" => {
                session = Session::new(&s.model, &s.toolkit.profile.name, &s.ws);
                eprintln!("started a new session {}", session.id);
                continue;
            }
            "/help" => {
                eprintln!(
                    "/undo      restore files changed in this session\n/checkpoints  list workspace checkpoints\n\
                     /restore N  put the workspace back to checkpoint N\n/cost      token usage for this session\n\
                     /clear     start a new session (the old one stays saved)\n/sessions  list saved sessions\n\
                     !cmd       run a shell command yourself; output goes with your next message\n\
                     /exit      quit\nCtrl-C interrupts a running task; press it twice to quit."
                );
                for c in commands::discover(&s.ws) {
                    eprintln!("/{:<10} {}", c.name, c.description);
                }
                continue;
            }
            other if other.starts_with('/') && commands::expand(&s.ws, other).is_none() && !other.contains(' ') => {
                eprintln!("unknown command {other}; try /help");
                continue;
            }
            _ => {}
        }
        agent.config.cancel.store(false, std::sync::atomic::Ordering::SeqCst);
        let mut renderer = render::Human::default();
        let before = session.checkpoints.len();
        // Custom commands (GRAFT-0057): `/name args` expands to the command's prompt.
        let task = commands::expand(&s.ws, line).unwrap_or_else(|| line.to_string());
        let task = format!("{}{task}", std::mem::take(&mut shell_context));
        let outcome = agent.run(&mut session, &task, &mut |e: &Event| renderer.event(e));
        change_summary(&s.ws, &session, before);
        last = status_code(&outcome.status);
    }
    eprintln!("session {} saved", session.id);
    Ok(last)
}

/// Prompts on the terminal for `ask` decisions.
struct TerminalApprover;

impl Approver for TerminalApprover {
    fn approve(&self, tool: &str, action: &Action, input: &Value) -> Answer {
        let detail = match tool {
            "write" => format!("{} ({} bytes)", action.resource, input["content"].as_str().map_or(0, str::len)),
            _ => action.resource.clone(),
        };
        // Show what will change before asking (GRAFT-0029, as Cline and Claude Code do).
        let preview = match tool {
            "edit" => input["edit"].as_str().or(input["new_string"].as_str()),
            "write" => input["content"].as_str(),
            _ => None,
        };
        if let Some(text) = preview {
            let lines: Vec<&str> = text.lines().collect();
            for line in lines.iter().take(40) {
                eprintln!("    │ {line}");
            }
            if lines.len() > 40 {
                eprintln!("    │ … {} more lines", lines.len() - 40);
            }
        }
        // Risk rails (GRAFT-0042): say why, and don't offer "always".
        let risk = if tool == "bash" { oxgraft_strategies::approval::shell_risk(&action.resource) } else { None };
        if let Some(why) = risk {
            eprintln!("  ! risky: {why}. This is asked every time.");
        }
        loop {
            if risk.is_some() {
                eprint!("  allow {tool}: {detail}? [y]es / [n]o: ");
            } else {
                let scope = match tool {
                    "bash" => {
                        let keys = oxgraft_core::agent::always_keys(tool, &action.resource);
                        if keys.is_empty() {
                            "this exact command".to_string()
                        } else {
                            keys.iter()
                                .map(|k| format!("`{}`", k.trim_start_matches("bash:")))
                                .collect::<Vec<_>>()
                                .join(" + ")
                        }
                    }
                    _ => format!("all {tool}"),
                };
                eprint!("  allow {tool}: {detail}? [y]es / [n]o / [a]lways for {scope}: ");
            }
            let _ = std::io::stderr().flush();
            let mut answer = String::new();
            if std::io::stdin().lock().read_line(&mut answer).unwrap_or(0) == 0 {
                return Answer::No;
            }
            match answer.trim().to_ascii_lowercase().as_str() {
                "y" | "yes" => return Answer::Yes,
                "n" | "no" | "" => return Answer::No,
                "a" | "always" => return Answer::Always,
                _ => continue,
            }
        }
    }
}

/// `doctor --live`: one plain streamed reply, then one forced tool call. Returns
/// (ok, check, detail) lines. Costs a few hundred tokens.
fn live_checks(provider: &dyn oxgraft_seams::Provider, model: &str) -> Vec<(bool, &'static str, String)> {
    use oxgraft_seams::{Block, CompletionRequest, Message, ToolSpec};
    let mut out = Vec::new();
    let plain = CompletionRequest {
        model: model.to_string(),
        system: "You are a connectivity check. Answer with exactly one word.".into(),
        messages: vec![Message::user("Reply with the word: ready")],
        tools: Vec::new(),
        max_tokens: 1024,
    };
    let start = std::time::Instant::now();
    let mut deltas = 0usize;
    match provider.stream(&plain, &mut |_| deltas += 1) {
        Ok(c) => {
            let text = Message::assistant(c.content).text();
            let shown: String = text.trim().chars().take(60).collect();
            out.push((
                true,
                "live reply",
                format!(
                    "{:.1}s, {deltas} streamed chunk(s), {} in / {} out tokens: {shown:?}",
                    start.elapsed().as_secs_f64(),
                    c.usage.input_tokens,
                    c.usage.output_tokens
                ),
            ));
        }
        Err(e) => {
            out.push((false, "live reply", format!("{} (try OXGRAFT_STREAM=0 if only streaming fails)", e)));
            return out;
        }
    }
    let tools = CompletionRequest {
        model: model.to_string(),
        system: "You are a connectivity check. Use the tool you are given.".into(),
        messages: vec![Message::user("Call the `echo` tool with text \"ping\". Do not answer in prose.")],
        tools: vec![ToolSpec {
            name: "echo".into(),
            description: "Echo text back.".into(),
            input_schema: serde_json::json!({"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}),
        }],
        max_tokens: 1024,
    };
    match provider.complete(&tools) {
        Ok(c) => {
            let call = c.content.iter().find_map(|b| match b {
                Block::ToolUse { name, input, .. } => Some((name.clone(), input.clone())),
                _ => None,
            });
            match call {
                Some((name, input)) if name == "echo" && input.is_object() => {
                    out.push((true, "live tools", format!("tool call works: echo {input}")))
                }
                Some((name, input)) => out.push((false, "live tools", format!("malformed tool call: {name} {input}"))),
                None => out.push((
                    false,
                    "live tools",
                    "the model answered without calling the tool; it may not support tool calling, which Oxgraft needs"
                        .into(),
                )),
            }
        }
        Err(e) => out.push((false, "live tools", e.to_string())),
    }
    out
}

/// `oxgraft doctor` (GRAFT-0031, after Claude Code's /doctor): check everything a
/// session depends on and say what to fix. Exit 0 if a session can start.
fn doctor(common: &Common, live: bool) -> CliResult {
    let ws = workspace(common)?;
    let mut problems = 0;
    let mut line = |ok: bool, what: &str, detail: String| {
        if !ok {
            problems += 1;
        }
        println!("{} {what:<14} {detail}", if ok { "ok  " } else { "FAIL" });
    };
    println!("oxgraft {} · workspace {}", env!("CARGO_PKG_VERSION"), ws.display());
    let flags = config::Flags {
        model: common.model.as_deref(),
        profile: common.profile.as_deref(),
        effort: common.effort.as_deref(),
        max_turns: common.max_turns,
        approval_mode: common.mode.as_deref(),
        instructions: common.append_system_prompt.as_deref(),
    };
    let cfg = match config::resolve(&ws, &flags) {
        Ok(c) => {
            let sources = if c.sources.is_empty() { "defaults only".to_string() } else { c.sources.join(", ") };
            line(true, "config", sources);
            for w in &c.warnings {
                println!("warn {:<14} {w}", "config");
            }
            Some(c)
        }
        Err(e) => {
            line(false, "config", e);
            None
        }
    };
    if let Some(cfg) = &cfg {
        match &cfg.model {
            None => {
                line(false, "model", "none configured: set ANTHROPIC_API_KEY or pass --model provider/model".into())
            }
            Some(spec) => match oxgraft_providers::parse_model(spec)
                .and_then(|m| oxgraft_providers::build(&m, &oxgraft_providers::Options::default()).map(|_| m))
            {
                Ok(m) => {
                    line(true, "model", format!("{spec} (credentials found for `{}`)", m.provider));
                    if m.provider == "ollama" {
                        println!(
                            "warn {:<14} Ollama's default context is small and it silently drops the start of long \
                             prompts. Start the server with OLLAMA_CONTEXT_LENGTH=32768 (or more) and pick a model \
                             that supports tool calling; `oxgraft doctor --live` checks tools.",
                            "ollama"
                        );
                    }
                    if live {
                        let options = oxgraft_providers::Options { effort: cfg.effort.clone() };
                        match oxgraft_providers::build(&m, &options) {
                            Ok(provider) => {
                                for (ok, what, detail) in live_checks(provider.as_ref(), &m.model) {
                                    line(ok, what, detail);
                                }
                            }
                            Err(e) => line(false, "live", e.message),
                        }
                    }
                }
                Err(e) => line(false, "model", format!("{spec}: {}", e.message)),
            },
        }
        match profiles::load(
            &ws,
            &profiles::resolve_auto(&cfg.profile, cfg.model.as_deref().unwrap_or("")),
            cfg.trusted,
        )
        .and_then(|(src, t)| {
            Profile::from_toml(&t)
                .map_err(|e| e.to_string())
                .and_then(|p| Toolkit::assemble(p, available()).map_err(|e| e.to_string()))
                .map(|tk| (src, tk))
        }) {
            Ok((src, tk)) => {
                line(true, "profile", format!("{} ({src}), sandbox `{}`", tk.profile.name, tk.sandbox.info().id));
                if tk.sandbox.info().id != "none" {
                    match oxgraft_tools::sandbox_check(&ws) {
                        Ok(()) => line(true, "sandbox", "OS sandbox works here".into()),
                        Err(e) => line(false, "sandbox", format!("{e} (shell commands will be refused)")),
                    }
                }
            }
            Err(e) => line(false, "profile", e),
        }
        for (name, server) in &cfg.mcp_servers {
            match oxgraft_tools::mcp::connect(name, server, &ws) {
                Ok(tools) => line(true, "mcp", format!("{name}: {} tools", tools.len())),
                Err(e) => line(false, "mcp", e),
            }
        }
    }
    let shell = match oxgraft_tools::shell_kind() {
        oxgraft_tools::ShellKind::Posix(p) => format!("POSIX sh ({})", p.display()),
        oxgraft_tools::ShellKind::PowerShell => "PowerShell (install Git for Windows for a POSIX sh)".into(),
    };
    line(true, "shell", shell);
    let git = oxgraft_core::prompt::git_summary(&ws)
        .map(|s| s.lines().next().unwrap_or("").trim().trim_start_matches("- ").to_string());
    line(true, "git", git.unwrap_or_else(|| "not a git repository (undo still works)".into()));
    let instructions: Vec<&str> =
        oxgraft_core::prompt::INSTRUCTION_FILES.iter().copied().filter(|f| ws.join(f).is_file()).collect();
    line(
        true,
        "instructions",
        if instructions.is_empty() { "none (AGENTS.md recommended)".into() } else { instructions.join(", ") },
    );
    let skills = oxgraft_tools::discover_skills(&ws);
    line(true, "skills", format!("{} found", skills.len()));
    if let Some(cfg) = &cfg {
        line(
            true,
            "trust",
            if cfg.trusted {
                "trusted: this repo's hooks, MCP servers, rules, model and profile apply".into()
            } else {
                "not trusted: this repo's hooks, MCP servers, rules, model and profile are ignored".into()
            },
        );
        match mode_rules(&cfg.approval_mode) {
            Ok(_) => line(true, "approvals", format!("mode `{}` (risky commands always ask)", cfg.approval_mode)),
            Err(e) => line(false, "approvals", e),
        }
        let checkpoints = if !cfg.checkpoints {
            "off (`checkpoints = false`)".to_string()
        } else {
            match oxgraft_core::checkpoint::Checkpoints::check(&ws) {
                Ok(()) => format!(
                    "on: snapshots after changing turns in {}",
                    oxgraft_core::checkpoint::Checkpoints::location(&ws)
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                ),
                Err(e) => format!("unavailable: {e} (`oxgraft undo` still works)"),
            }
        };
        line(true, "checkpoints", checkpoints);
        line(true, "streaming", if cfg.stream { "on".into() } else { "off".into() });
    }
    println!("\n{}", if problems == 0 { "ready".to_string() } else { format!("{problems} problem(s) found") });
    Ok(if problems == 0 { 0 } else { EXIT_USAGE })
}
