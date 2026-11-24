//! The `oxgraft` command (SPEC-0004).

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
    },
    /// Interactive session (the default when no command is given).
    Chat {
        /// Continue a saved session (id or `last`).
        #[arg(long)]
        resume: Option<String>,
    },
    /// List saved sessions in this workspace.
    Sessions,
    /// Restore files changed by a session (default: the latest).
    Undo { id: Option<String> },
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

/// Exit codes (SPEC-0004 AC2).
const EXIT_ERROR: u8 = 1;
const EXIT_USAGE: u8 = 2;
const EXIT_STOPPED: u8 = 3;

fn available() -> Available {
    Available {
        edit_formats: oxgraft_strategies::edit_formats(),
        contexts: oxgraft_strategies::contexts(),
        approvals: oxgraft_strategies::approvals(),
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
        None => chat(&common, None),
        Some(Command::Chat { resume }) => chat(&common, resume),
        Some(Command::Run { task, json, resume }) => run(&common, task, json, resume),
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
        Some(Command::McpFixture) => {
            oxgraft_tools::mcp::serve_fixture();
            Ok(0)
        }
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
        },
    )
    .map_err(usage)?;
    let model_spec = cfg.model.clone().ok_or_else(|| {
        usage(
            "no model configured. Set ANTHROPIC_API_KEY to use anthropic/claude-opus-5-5, or pass --model \
             provider/model (see `oxgraft models`), or set `model` in .oxgraft/config.toml",
        )
    })?;
    let model_ref = oxgraft_providers::parse_model(&model_spec).map_err(usage)?;
    let provider = oxgraft_providers::build(&model_ref, &oxgraft_providers::Options { effort: cfg.effort.clone() })
        .map_err(usage)?;
    let (_, profile_text) = profiles::load(&ws, &cfg.profile).map_err(usage)?;
    let profile = Profile::from_toml(&profile_text).map_err(usage)?;
    let toolkit = Toolkit::assemble(profile, available()).map_err(usage)?;
    let mut config = AgentConfig::new(&model_ref.model, ws.clone());
    config.max_turns = cfg.max_turns;
    config.max_tokens = cfg.max_tokens;
    config.max_session_tokens = cfg.max_session_tokens;
    config.context_window = cfg.context_window;
    config.hooks = cfg.hooks.clone();
    for warning in &cfg.warnings {
        eprintln!("oxgraft: warning: {warning}");
    }
    let (mut tools, skills) = oxgraft_tools::for_workspace(&ws);
    for (name, server) in &cfg.mcp_servers {
        match oxgraft_tools::mcp::connect(name, server, &ws) {
            Ok(found) => tools.extend(found),
            // One broken server must not stop the session (stability over completeness).
            Err(e) => eprintln!("oxgraft: warning: {e}"),
        }
    }
    config.append_system = skills;
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

fn run(common: &Common, task: Vec<String>, json: bool, resume: Option<String>) -> CliResult {
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
    let s = setup(common)?;
    let mut session = match resume {
        Some(id) => load_session(&s.ws, &id)?,
        None => Session::new(&s.model, &s.toolkit.profile.name, &s.ws),
    };
    let approver: Box<dyn Approver> = if common.yes || json || !std::io::stdin().is_terminal() {
        Box::new(Fixed(if common.yes { Answer::Yes } else { Answer::No }))
    } else {
        Box::new(TerminalApprover)
    };
    let agent = Agent::new(&s.toolkit, s.provider.as_ref(), s.tools, approver.as_ref(), s.config);
    let mut renderer = render::Human::default();
    let outcome = agent.run(&mut session, task.trim(), &mut |e: &Event| {
        if json {
            println!("{}", e.to_jsonl());
            let _ = std::io::stdout().flush();
        } else {
            renderer.event(e);
        }
    });
    if !json {
        eprintln!("session {} · resume with: oxgraft run --resume {} \"...\"", session.id, session.id);
    }
    Ok(status_code(&outcome.status))
}

fn chat(common: &Common, resume: Option<String>) -> CliResult {
    let s = setup(common)?;
    let mut session = match resume {
        Some(id) => load_session(&s.ws, &id)?,
        None => Session::new(&s.model, &s.toolkit.profile.name, &s.ws),
    };
    let approver: Box<dyn Approver> =
        if common.yes { Box::new(Fixed(Answer::Yes)) } else { Box::new(TerminalApprover) };
    let agent = Agent::new(&s.toolkit, s.provider.as_ref(), s.tools, approver.as_ref(), s.config);
    eprintln!(
        "oxgraft · {} · profile {} · {}\nType a task. Commands: /undo, /sessions, /exit",
        s.model,
        s.toolkit.profile.name,
        s.ws.display()
    );
    let stdin = std::io::stdin();
    let mut last = 0;
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
            _ => {}
        }
        agent.config.cancel.store(false, std::sync::atomic::Ordering::SeqCst);
        let mut renderer = render::Human::default();
        let outcome = agent.run(&mut session, line, &mut |e: &Event| renderer.event(e));
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
        loop {
            eprint!("  allow {tool}: {detail}? [y]es / [n]o / [a]lways for {tool}: ");
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
