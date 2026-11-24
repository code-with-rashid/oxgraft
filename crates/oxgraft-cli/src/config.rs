//! Config resolution (SPEC-0004): flags → env → workspace → user → defaults.

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub model: Option<String>,
    pub profile: Option<String>,
    pub effort: Option<String>,
    pub max_turns: Option<u32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub model: Option<String>,
    pub profile: String,
    pub effort: Option<String>,
    pub max_turns: u32,
    pub max_tokens: u32,
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

pub fn resolve(workspace: &Path, flags: &Flags<'_>) -> Result<Config, String> {
    let mut layers: Vec<(String, FileConfig)> = Vec::new();
    if let Some(p) = user_config_path()
        && let Some(c) = read(&p)?
    {
        layers.push((p.display().to_string(), c));
    }
    let ws = workspace.join(".oxgraft").join("config.toml");
    if let Some(c) = read(&ws)? {
        layers.push((ws.display().to_string(), c));
    }
    let env_layer = FileConfig {
        model: env("OXGRAFT_MODEL"),
        profile: env("OXGRAFT_PROFILE"),
        effort: env("OXGRAFT_EFFORT"),
        max_turns: env("OXGRAFT_MAX_TURNS").and_then(|v| v.parse().ok()),
        max_tokens: env("OXGRAFT_MAX_TOKENS").and_then(|v| v.parse().ok()),
    };
    layers.push(("environment".into(), env_layer));
    layers.push((
        "flags".into(),
        FileConfig {
            model: flags.model.map(str::to_string),
            profile: flags.profile.map(str::to_string),
            effort: flags.effort.map(str::to_string),
            max_turns: flags.max_turns,
            max_tokens: None,
        },
    ));

    let mut cfg = Config {
        model: None,
        profile: "default".into(),
        effort: Some("high".into()),
        max_turns: 50,
        max_tokens: 16_000,
        sources: Vec::new(),
    };
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

    #[test]
    fn workspace_config_and_flags_layer() {
        let ws = std::env::temp_dir().join(format!("oxgraft-cfg-{}", std::process::id()));
        std::fs::create_dir_all(ws.join(".oxgraft")).unwrap();
        std::fs::write(ws.join(".oxgraft/config.toml"), "model = \"ollama/qwen3\"\nmax_turns = 7\n").unwrap();
        let flags = Flags { model: None, profile: Some("baseline"), effort: None, max_turns: None };
        let cfg = resolve(&ws, &flags).unwrap();
        if std::env::var("OXGRAFT_MODEL").is_err() {
            assert_eq!(cfg.model.as_deref(), Some("ollama/qwen3"));
        }
        assert_eq!(cfg.profile, "baseline");
        let flags = Flags { model: Some("mock:x.json"), profile: None, effort: None, max_turns: Some(3) };
        let cfg = resolve(&ws, &flags).unwrap();
        assert_eq!(cfg.model.as_deref(), Some("mock:x.json"));
        assert_eq!(cfg.max_turns, 3);
        std::fs::write(ws.join(".oxgraft/config.toml"), "modle = \"typo\"\n").unwrap();
        assert!(resolve(&ws, &flags).is_err());
        let _ = std::fs::remove_dir_all(&ws);
    }
}
