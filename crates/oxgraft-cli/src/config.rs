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
    pub stream: bool,
    pub checkpoints: bool,
    pub hooks: Vec<Hook>,
    pub mcp_servers: BTreeMap<String, ServerConfig>,
    pub rules: Vec<oxgraft_strategies::approval::Rule>,
    pub warnings: Vec<String>,
    pub sources: Vec<String>,
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
    pub model: Option<&'a str>,
    pub profile: Option<&'a str>,
    pub effort: Option<&'a str>,
    pub max_turns: Option<u32>,
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
        stream: true,
        checkpoints: true,
        hooks: Vec::new(),
        mcp_servers: BTreeMap::new(),
        rules: Vec::new(),
        warnings: Vec::new(),
        sources: Vec::new(),
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
            used = true;
        }
        if let Some(v) = layer.checkpoints {
            cfg.checkpoints = v;
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
    if cfg.model.is_none() && env("ANTHROPIC_API_KEY").or_else(|| env("ANTHROPIC_AUTH_TOKEN")).is_some() {
        cfg.model = Some(format!("anthropic/{}", oxgraft_providers::anthropic::DEFAULT_MODEL));
    }
    Ok(cfg)
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

    #[test]
    fn workspace_config_and_flags_layer() {
        let ws = std::env::temp_dir().join(format!("oxgraft-cfg-{}", std::process::id()));
        std::fs::create_dir_all(ws.join(".oxgraft")).unwrap();
        std::fs::write(
            ws.join(".oxgraft/config.toml"),
            "model = \"ollama/qwen3\"\nmax_turns = 7\n[[hooks]]\nevent = \"pre_tool\"\ncommand = \"evil\"\n",
        )
        .unwrap();
        let flags = Flags { model: None, profile: Some("baseline"), effort: None, max_turns: None };
        let cfg = resolve(&ws, &flags).unwrap();
        assert_ne!(cfg.model.as_deref(), Some("ollama/qwen3"), "untrusted workspace must not pick the model");
        assert!(cfg.warnings.iter().any(|w| w.contains("ignoring `model")));
        assert_eq!(cfg.profile, "baseline");
        assert!(cfg.hooks.iter().all(|h| h.command != "evil"), "untrusted workspace hooks must not load");
        assert!(cfg.warnings.iter().any(|w| w.contains("ignoring 1 hook")));
        let flags = Flags { model: Some("mock:x.json"), profile: None, effort: None, max_turns: Some(3) };
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
