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
    /// Allow a tool without asking for this run: `TOOL` or `TOOL:PATTERN`, e.g.
    /// `bash:cargo *` (repeatable; deny rules still win).
    #[arg(long = "allow-tool", global = true, value_name = "TOOL[:PATTERN]")]
    allow_tool: Vec<String>,
    /// Deny a tool for this run: `TOOL` or `TOOL:PATTERN` (repeatable).
    #[arg(long = "deny-tool", global = true, value_name = "TOOL[:PATTERN]")]
    deny_tool: Vec<String>,
    /// Plan only: read-only mode, and the model returns a step-by-step plan
    /// instead of changing anything. Continue with `--resume last`.
    #[arg(long, global = true, conflicts_with = "mode")]
    plan: bool,
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
        /// Print only the final answer on stdout (no streaming or tool activity);
        /// errors still go to stderr.
        #[arg(long, short = 'q', conflicts_with = "json")]
        quiet: bool,
        /// Also write the final answer to this file.
        #[arg(long, value_name = "FILE")]
        output_last_message: Option<PathBuf>,
        /// Run in a new git worktree on branch `oxgraft/<id>`, leaving your checkout
        /// untouched; merge the branch afterwards if you like the result.
        #[arg(long, conflicts_with_all = ["resume", "fork"])]
        worktree: bool,
        /// The final answer must be a JSON object matching this JSON schema file;
        /// it is printed as compact JSON (pair with --quiet for scripts).
        #[arg(long, value_name = "FILE")]
        output_schema: Option<PathBuf>,
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
    Sessions {
        /// One JSON object per session (id, created, title, model, profile, counts).
        #[arg(long)]
        json: bool,
    },
    /// Restore files changed by a session (default: the latest).
    Undo { id: Option<String> },
    /// Show the settings in effect here, after every config layer, and where they came from.
    Config,
    /// What a session changed since it started, as a patch (default: the latest).
    Diff { id: Option<String> },
    /// Token use per model across this workspace's sessions.
    Stats {
        #[arg(long)]
        json: bool,
    },
    /// Delete old sessions in this workspace, keeping the newest ones.
    Prune {
        /// How many sessions to keep.
        #[arg(long, default_value_t = 50)]
        keep: usize,
        /// Only list what would be deleted.
        #[arg(long)]
        dry_run: bool,
    },
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

/// Plan mode instructions (GRAFT-0070, after Claude Code's plan mode).
const PLAN_INSTRUCTIONS: &str = "PLAN MODE: do not change anything (edits and commands are disabled). Explore with \
the read-only tools, then reply with a concise plan: the goal, the files to change and how, the risks, and how the \
change will be verified, as numbered steps. The user will review it and continue the session to carry it out.";

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

/// A new git worktree for a run (GRAFT-0102, after Claude Code's `--worktree`):
/// `.oxgraft/worktrees/<id>` on branch `oxgraft/<id>` from the current HEAD.
fn create_worktree(ws: &Path) -> Result<(PathBuf, String), String> {
    // Git for Windows mishandles `\\?\` paths from canonicalize (review fix).
    let ws = &oxgraft_core::checkpoint::plain_path(ws);
    let git = |args: &[&str]| -> Result<String, String> {
        let out =
            std::process::Command::new("git").arg("-C").arg(ws).args(args).output().map_err(|e| format!("git: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    };
    git(&["rev-parse", "--verify", "HEAD"])
        .map_err(|_| "--worktree needs a git repository with at least one commit".to_string())?;
    let id = Session::new("", "", ws).id;
    let branch = format!("oxgraft/{id}");
    let path = ws.join(".oxgraft").join("worktrees").join(&id);
    let path_text = path.to_string_lossy().into_owned();
    // Keep the worktree out of the user's own `git status`.
    let ignore = ws.join(".oxgraft").join(".gitignore");
    if !ignore.exists() {
        std::fs::create_dir_all(ws.join(".oxgraft")).map_err(|e| e.to_string())?;
        std::fs::write(&ignore, "*\n").map_err(|e| e.to_string())?;
    }
    git(&["worktree", "add", "-q", "-b", &branch, &path_text, "HEAD"])
        .map_err(|e| format!("git worktree add failed: {e}"))?;
    Ok((path, branch))
}

/// Commit everything in a worktree made by `run --worktree`. Ok(false) = nothing
/// to commit. Uses the user's git identity, falling back to a neutral one.
fn commit_worktree(path: &Path, title: &str) -> Result<bool, String> {
    let git = |args: &[&str]| {
        std::process::Command::new("git").arg("-C").arg(path).args(args).output().map_err(|e| format!("git: {e}"))
    };
    git(&["add", "-A"])?;
    if git(&["diff", "--cached", "--quiet"])?.status.success() {
        return Ok(false);
    }
    let message = format!("oxgraft: {}", if title.trim().is_empty() { "changes" } else { title.trim() });
    let mut out = git(&["commit", "-q", "--no-verify", "-m", &message])?;
    if !out.status.success() {
        // No identity configured: commit as a neutral author rather than fail.
        out = git(&[
            "-c",
            "user.name=oxgraft",
            "-c",
            "user.email=oxgraft@localhost",
            "commit",
            "-q",
            "--no-verify",
            "-m",
            &message,
        ])?;
    }
    if out.status.success() { Ok(true) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// Rules from `--allow-tool` / `--deny-tool` (GRAFT-0097, after Claude Code's
/// `--allowedTools` / `--disallowedTools`): `TOOL` or `TOOL:PATTERN`. Added last,
/// so they beat mode and config allows; any deny still wins over any allow.
fn flag_rules(allow: &[String], deny: &[String]) -> Vec<oxgraft_strategies::approval::Rule> {
    use oxgraft_seams::Decision;
    use oxgraft_strategies::approval::Rule;
    let parse = |spec: &str, d: Decision| {
        let (tool, pattern) = spec.split_once(':').unwrap_or((spec, "*"));
        // Tool names are lowercase (`bash`, `web_fetch`); accept `Bash` too.
        let rule = Rule::new(&tool.trim().to_ascii_lowercase(), pattern.trim(), d);
        if d == Decision::Deny { rule.because("denied for this run with --deny-tool") } else { rule }
    };
    allow.iter().map(|a| parse(a, Decision::Allow)).chain(deny.iter().map(|d| parse(d, Decision::Deny))).collect()
}

/// Approval modes, in order of autonomy.
const MODES: &[&str] = &["read-only", "suggest", "auto-edit", "full-auto"];

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
        Some(Command::Run { task, json, resume, fork, until, quiet, output_last_message, output_schema, worktree }) => {
            let schema = match output_schema {
                Some(path) => {
                    let text = std::fs::read_to_string(&path).map_err(|e| usage(format!("{}: {e}", path.display())))?;
                    Some(serde_json::from_str::<Value>(&text).map_err(|e| usage(format!("{}: {e}", path.display())))?)
                }
                None => None,
            };
            let output = RunOutput { json, quiet, last_message: output_last_message, schema };
            if worktree {
                let ws = workspace(&common)?;
                let (path, branch) = create_worktree(&ws).map_err(|e| (EXIT_ERROR, e))?;
                if !json && !quiet {
                    eprintln!("working in {} on branch {branch}", path.display());
                }
                let mut common = common.clone();
                common.cwd = Some(path.clone());
                let title: String = task.join(" ").lines().next().unwrap_or("").chars().take(60).collect();
                let result = run(&common, task, output, None, None, until);
                // Commit what the run changed, so the branch can be merged (review fix).
                let committed = commit_worktree(&path, &title);
                if !json && !quiet {
                    match committed {
                        Ok(true) => eprintln!(
                            "worktree {} on branch {branch}: review with `git diff HEAD...{branch}`, merge with \
                             `git merge {branch}`, discard with `git worktree remove --force {}` and `git branch -D {branch}`",
                            path.display(),
                            path.display()
                        ),
                        Ok(false) => eprintln!(
                            "worktree {} on branch {branch}: no changes; remove with `git worktree remove {}` and \
                             `git branch -D {branch}`",
                            path.display(),
                            path.display()
                        ),
                        Err(e) => eprintln!(
                            "worktree {} on branch {branch}: could not commit the changes: {e}",
                            path.display()
                        ),
                    }
                }
                return result;
            }
            run(&common, task, output, resume, fork, until)
        }
        Some(Command::Sessions { json }) => {
            let ws = workspace(&common)?;
            let sessions = Session::list(&ws);
            if json {
                for s in &sessions {
                    let line = serde_json::json!({
                        "id": s.id,
                        "created": s.created,
                        "title": s.title,
                        "model": s.model,
                        "profile": s.profile,
                        "messages": s.messages.len(),
                        "files_changed": s.changed_files.len(),
                        "checkpoints": s.checkpoints.len(),
                        "input_tokens": s.usage.input_tokens,
                        "output_tokens": s.usage.output_tokens,
                        "forked_from": s.forked_from,
                    });
                    println!("{line}");
                }
                return Ok(0);
            }
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
        Some(Command::Config) => {
            let ws = workspace(&common)?;
            let flags = config::Flags {
                model: common.model.as_deref(),
                profile: common.profile.as_deref(),
                effort: common.effort.as_deref(),
                max_turns: common.max_turns,
                approval_mode: if common.plan { Some("read-only") } else { common.mode.as_deref() },
                instructions: common.append_system_prompt.as_deref(),
            };
            let cfg = config::resolve(&ws, &flags).map_err(usage)?;
            for line in describe_config(&ws, &cfg) {
                println!("{line}");
            }
            Ok(0)
        }
        Some(Command::Diff { id }) => {
            let ws = workspace(&common)?;
            let session = load_session(&ws, id.as_deref().unwrap_or("last"))?;
            print!("{}", session_diff(&ws, &session).map_err(|e| (EXIT_ERROR, e))?);
            Ok(0)
        }
        Some(Command::Stats { json }) => {
            let ws = workspace(&common)?;
            let rows = usage_by_model(&Session::list(&ws));
            if json {
                for (model, (sessions, input, output)) in &rows {
                    println!(
                        "{}",
                        serde_json::json!({"model": model, "sessions": sessions, "input_tokens": input, "output_tokens": output})
                    );
                }
                return Ok(0);
            }
            if rows.is_empty() {
                println!("no sessions in {}", ws.display());
            }
            println!("{:<36} {:>8} {:>14} {:>14}", "MODEL", "SESSIONS", "INPUT TOKENS", "OUTPUT TOKENS");
            for (model, (sessions, input, output)) in &rows {
                println!("{model:<36} {sessions:>8} {input:>14} {output:>14}");
            }
            Ok(0)
        }
        Some(Command::Prune { keep, dry_run }) => {
            let ws = workspace(&common)?;
            let removed = Session::prune(&ws, keep, dry_run).map_err(|e| (EXIT_ERROR, e.to_string()))?;
            let verb = if dry_run { "would delete" } else { "deleted" };
            println!("{verb} {} session(s), keeping the newest {keep}", removed.len());
            for id in removed {
                println!("  {id}");
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
        Some(Command::Init { json }) => {
            let output = RunOutput { json, quiet: false, last_message: None, schema: None };
            run(&common, vec![INIT_TASK.to_string()], output, None, None, None)
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
    /// What built `toolkit`, so chat can build one per approval mode (GRAFT-0077).
    profile_text: String,
    user_rules: Vec<oxgraft_strategies::approval::Rule>,
    mode: String,
    provider: Box<dyn oxgraft_seams::Provider>,
    model: String,
    config: AgentConfig,
    tools: Vec<Box<dyn oxgraft_seams::Tool>>,
    /// Ring the terminal bell after long tasks.
    bell: bool,
    /// The configured compaction window, and whether it was left at the default
    /// (so `/model` can apply the local-model default).
    context_window: Option<u64>,
    context_window_default: bool,
    /// Provider options (effort, thinking), for switching models in chat.
    effort: Option<String>,
    thinking: bool,
    /// Provider and model for the `task` subagent, when `subagent_model` is set.
    subagent: Option<(Box<dyn oxgraft_seams::Provider>, String)>,
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
            approval_mode: if common.plan { Some("read-only") } else { common.mode.as_deref() },
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
    let provider = oxgraft_providers::build(
        &model_ref,
        &oxgraft_providers::Options { effort: cfg.effort.clone(), no_thinking: !cfg.thinking },
    )
    .map_err(usage)?;
    let (_, profile_text) =
        profiles::load(&ws, &profiles::resolve_auto(&cfg.profile, &model_ref.model), cfg.trusted).map_err(usage)?;
    let profile = Profile::from_toml(&profile_text).map_err(usage)?;
    let mut rules = mode_rules(&cfg.approval_mode).map_err(usage)?;
    rules.extend(cfg.rules.clone());
    rules.extend(flag_rules(&common.allow_tool, &common.deny_tool));
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
    config.todo_reminder = !common.plan;
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
    if common.plan {
        // Plan mode (GRAFT-0070): edits and commands are denied by read-only mode.
        extra.push(PLAN_INSTRUCTIONS.into());
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
    let subagent = match &cfg.subagent_model {
        Some(spec) => {
            let m =
                oxgraft_providers::parse_model(spec).map_err(|e| usage(format!("subagent_model: {}", e.message)))?;
            let options = oxgraft_providers::Options { effort: cfg.effort.clone(), no_thinking: !cfg.thinking };
            let p =
                oxgraft_providers::build(&m, &options).map_err(|e| usage(format!("subagent_model: {}", e.message)))?;
            Some((p, m.model))
        }
        None => None,
    };
    Ok(Setup {
        ws,
        effort: cfg.effort.clone(),
        thinking: cfg.thinking,
        bell: cfg.bell,
        context_window: cfg.context_window,
        context_window_default: cfg.context_window_default,
        subagent,
        toolkit,
        profile_text,
        user_rules: {
            let mut r = cfg.rules.clone();
            r.extend(flag_rules(&common.allow_tool, &common.deny_tool));
            r
        },
        mode: cfg.approval_mode.clone(),
        provider,
        model: model_spec,
        config,
        tools,
    })
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

/// How `run` reports (GRAFT-0075 for quiet and last-message).
struct RunOutput {
    json: bool,
    quiet: bool,
    last_message: Option<PathBuf>,
    /// `--output-schema` (GRAFT-0101).
    schema: Option<Value>,
}

fn run(
    common: &Common,
    task: Vec<String>,
    output: RunOutput,
    resume: Option<String>,
    fork: Option<String>,
    until: Option<String>,
) -> CliResult {
    let json = output.json;
    if let Some(path) = &output.last_message {
        // Cleared first, so a run that fails early never leaves an old answer behind.
        std::fs::write(path, "").map_err(|e| (EXIT_ERROR, format!("{}: {e}", path.display())))?;
    }
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
    let looks_like_command =
        task.trim_start().strip_prefix('/').and_then(|rest| rest.split_whitespace().next()).is_some_and(|name| {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        });
    if looks_like_command {
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
    s.config.output_schema = output.schema.clone();
    let mut session = open_session(&s, resume, fork)?;
    let approver: Box<dyn Approver> = if common.yes || json || !std::io::stdin().is_terminal() {
        Box::new(Fixed(if common.yes { Answer::Yes } else { Answer::No }))
    } else {
        Box::new(TerminalApprover)
    };
    let mut agent = Agent::new(&s.toolkit, s.provider.as_ref(), s.tools, approver.as_ref(), s.config);
    agent.subagent = s.subagent.as_ref().map(|(p, m)| (p.as_ref(), m.clone()));
    let mut renderer = render::Human::default();
    let started = std::time::Instant::now();
    let before = session.checkpoints.len();
    let outcome = agent.run(&mut session, task.trim(), &mut |e: &Event| {
        if json {
            println!("{}", e.to_jsonl());
            let _ = std::io::stdout().flush();
        } else if output.quiet {
            if let Event::Warning { message } = e {
                eprintln!("oxgraft: warning: {message}");
            }
        } else {
            renderer.event(e);
        }
    });
    // With a schema, the answer is reported as compact JSON (fences removed).
    let answer = match &output.schema {
        Some(_) => oxgraft_core::schema::normalise(&outcome.final_text)
            .unwrap_or_else(|| outcome.final_text.trim().to_string()),
        None => outcome.final_text.trim().to_string(),
    };
    if output.quiet {
        if !answer.is_empty() {
            println!("{answer}");
        }
        if let Status::Stopped(reason) | Status::Failed(reason) = &outcome.status {
            eprintln!("oxgraft: stopped: {reason}");
        }
    } else if !json {
        change_summary(&s.ws, &session, before);
        ring_if_long(started, s.bell);
        eprintln!("session {} · resume with: oxgraft run --resume {} \"...\"", session.id, session.id);
    }
    if let Some(path) = &output.last_message {
        std::fs::write(path, &answer).map_err(|e| (EXIT_ERROR, format!("{}: {e}", path.display())))?;
    }
    Ok(status_code(&outcome.status))
}

/// Read a free-text approval answer (GRAFT-0078, review fix): yes/always phrasings
/// approve, "no, …" declines with the rest as guidance, other text of a few words
/// is guidance. `None` = ask again.
fn classify_answer(lower: &str, original: &str) -> Option<Answer> {
    let first = lower.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("");
    const YES: &[&str] =
        &["y", "yes", "yeah", "yep", "ok", "okay", "sure", "allow", "approve", "approved", "go", "proceed", "fine"];
    if first == "always" {
        return Some(Answer::Always);
    }
    if YES.contains(&first) {
        return Some(Answer::Yes);
    }
    if matches!(first, "n" | "no" | "nope" | "don" | "dont" | "stop") {
        let rest = original
            .trim_start_matches(|c: char| c.is_alphanumeric() || c == '\'')
            .trim_start_matches([',', ':', ';', '-', '.', ' '])
            .trim();
        return Some(if rest.is_empty() { Answer::No } else { Answer::NoWith(rest.to_string()) });
    }
    (lower.chars().count() > 3).then(|| Answer::NoWith(original.to_string()))
}

/// Model-supplied text shown in a prompt: control characters (escape sequences,
/// carriage returns) are made visible so they can't repaint the screen.
fn printable(line: &str) -> String {
    line.chars().map(|c| if c.is_control() && c != '\t' { '?' } else { c }).collect()
}

/// Preview of an edit or write for the approval prompt: SEARCH/REPLACE or patch
/// text as sent; old/new_string pairs (and batched `edits`) as `- old` / `+ new`
/// lines (GRAFT-0084); file content for writes. At most `max` lines.
fn preview_lines(tool: &str, input: &Value, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let pair = |lines: &mut Vec<String>, old: &str, new: &str| {
        lines.extend(old.lines().map(|l| format!("- {l}")));
        lines.extend(new.lines().map(|l| format!("+ {l}")));
    };
    match tool {
        "edit" if input.get("edit").is_some() => {
            lines.extend(input["edit"].as_str().unwrap_or("").lines().map(str::to_string));
        }
        "edit" if input.get("edits").is_some() => {
            for (i, e) in input["edits"].as_array().into_iter().flatten().enumerate() {
                if i > 0 {
                    lines.push("···".into());
                }
                pair(&mut lines, e["old_string"].as_str().unwrap_or(""), e["new_string"].as_str().unwrap_or(""));
            }
        }
        "edit" => {
            pair(&mut lines, input["old_string"].as_str().unwrap_or(""), input["new_string"].as_str().unwrap_or(""))
        }
        "write" => lines.extend(input["content"].as_str().unwrap_or("").lines().map(str::to_string)),
        _ => {}
    }
    if lines.len() > max {
        let more = lines.len() - max;
        lines.truncate(max);
        lines.push(format!("… {more} more lines"));
    }
    lines
}

/// Read one chat message (GRAFT-0082): a line; lines ending in `\\` continue; a
/// line that is just `"""` starts a block that runs until the next `"""`. `None`
/// at end of input.
fn read_message(input: &mut dyn BufRead) -> std::io::Result<Option<String>> {
    let mut first = String::new();
    if input.read_line(&mut first)? == 0 {
        return Ok(None);
    }
    if first.trim() == "\"\"\"" {
        let mut text = String::new();
        loop {
            let mut line = String::new();
            if input.read_line(&mut line)? == 0 || line.trim() == "\"\"\"" {
                return Ok(Some(text.trim_end().to_string()));
            }
            text.push_str(&line);
        }
    }
    let mut text = first.trim_end_matches(['\r', '\n']).to_string();
    while text.ends_with('\\') {
        text.pop();
        text.push('\n');
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        text.push_str(line.trim_end_matches(['\r', '\n']));
    }
    Ok(Some(text))
}

/// Run a command the user typed with `!` in chat (GRAFT-0058): the user's own
/// action, so no approval; same shell as the bash tool; 5-minute limit.
fn run_user_command(ws: &Path, command: &str) -> (bool, String) {
    let (program, args) = oxgraft_tools::shell_program();
    oxgraft_core::hooks::run_check(&oxgraft_core::hooks::Shell { program, args }, command, ws, 300, false)
}

/// Ring the terminal bell when a task took long enough that the user may have
/// looked away (GRAFT-0085). Only on a terminal; `bell = false` turns it off.
fn ring_if_long(started: std::time::Instant, enabled: bool) {
    if enabled && started.elapsed() >= std::time::Duration::from_secs(30) && std::io::stderr().is_terminal() {
        eprint!("\x07");
        let _ = std::io::stderr().flush();
    }
}

/// The resolved configuration as `key  value` lines (GRAFT-0089).
fn describe_config(ws: &Path, cfg: &config::Config) -> Vec<String> {
    let or = |v: Option<String>, d: &str| v.unwrap_or_else(|| d.to_string());
    let mut out = vec![
        format!("{:<20} {}", "workspace", ws.display()),
        format!(
            "{:<20} {}",
            "trusted",
            if cfg.trusted { "yes" } else { "no (repo hooks, MCP, rules, model, profile ignored)" }
        ),
        format!("{:<20} {}", "model", or(cfg.model.clone(), "(none)")),
        format!("{:<20} {}", "subagent_model", or(cfg.subagent_model.clone(), "(same as model)")),
        format!("{:<20} {}", "profile", cfg.profile),
        format!("{:<20} {}", "approval_mode", cfg.approval_mode),
        format!("{:<20} {}", "effort", or(cfg.effort.clone(), "(provider default)")),
        format!("{:<20} {}", "thinking", cfg.thinking),
        format!("{:<20} {}", "stream", cfg.stream),
        format!("{:<20} {}", "checkpoints", cfg.checkpoints),
        format!("{:<20} {}", "bell", cfg.bell),
        format!("{:<20} {}", "max_turns", cfg.max_turns),
        format!("{:<20} {}", "max_tokens", cfg.max_tokens),
        format!("{:<20} {}", "max_session_tokens", cfg.max_session_tokens.map_or("(none)".into(), |v| v.to_string())),
        format!(
            "{:<20} {}{}",
            "context_window",
            cfg.context_window.map_or("off".into(), |v| v.to_string()),
            if cfg.context_window_default { " (default; 32768 for local models)" } else { "" }
        ),
        format!("{:<20} {}", "verify_command", or(cfg.verify_command.clone(), "(none)")),
        format!("{:<20} {}", "instructions", cfg.instructions.len()),
        format!("{:<20} {}", "hooks", cfg.hooks.len()),
        format!(
            "{:<20} {}",
            "mcp_servers",
            if cfg.mcp_servers.is_empty() {
                "(none)".into()
            } else {
                cfg.mcp_servers.keys().cloned().collect::<Vec<_>>().join(", ")
            }
        ),
        format!("{:<20} {}", "rules", cfg.rules.len()),
        format!(
            "{:<20} {}",
            "sources",
            if cfg.sources.is_empty() { "defaults only".into() } else { cfg.sources.join(", ") }
        ),
        format!(
            "{:<20} {}",
            "user config",
            config::user_config_path().map_or("(none)".into(), |p| p.display().to_string())
        ),
        format!(
            "{:<20} {}",
            "data dir",
            oxgraft_core::checkpoint::data_dir().map_or("(none)".into(), |p| p.display().to_string())
        ),
    ];
    out.extend(cfg.warnings.iter().map(|w| format!("{:<20} {w}", "warning")));
    out
}

/// The patch from the session's first checkpoint to the workspace as it is now
/// (GRAFT-0088, after Aider's /diff), including changes made by shell commands.
fn session_diff(ws: &Path, session: &Session) -> Result<String, String> {
    let Some(first) = session.checkpoints.first() else {
        return Ok(format!("session {} has no checkpoints, so there is nothing to diff\n", session.id));
    };
    let cp = oxgraft_core::checkpoint::Checkpoints::open(ws)?;
    let now = cp.snapshot("diff")?;
    let patch = cp.diff_patch(&first.commit, &now)?;
    Ok(if patch.trim().is_empty() { "no changes since the session started\n".into() } else { format!("{patch}\n") })
}

/// Sessions, input and output tokens per model (GRAFT-0087), most input first.
fn usage_by_model(sessions: &[Session]) -> Vec<(String, (usize, u64, u64))> {
    let mut map: std::collections::BTreeMap<String, (usize, u64, u64)> = std::collections::BTreeMap::new();
    for s in sessions {
        if s.usage_by_model.is_empty() {
            // Sessions saved before per-model accounting.
            let e = map.entry(s.model.clone()).or_default();
            e.0 += 1;
            e.1 += s.usage.input_tokens;
            e.2 += s.usage.output_tokens;
        }
        for (model, u) in &s.usage_by_model {
            let e = map.entry(model.clone()).or_default();
            e.0 += 1;
            e.1 += u.input_tokens;
            e.2 += u.output_tokens;
        }
    }
    let mut rows: Vec<_> = map.into_iter().collect();
    rows.sort_by(|a, b| b.1.1.cmp(&a.1.1).then_with(|| a.0.cmp(&b.0)));
    rows
}

/// How full the context is (GRAFT-0099, after Claude Code's /context).
fn context_line(used: u64, window: Option<u64>) -> String {
    match window {
        _ if used == 0 => "context: no requests yet".into(),
        Some(w) if w > 0 => format!(
            "context: {used} of {w} tokens ({}%); compaction at 80%, or /compact now",
            used.saturating_mul(100) / w
        ),
        _ => format!("context: {used} tokens (no window set, so no automatic compaction)"),
    }
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
    // One toolkit per approval mode, so `/mode` can switch without restarting.
    let mode_toolkits: Vec<Toolkit> = MODES
        .iter()
        .map(|m| {
            let mut rules = mode_rules(m).map_err(usage)?;
            rules.extend(s.user_rules.clone());
            let profile = Profile::from_toml(&s.profile_text).map_err(usage)?;
            Toolkit::assemble(profile, available_with(rules)).map_err(usage)
        })
        .collect::<Result<_, _>>()?;
    let (effort, thinking, bell) = (s.effort.clone(), s.thinking, s.bell);
    let (context_window, context_window_default) = (s.context_window, s.context_window_default);
    let mut current_spec = s.model.clone();
    let mut mode = s.mode.clone();
    let mut agent = Agent::new(&s.toolkit, s.provider.as_ref(), s.tools, approver.as_ref(), s.config);
    agent.subagent = s.subagent.as_ref().map(|(p, m)| (p.as_ref(), m.clone()));
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
        let Some(input) = read_message(&mut stdin.lock()).map_err(usage)? else { break };
        let line = input.trim();
        // `/init` (GRAFT-0032 in chat): write or improve AGENTS.md.
        let line = if line == "/init" { INIT_TASK } else { line };
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
            e if e == "/export" || e.starts_with("/export ") => {
                // GRAFT-0066 in chat: the conversation as Markdown.
                let name = e.trim_start_matches("/export").trim();
                let path =
                    if name.is_empty() { s.ws.join(format!("oxgraft-{}.md", session.id)) } else { s.ws.join(name) };
                match std::fs::write(&path, session.to_markdown()) {
                    Ok(()) => eprintln!("wrote {}", path.display()),
                    Err(err) => eprintln!("export: {}: {err}", path.display()),
                }
                continue;
            }
            r if r.starts_with("/resume") => {
                // GRAFT-0100: switch to a saved session (the current one is already saved).
                let id = r.trim_start_matches("/resume").trim();
                let id = if id.is_empty() { "last" } else { id };
                match load_session(&s.ws, id) {
                    Ok(other) if other.id == session.id => eprintln!("already in session {}", session.id),
                    Ok(other) => {
                        session = other;
                        let last = session
                            .messages
                            .iter()
                            .rev()
                            .find(|m| m.role == oxgraft_seams::Role::Assistant && !m.text().trim().is_empty())
                            .map(|m| m.text().chars().take(200).collect::<String>())
                            .unwrap_or_default();
                        eprintln!(
                            "resumed session {}: {} ({} messages)",
                            session.id,
                            session.title,
                            session.messages.len()
                        );
                        if !last.is_empty() {
                            eprintln!("last reply: {}", render::plain(&last));
                        }
                    }
                    Err((_, e)) => eprintln!("resume: {e}"),
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
                eprintln!("{}", context_line(session.context_tokens, agent.config.context_window));
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
            m if m == "/model" || m.starts_with("/model ") => {
                // GRAFT-0083: switch model mid-conversation; history is kept.
                let wanted = m.trim_start_matches("/model").trim();
                if wanted.is_empty() {
                    eprintln!(
                        "model: {} (switch with /model <provider/model> or an alias like sonnet)",
                        agent.config.model
                    );
                    continue;
                }
                let built = oxgraft_providers::parse_model(wanted).and_then(|r| {
                    oxgraft_providers::build(
                        &r,
                        &oxgraft_providers::Options { effort: effort.clone(), no_thinking: !thinking },
                    )
                    .map(|p| (r, p))
                });
                match built {
                    Ok((r, provider)) => {
                        // Thinking blocks are bound to the model that wrote them, and tool
                        // ids differ between APIs: hand over through a summary written by
                        // the current model, so the new one gets plain text (review fix).
                        let foreign = session.messages.iter().flat_map(|m| &m.content).any(|b| {
                            matches!(b, oxgraft_seams::Block::Opaque { .. } | oxgraft_seams::Block::ToolUse { .. })
                        });
                        if foreign {
                            let mut renderer = render::Human::default();
                            agent.compact_session(&mut session, &mut |e: &Event| renderer.event(e));
                        }
                        // The agent borrows its provider for the whole chat; a switched-to
                        // provider lives until exit (a few bytes per switch).
                        agent.provider = Box::leak(provider);
                        agent.config.model = r.model.clone();
                        agent.config.context_window =
                            if context_window_default && LOCAL_PROVIDERS.contains(&r.provider.as_str()) {
                                Some(LOCAL_CONTEXT_WINDOW)
                            } else {
                                context_window
                            };
                        current_spec = format!("{}/{}", r.provider, r.model);
                        session.model = current_spec.clone();
                        eprintln!("model: {} via {} (the edit format chosen at startup stays)", r.model, r.provider);
                    }
                    Err(e) => eprintln!("cannot switch to `{wanted}`: {}", e.message),
                }
                continue;
            }
            m if m == "/mode" || m.starts_with("/mode ") => {
                let wanted = m.trim_start_matches("/mode").trim();
                if wanted.is_empty() {
                    eprintln!("mode: {mode} (choose one of: {})", MODES.join(", "));
                } else if let Some(i) = MODES.iter().position(|x| *x == wanted) {
                    agent.toolkit = &mode_toolkits[i];
                    mode = wanted.to_string();
                    eprintln!("mode: {mode} (risky commands and protected paths still ask)");
                } else {
                    eprintln!("unknown mode `{wanted}`; choose one of: {}", MODES.join(", "));
                }
                continue;
            }
            "/diff" => {
                match session_diff(&s.ws, &session) {
                    Ok(patch) => {
                        let lines: Vec<&str> = patch.lines().collect();
                        for l in lines.iter().take(400) {
                            eprintln!("{}", printable(l));
                        }
                        if lines.len() > 400 {
                            eprintln!("… {} more lines (oxgraft diff {} for all)", lines.len() - 400, session.id);
                        }
                    }
                    Err(e) => eprintln!("diff: {e}"),
                }
                continue;
            }
            "/compact" => {
                let mut renderer = render::Human::default();
                if agent.compact_session(&mut session, &mut |e: &Event| renderer.event(e)) {
                    eprintln!("context compacted: {} messages summarised so far", session.archived_messages);
                } else {
                    eprintln!("nothing to compact");
                }
                continue;
            }
            "/clear" | "/new" => {
                session = Session::new(&current_spec, &s.toolkit.profile.name, &s.ws);
                eprintln!("started a new session {}", session.id);
                continue;
            }
            "/help" => {
                eprintln!(
                    "/undo      restore files changed in this session\n/checkpoints  list workspace checkpoints\n\
                     /restore N  put the workspace back to checkpoint N\n/cost      token usage for this session\n/compact   summarise the conversation to free context\n/diff      everything this session changed, as a patch\n/init      write or improve AGENTS.md for this repository\n/mode X    switch approvals: read-only, suggest, auto-edit, full-auto\n/model M   switch model (provider/model or alias), keeping the conversation\n\
                     /clear     start a new session (the old one stays saved)\n/sessions  list saved sessions\n/resume ID switch to a saved session (default: the latest)\n/export F  write this conversation as Markdown (default oxgraft-<id>.md)\n\
                     !cmd       run a shell command yourself; output goes with your next message\n\
                     \"\"\"        start or end a multi-line message (or end a line with \\ to continue)\n\
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
        let started = std::time::Instant::now();
        let before = session.checkpoints.len();
        // Custom commands (GRAFT-0057): `/name args` expands to the command's prompt.
        let task = commands::expand(&s.ws, line).unwrap_or_else(|| line.to_string());
        let task = format!("{}{task}", std::mem::take(&mut shell_context));
        let outcome = agent.run(&mut session, &task, &mut |e: &Event| renderer.event(e));
        change_summary(&s.ws, &session, before);
        ring_if_long(started, bell);
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
        for line in preview_lines(tool, input, 40) {
            eprintln!("    │ {}", printable(&line));
        }
        // Risk rails (GRAFT-0042): say why, and don't offer "always".
        let risk = if tool == "bash" { oxgraft_strategies::approval::shell_risk(&action.resource) } else { None };
        if let Some(why) = risk {
            eprintln!("  ! risky: {why}. This is asked every time.");
        }
        loop {
            if risk.is_some() {
                eprint!("  allow {tool}: {detail}? [y]es / [n]o, or say what to do instead: ");
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
                    "web_fetch" => oxgraft_core::agent::always_keys(tool, &action.resource)
                        .first()
                        .map_or("this URL".to_string(), |k| k.trim_start_matches("web_fetch:").to_string()),
                    _ => format!("all {tool}"),
                };
                eprint!("  allow {tool}: {detail}? [y]es / [n]o / [a]lways for {scope}, or say what to do instead: ");
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
                text => match classify_answer(text, answer.trim()) {
                    Some(a) => return a,
                    None => continue,
                },
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
        max_tokens: 4096,
    };
    let start = std::time::Instant::now();
    let mut deltas = 0usize;
    match provider.stream(&plain, &mut |_| deltas += 1) {
        Ok(c) => {
            let text = Message::assistant(c.content).text();
            if text.trim().is_empty() {
                out.push((
                    false,
                    "live reply",
                    "the model returned no text (raise max_tokens, or set thinking = false)".into(),
                ));
                return out;
            }
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
        max_tokens: 4096,
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
        approval_mode: if common.plan { Some("read-only") } else { common.mode.as_deref() },
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
                        let options =
                            oxgraft_providers::Options { effort: cfg.effort.clone(), no_thinking: !cfg.thinking };
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
        // The subagent's model needs working credentials too.
        if let Some(spec) = &cfg.subagent_model {
            let options = oxgraft_providers::Options { effort: cfg.effort.clone(), no_thinking: !cfg.thinking };
            match oxgraft_providers::parse_model(spec).and_then(|m| oxgraft_providers::build(&m, &options).map(|_| m)) {
                Ok(m) => line(true, "subagent", format!("{spec} (credentials found for `{}`)", m.provider)),
                Err(e) => line(false, "subagent", format!("{spec}: {}", e.message)),
            }
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

#[cfg(test)]
mod input_tests {
    use super::read_message;

    #[test]
    fn context_usage_lines() {
        assert_eq!(super::context_line(0, Some(100)), "context: no requests yet");
        assert!(super::context_line(50_000, Some(200_000)).starts_with("context: 50000 of 200000 tokens (25%)"));
        assert!(super::context_line(10, None).contains("no automatic compaction"));
    }

    #[test]
    fn approval_answers_are_read_generously() {
        use oxgraft_core::Answer;
        let a = |t: &str| super::classify_answer(&t.to_ascii_lowercase(), t);
        for yes in ["yes please", "okay", "Sure!", "allow", "ok go"] {
            assert_eq!(a(yes), Some(Answer::Yes), "{yes}");
        }
        assert_eq!(a("always please"), Some(Answer::Always));
        assert_eq!(a("no thanks"), Some(Answer::NoWith("thanks".into())));
        assert_eq!(a("no, use cargo check instead"), Some(Answer::NoWith("use cargo check instead".into())));
        assert_eq!(a("nope"), Some(Answer::No));
        assert_eq!(a("use the read tool"), Some(Answer::NoWith("use the read tool".into())));
        assert_eq!(a("xy"), None);
        assert_eq!(super::printable("a\x1b[2Jb\rc"), "a?[2Jb?c");
    }

    #[test]
    fn edit_previews_show_old_and_new() {
        let p = super::preview_lines(
            "edit",
            &serde_json::json!({"path": "a", "old_string": "x = 1", "new_string": "x = 2"}),
            40,
        );
        assert_eq!(p, vec!["- x = 1", "+ x = 2"]);
        let batch = serde_json::json!({"edits": [{"old_string": "a", "new_string": "b"}, {"old_string": "c", "new_string": "d"}]});
        assert_eq!(super::preview_lines("edit", &batch, 40), vec!["- a", "+ b", "···", "- c", "+ d"]);
        let long = serde_json::json!({"content": "1\n2\n3\n4"});
        assert_eq!(super::preview_lines("write", &long, 2), vec!["1", "2", "… 2 more lines"]);
        assert!(super::preview_lines("bash", &serde_json::json!({"command": "ls"}), 40).is_empty());
    }

    #[test]
    fn multi_line_messages() {
        let mut input = std::io::Cursor::new("one line\nfirst \\\nsecond\n\"\"\"\nblock a\n\nblock b\n\"\"\"\nlast");
        assert_eq!(read_message(&mut input).unwrap().as_deref(), Some("one line"));
        assert_eq!(read_message(&mut input).unwrap().as_deref(), Some("first \nsecond"));
        assert_eq!(read_message(&mut input).unwrap().as_deref(), Some("block a\n\nblock b"));
        assert_eq!(read_message(&mut input).unwrap().as_deref(), Some("last"));
        assert_eq!(read_message(&mut input).unwrap(), None);
    }
}
