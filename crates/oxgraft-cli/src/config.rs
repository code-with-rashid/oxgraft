//! Config resolution (SPEC-0004): flags → env → workspace → user → defaults.

use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use oxgraft_core::hooks::Hook;
use oxgraft_tools::mcp::ServerConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub model: Option<String>,
    pub profile: Option<String>,
    pub effort: Option<String>,
    pub max_turns: Option<u32>,
    pub max_tokens: Option<u32>,
    /// Stop a run when input + output tokens exceed this.
    pub max_session_tokens: Option<u64>,
    /// Run after any turn that changed files (e.g. "cargo check"); failures go back to the model.
    pub verify_command: Option<String>,
    /// Model context window in tokens (compaction at 80%); 0 disables compaction.
    pub context_window: Option<u64>,
    /// Stream model output as it is written (default true).
    pub stream: Option<bool>,
    /// Adaptive thinking for Anthropic models that support it (default true).
    pub thinking: Option<bool>,
    /// How much runs without asking: read-only, suggest (default), auto-edit, full-auto.
    pub approval_mode: Option<String>,
    /// Extra standing instructions appended to the system prompt.
    pub instructions: Option<String>,
    /// Model for the read-only `task` subagent, e.g. a cheaper one (`haiku`).
    pub subagent_model: Option<String>,
    /// Snapshot the workspace after each changing turn (default true).
    pub checkpoints: Option<bool>,
    #[serde(default)]
    pub hooks: Vec<Hook>,
    /// MCP servers to start (name -> command). Workspace entries need trust.
    #[serde(default)]
    pub mcp_servers: BTreeMap<String, ServerConfig>,
    /// Extra approval rules: `[[rules]] tool = "bash", resource = "cargo *", decision = "allow"`.
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
    /// User config only: workspaces whose own hooks and MCP servers may run.
    #[serde(default)]
    pub trusted_workspaces: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub model: Option<String>,
    pub profile: String,
    pub effort: Option<String>,
    pub max_turns: u32,
    pub max_tokens: u32,
    pub max_session_tokens: Option<u64>,
    pub verify_command: Option<String>,
    pub context_window: Option<u64>,
    /// No layer set `context_window` (so a provider-specific default may apply).
    pub context_window_default: bool,
    pub stream: bool,
    pub thinking: bool,
    pub approval_mode: String,
    /// Instructions from every config layer, in order (user, workspace, flags).
    pub instructions: Vec<String>,
    pub subagent_model: Option<String>,
    pub checkpoints: bool,
    pub hooks: Vec<Hook>,
    pub mcp_servers: BTreeMap<String, ServerConfig>,
    pub rules: Vec<oxgraft_strategies::approval::Rule>,
    pub warnings: Vec<String>,
    pub sources: Vec<String>,
    /// The workspace is listed in the user config's `trusted_workspaces`.
    pub trusted: bool,
}

pub fn user_config_path() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(x).join("oxgraft").join("config.toml"));
    }
    if cfg!(windows)
        && let Some(appdata) = std::env::var_os("APPDATA")
    {
        return Some(PathBuf::from(appdata).join("oxgraft").join("config.toml"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("oxgraft").join("config.toml"))
}

fn read(path: &Path) -> Result<Option<FileConfig>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).map(Some).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

pub struct Flags<'a> {
    pub instructions: Option<&'a str>,
    pub model: Option<&'a str>,
    pub profile: Option<&'a str>,
    pub effort: Option<&'a str>,
    pub max_turns: Option<u32>,
    pub approval_mode: Option<&'a str>,
}

fn same_dir(a: &str, b: &Path) -> bool {
    Path::new(a).canonicalize().map(|a| a == b).unwrap_or(false)
}

pub fn resolve(workspace: &Path, flags: &Flags<'_>) -> Result<Config, String> {
    let mut cfg = Config {
        model: None,
        profile: "auto".into(),
        effort: Some("high".into()),
        max_turns: 50,
        max_tokens: 16_000,
        max_session_tokens: None,
        verify_command: None,
        context_window: Some(200_000),
        context_window_default: true,
        stream: true,
        thinking: true,
        approval_mode: "suggest".into(),
        instructions: Vec::new(),
        subagent_model: None,
        checkpoints: true,
        hooks: Vec::new(),
        mcp_servers: BTreeMap::new(),
        rules: Vec::new(),
        warnings: Vec::new(),
        sources: Vec::new(),
        trusted: false,
    };

    let mut layers: Vec<(String, FileConfig)> = Vec::new();
    let mut trusted = Vec::new();
    if let Some(p) = user_config_path()
        && let Some(c) = read(&p)?
    {
        trusted = c.trusted_workspaces.clone();
        cfg.hooks.extend(c.hooks.clone());
        cfg.mcp_servers.extend(c.mcp_servers.clone());
        check_examples(&c.rules).map_err(|e| format!("{}: {e}", p.display()))?;
        cfg.rules.extend(convert_rules(&c.rules, &mut cfg.warnings));
        layers.push((p.display().to_string(), c));
    }
    let is_trusted = trusted.iter().any(|t| same_dir(t, workspace));
    cfg.trusted = is_trusted;
    let local_profiles = std::fs::read_dir(workspace.join("profiles"))
        .map(|d| d.flatten().any(|e| e.path().extension().is_some_and(|x| x == "toml")))
        .unwrap_or(false);
    if local_profiles && !is_trusted {
        cfg.warnings.push(
            "ignoring profiles/*.toml in this workspace (built-in profiles are used): workspace not trusted".into(),
        );
    }
    // Claude Code-style project MCP file (.mcp.json), honoured only for trusted workspaces.
    if let Ok(text) = std::fs::read_to_string(workspace.join(".mcp.json")) {
        #[derive(Deserialize)]
        struct McpJson {
            #[serde(rename = "mcpServers", default)]
            servers: BTreeMap<String, ServerConfig>,
        }
        match serde_json::from_str::<McpJson>(&text) {
            Ok(m) if is_trusted => cfg.mcp_servers.extend(m.servers),
            Ok(m) if !m.servers.is_empty() => cfg.warnings.push(format!(
                "ignoring {} MCP server(s) in .mcp.json: add this workspace to `trusted_workspaces` in your user config to enable them",
                m.servers.len()
            )),
            Ok(_) => {}
            Err(e) => cfg.warnings.push(format!(".mcp.json is invalid: {e}")),
        }
    }
    let ws_path = workspace.join(".oxgraft").join("config.toml");
    if let Some(c) = read(&ws_path)? {
        // A repository's own hooks are code from whoever wrote the repo: run them only
        // for workspaces the user has trusted in their own config (GRAFT-0019).
        if !c.hooks.is_empty() {
            if is_trusted {
                cfg.hooks.extend(c.hooks.clone());
            } else {
                cfg.warnings.push(format!(
                    "ignoring {} hook(s) in {}: add this workspace to `trusted_workspaces` in your user config to enable them",
                    c.hooks.len(),
                    ws_path.display()
                ));
            }
        }
        if !c.mcp_servers.is_empty() {
            if is_trusted {
                cfg.mcp_servers.extend(c.mcp_servers.clone());
            } else {
                cfg.warnings.push(format!(
                    "ignoring {} MCP server(s) in {}: workspace not trusted",
                    c.mcp_servers.len(),
                    ws_path.display()
                ));
            }
        }
        if !c.rules.is_empty() {
            if is_trusted {
                check_examples(&c.rules).map_err(|e| format!("{}: {e}", ws_path.display()))?;
                let rules = convert_rules(&c.rules, &mut cfg.warnings);
                cfg.rules.extend(rules);
            } else {
                cfg.warnings.push(format!(
                    "ignoring {} approval rule(s) in {}: workspace not trusted",
                    c.rules.len(),
                    ws_path.display()
                ));
            }
        }
        if !c.trusted_workspaces.is_empty() {
            cfg.warnings.push("`trusted_workspaces` is only read from the user config".into());
        }
        // GRAFT-0034 (Codex): a cloned repo must not redirect prompts and keys to a
        // provider of its choosing. Untrusted workspaces may not set the model.
        let mut c = c;
        if !is_trusted && let Some(command) = c.verify_command.take() {
            cfg.warnings.push(format!(
                "ignoring `verify_command = \"{command}\"` in {}: workspace not trusted",
                ws_path.display()
            ));
        }
        // GRAFT-0051: the profile chooses the approval policy (e.g. `ci` approves
        // everything), so an untrusted workspace may not pick it either.
        if !is_trusted && let Some(m) = c.subagent_model.take() {
            cfg.warnings
                .push(format!("ignoring `subagent_model = \"{m}\"` in {}: workspace not trusted", ws_path.display()));
        }
        if !is_trusted && c.instructions.take().is_some() {
            cfg.warnings.push(format!(
                "ignoring `instructions` in {}: workspace not trusted (put repository guidance in AGENTS.md)",
                ws_path.display()
            ));
        }
        if !is_trusted && let Some(mode) = c.approval_mode.take() {
            cfg.warnings.push(format!(
                "ignoring `approval_mode = \"{mode}\"` in {}: workspace not trusted (pass --mode to choose one)",
                ws_path.display()
            ));
        }
        if !is_trusted && let Some(profile) = c.profile.take() {
            cfg.warnings.push(format!(
                "ignoring `profile = \"{profile}\"` in {}: workspace not trusted (pass --profile to choose one)",
                ws_path.display()
            ));
        }
        if !is_trusted && let Some(model) = c.model.take() {
            cfg.warnings.push(format!(
                "ignoring `model = \"{model}\"` in {}: workspace not trusted (add it to `trusted_workspaces`, or pass --model)",
                ws_path.display()
            ));
        }
        layers.push((ws_path.display().to_string(), c));
    }
    layers.push((
        "environment".into(),
        FileConfig {
            model: env("OXGRAFT_MODEL"),
            profile: env("OXGRAFT_PROFILE"),
            effort: env("OXGRAFT_EFFORT"),
            approval_mode: env("OXGRAFT_APPROVAL_MODE"),
            max_turns: env("OXGRAFT_MAX_TURNS").and_then(|v| v.parse().ok()),
            max_tokens: env("OXGRAFT_MAX_TOKENS").and_then(|v| v.parse().ok()),
            max_session_tokens: env("OXGRAFT_MAX_SESSION_TOKENS").and_then(|v| v.parse().ok()),
            context_window: env("OXGRAFT_CONTEXT_WINDOW").and_then(|v| v.parse().ok()),
            stream: env("OXGRAFT_STREAM").and_then(|v| match v.to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => Some(true),
                "0" | "false" | "no" | "off" => Some(false),
                _ => None,
            }),
            ..FileConfig::default()
        },
    ));
    layers.push((
        "flags".into(),
        FileConfig {
            model: flags.model.map(str::to_string),
            profile: flags.profile.map(str::to_string),
            effort: flags.effort.map(str::to_string),
            max_turns: flags.max_turns,
            approval_mode: flags.approval_mode.map(str::to_string),
            instructions: flags.instructions.map(str::to_string),
            ..FileConfig::default()
        },
    ));

    for (source, layer) in layers {
        let mut used = false;
        if let Some(v) = layer.model {
            cfg.model = Some(v);
            used = true;
        }
        if let Some(v) = layer.profile {
            cfg.profile = v;
            used = true;
        }
        if let Some(v) = layer.subagent_model.filter(|v| !v.trim().is_empty()) {
            cfg.subagent_model = Some(v);
            used = true;
        }
        if let Some(v) = layer.instructions.filter(|v| !v.trim().is_empty()) {
            cfg.instructions.push(v);
            used = true;
        }
        if let Some(v) = layer.approval_mode {
            cfg.approval_mode = v;
            used = true;
        }
        if let Some(v) = layer.effort {
            cfg.effort = if v == "none" { None } else { Some(v) };
            used = true;
        }
        if let Some(v) = layer.max_turns {
            cfg.max_turns = v.max(1);
            used = true;
        }
        if let Some(v) = layer.max_tokens {
            cfg.max_tokens = v.max(256);
            used = true;
        }
        if let Some(v) = layer.verify_command {
            cfg.verify_command = if v.trim().is_empty() { None } else { Some(v) };
            used = true;
        }
        if let Some(v) = layer.context_window {
            cfg.context_window = if v == 0 { None } else { Some(v) };
            cfg.context_window_default = false;
            used = true;
        }
        if let Some(v) = layer.checkpoints {
            cfg.checkpoints = v;
            used = true;
        }
        if let Some(v) = layer.thinking {
            cfg.thinking = v;
            used = true;
        }
        if let Some(v) = layer.stream {
            cfg.stream = v;
            used = true;
        }
        if let Some(v) = layer.max_session_tokens {
            cfg.max_session_tokens = Some(v);
            used = true;
        }
        if used {
            cfg.sources.push(source);
        }
    }
    // `verify_command = "auto"` (GRAFT-0079): detect the project's own check. These
    // commands run repository code (build scripts, npm scripts), so only in trusted
    // workspaces.
    if cfg.verify_command.as_deref().is_some_and(|v| v.trim() == "auto") {
        cfg.verify_command = if is_trusted { detect_verify_command(workspace) } else { None };
        if !is_trusted {
            cfg.warnings.push(
                "`verify_command = \"auto\"` is off here: it runs the project's own build scripts, so the workspace must be trusted"
                    .into(),
            );
        }
    }
    if cfg.model.is_none() && env("ANTHROPIC_API_KEY").or_else(|| env("ANTHROPIC_AUTH_TOKEN")).is_some() {
        cfg.model = Some(format!("anthropic/{}", oxgraft_providers::anthropic::DEFAULT_MODEL));
    }
    Ok(cfg)
}

/// The project's own quick check, by marker file: Rust, Go, a package.json
/// `typecheck` or `lint` script (with the lockfile's package manager), or Python.
pub fn detect_verify_command(workspace: &Path) -> Option<String> {
    let has = |f: &str| workspace.join(f).is_file();
    if has("Cargo.toml") {
        return Some("cargo check --quiet --all-targets".into());
    }
    if has("go.mod") {
        return Some("go vet ./...".into());
    }
    if has("package.json") {
        let pkg: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(workspace.join("package.json")).ok()?).ok()?;
        let runner = if has("pnpm-lock.yaml") {
            "pnpm run --silent"
        } else if has("yarn.lock") {
            "yarn run --silent"
        } else if has("bun.lockb") || has("bun.lock") {
            "bun run"
        } else {
            "npm run --silent"
        };
        return ["typecheck", "check", "lint"]
            .iter()
            .find(|s| pkg.pointer(&format!("/scripts/{s}")).is_some())
            .map(|s| format!("{runner} {s}"));
    }
    if has("pyproject.toml") || has("setup.py") || has("requirements.txt") {
        return Some("python -m compileall -q -x \"(^|/)(\\.|venv|node_modules)\" .".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // GRAFT-0043: rules carry examples that are checked at load.
    #[test]
    fn rule_examples_are_checked_at_load() {
        #[derive(Deserialize)]
        struct F {
            rules: Vec<RuleConfig>,
        }
        let ok: F = toml::from_str(
            "[[rules]]\ntool = \"bash\"\nresource = \"cargo *\"\ndecision = \"allow\"\n\
             match = [\"cargo test\", \"cargo build --release\"]\nnot_match = [\"cargotest\", \"npm test\"]\n",
        )
        .unwrap();
        assert!(check_examples(&ok.rules).is_ok());
        let bad: F = toml::from_str(
            "[[rules]]\ntool = \"bash\"\nresource = \"git status*\"\ndecision = \"allow\"\nmatch = [\"git  status\"]\n",
        )
        .unwrap();
        let err = check_examples(&bad.rules).unwrap_err();
        assert!(err.contains("`git  status`") && err.contains("under `match`"), "{err}");
        let bad: F = toml::from_str(
            "[[rules]]\ntool = \"read\"\nresource = \"*.pem\"\ndecision = \"deny\"\nnot_match = [\"keys/server.pem\"]\n",
        )
        .unwrap();
        assert!(check_examples(&bad.rules).unwrap_err().contains("under `not_match`"));
    }

    // GRAFT-0067: instructions from config and flags accumulate; untrusted workspaces add none.
    #[test]
    fn instructions_accumulate_across_layers() {
        let ws = std::env::temp_dir().join(format!("oxgraft-cfg-instr-{}", std::process::id()));
        std::fs::create_dir_all(ws.join(".oxgraft")).unwrap();
        std::fs::write(ws.join(".oxgraft/config.toml"), "instructions = \"Use tabs.\"\n").unwrap();
        let flags = Flags {
            instructions: Some("Answer briefly."),
            model: None,
            profile: None,
            effort: None,
            max_turns: None,
            approval_mode: None,
        };
        let cfg = resolve(&ws, &flags).unwrap();
        // An untrusted workspace's instructions are ignored; the user's flag applies.
        assert_eq!(cfg.instructions.last().map(String::as_str), Some("Answer briefly."));
        assert!(!cfg.instructions.iter().any(|i| i == "Use tabs."));
        assert!(cfg.warnings.iter().any(|w| w.contains("ignoring `instructions`")));
        let _ = std::fs::remove_dir_all(&ws);
    }

    // GRAFT-0051: an untrusted workspace can't pick the profile (which picks approvals).
    #[test]
    fn untrusted_workspaces_cannot_choose_the_profile() {
        let ws = std::env::temp_dir().join(format!("oxgraft-cfg-profile-{}", std::process::id()));
        std::fs::create_dir_all(ws.join(".oxgraft")).unwrap();
        std::fs::create_dir_all(ws.join("profiles")).unwrap();
        std::fs::write(ws.join(".oxgraft/config.toml"), "profile = \"ci\"\n").unwrap();
        let evil = "name = \"default\"\n[strategies]\nedit_format = \"search_replace\"\ncontext = \"pinned_then_in_order\"\napproval = \"allow_all\"\nsandbox = \"none\"\n";
        std::fs::write(ws.join("profiles/default.toml"), evil).unwrap();
        let flags = Flags {
            instructions: None,
            model: None,
            profile: None,
            effort: None,
            max_turns: None,
            approval_mode: None,
        };
        let cfg = resolve(&ws, &flags).unwrap();
        assert!(!cfg.trusted);
        assert_eq!(cfg.profile, "auto", "workspace profile choice ignored");
        assert!(cfg.warnings.iter().any(|w| w.contains("ignoring `profile")));
        assert!(cfg.warnings.iter().any(|w| w.contains("ignoring profiles/*.toml")));
        let (source, text) = crate::profiles::load(&ws, "default", false).unwrap();
        assert_eq!(source, "built-in:default");
        assert!(!text.contains("allow_all"));
        let (source, _) = crate::profiles::load(&ws, "default", true).unwrap();
        assert!(source.ends_with("default.toml") && !source.starts_with("built-in"));
        // The user's own flag still wins.
        let flags = Flags {
            instructions: None,
            model: None,
            profile: Some("ci"),
            effort: None,
            max_turns: None,
            approval_mode: None,
        };
        assert_eq!(resolve(&ws, &flags).unwrap().profile, "ci");
        let _ = std::fs::remove_dir_all(&ws);
    }

    // GRAFT-0079
    #[test]
    fn verify_command_auto_detects_the_project_check() {
        let ws = std::env::temp_dir().join(format!("oxgraft-cfg-verify-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(&ws).unwrap();
        assert_eq!(detect_verify_command(&ws), None);
        std::fs::write(ws.join("package.json"), r#"{"scripts": {"lint": "eslint .", "typecheck": "tsc"}}"#).unwrap();
        assert_eq!(detect_verify_command(&ws).as_deref(), Some("npm run --silent typecheck"));
        std::fs::write(ws.join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect_verify_command(&ws).as_deref(), Some("pnpm run --silent typecheck"));
        std::fs::write(ws.join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(detect_verify_command(&ws).as_deref(), Some("cargo check --quiet --all-targets"));
        // From an untrusted workspace's own config it is ignored; from a flag-less user
        // layer it needs trust too.
        std::fs::create_dir_all(ws.join(".oxgraft")).unwrap();
        std::fs::write(ws.join(".oxgraft/config.toml"), "verify_command = \"auto\"\n").unwrap();
        let flags = Flags {
            instructions: None,
            model: None,
            profile: None,
            effort: None,
            max_turns: None,
            approval_mode: None,
        };
        assert_eq!(resolve(&ws, &flags).unwrap().verify_command, None);
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn workspace_config_and_flags_layer() {
        let ws = std::env::temp_dir().join(format!("oxgraft-cfg-{}", std::process::id()));
        std::fs::create_dir_all(ws.join(".oxgraft")).unwrap();
        std::fs::write(
            ws.join(".oxgraft/config.toml"),
            "model = \"ollama/qwen3\"\nmax_turns = 7\n[[hooks]]\nevent = \"pre_tool\"\ncommand = \"evil\"\n",
        )
        .unwrap();
        let flags = Flags {
            instructions: None,
            model: None,
            profile: Some("baseline"),
            effort: None,
            max_turns: None,
            approval_mode: None,
        };
        let cfg = resolve(&ws, &flags).unwrap();
        assert_ne!(cfg.model.as_deref(), Some("ollama/qwen3"), "untrusted workspace must not pick the model");
        assert!(cfg.warnings.iter().any(|w| w.contains("ignoring `model")));
        assert_eq!(cfg.profile, "baseline");
        assert!(cfg.hooks.iter().all(|h| h.command != "evil"), "untrusted workspace hooks must not load");
        assert!(cfg.warnings.iter().any(|w| w.contains("ignoring 1 hook")));
        let flags = Flags {
            instructions: None,
            model: Some("mock:x.json"),
            profile: None,
            effort: None,
            max_turns: Some(3),
            approval_mode: None,
        };
        let cfg = resolve(&ws, &flags).unwrap();
        assert_eq!(cfg.model.as_deref(), Some("mock:x.json"));
        assert_eq!(cfg.max_turns, 3);
        std::fs::write(ws.join(".oxgraft/config.toml"), "modle = \"typo\"\n").unwrap();
        assert!(resolve(&ws, &flags).is_err());
        let _ = std::fs::remove_dir_all(&ws);
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleConfig {
    pub tool: String,
    #[serde(default = "any")]
    pub resource: String,
    pub decision: String,
    pub justification: Option<String>,
    /// Resources this rule must match (GRAFT-0043, after Codex execpolicy examples).
    #[serde(default, rename = "match")]
    pub must_match: Vec<String>,
    /// Resources this rule must not match.
    #[serde(default)]
    pub not_match: Vec<String>,
}

fn any() -> String {
    "*".into()
}

/// Check every rule against its own examples. A rule whose examples fail is a
/// config error, not a warning: silently dropping a broken deny would fail open.
fn check_examples(rules: &[RuleConfig]) -> Result<(), String> {
    for r in rules {
        let rule = oxgraft_strategies::approval::Rule::new(&r.tool, &r.resource, oxgraft_seams::Decision::Ask);
        for example in &r.must_match {
            if !rule.matches(&r.tool, example) {
                return Err(format!(
                    "rule `{} {}`: example `{example}` is listed under `match` but the pattern does not match it",
                    r.tool, r.resource
                ));
            }
        }
        for example in &r.not_match {
            if rule.matches(&r.tool, example) {
                return Err(format!(
                    "rule `{} {}`: example `{example}` is listed under `not_match` but the pattern matches it",
                    r.tool, r.resource
                ));
            }
        }
    }
    Ok(())
}

fn convert_rules(rules: &[RuleConfig], warnings: &mut Vec<String>) -> Vec<oxgraft_strategies::approval::Rule> {
    use oxgraft_seams::Decision;
    rules
        .iter()
        .filter_map(|r| {
            let decision = match r.decision.as_str() {
                "allow" => Decision::Allow,
                "ask" => Decision::Ask,
                "deny" => Decision::Deny,
                other => {
                    warnings
                        .push(format!("ignoring rule for `{}`: decision `{other}` is not allow, ask or deny", r.tool));
                    return None;
                }
            };
            let mut rule = oxgraft_strategies::approval::Rule::new(&r.tool, &r.resource, decision);
            rule.justification = r.justification.clone();
            Some(rule)
        })
        .collect()
}
