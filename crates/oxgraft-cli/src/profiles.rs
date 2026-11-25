//! Built-in profiles. Kept identical to `profiles/*.toml` in the repo (tested).

pub const BUILTIN: &[(&str, &str)] = &[
    ("default", include_str!("../profiles/default.toml")),
    ("baseline", include_str!("../profiles/baseline.toml")),
    ("ci", include_str!("../profiles/ci.toml")),
    ("tolerant", include_str!("../profiles/tolerant.toml")),
    ("codex-like", include_str!("../profiles/codex-like.toml")),
    ("sandboxed", include_str!("../profiles/sandboxed.toml")),
    ("fewer-prompts", include_str!("../profiles/fewer-prompts.toml")),
];

/// Resolve a profile by path, by name in `<workspace>/profiles/`, or built-in (SPEC-0004 AC5).
/// A workspace's own `profiles/` are used only when the workspace is trusted: a
/// profile picks the approval policy, so a cloned repo must not choose it (GRAFT-0051).
pub fn load(workspace: &std::path::Path, name_or_path: &str, trusted: bool) -> Result<(String, String), String> {
    let as_path = std::path::Path::new(name_or_path);
    if name_or_path.ends_with(".toml") || as_path.components().count() > 1 {
        return std::fs::read_to_string(as_path)
            .map(|t| (name_or_path.to_string(), t))
            .map_err(|e| format!("cannot read profile {name_or_path}: {e}"));
    }
    let local = workspace.join("profiles").join(format!("{name_or_path}.toml"));
    if trusted && let Ok(text) = std::fs::read_to_string(&local) {
        return Ok((local.display().to_string(), text));
    }
    BUILTIN
        .iter()
        .find(|(n, _)| *n == name_or_path)
        .map(|(n, t)| (format!("built-in:{n}"), (*t).to_string()))
        .ok_or_else(|| {
            let names: Vec<_> = BUILTIN.iter().map(|(n, _)| *n).collect();
            format!("unknown profile `{name_or_path}` (built-in: {}; or pass a .toml path)", names.join(", "))
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn builtins_match_repo_profiles() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
        for (name, text) in super::BUILTIN {
            let repo = std::fs::read_to_string(root.join(format!("{name}.toml"))).unwrap();
            assert_eq!(
                repo.replace("\r\n", "\n"),
                text.replace("\r\n", "\n"),
                "profiles/{name}.toml drifted from the built-in copy"
            );
        }
    }
}

/// Model families trained on a particular edit format (GRAFT-0038, after OpenCode's
/// per-model edit tool, but as a table instead of ad-hoc substring checks).
const MODEL_FAMILIES: &[(&str, &str)] = &[
    // (model-name prefix after any vendor/ part, profile)
    ("gpt-5", "codex-like"),
    ("codex", "codex-like"),
    ("o3", "codex-like"),
    ("o4", "codex-like"),
];

/// Resolve the `auto` profile for a model; any other name is returned unchanged.
pub fn resolve_auto(name: &str, model: &str) -> String {
    if name != "auto" {
        return name.to_string();
    }
    let bare = model.rsplit('/').next().unwrap_or(model).to_ascii_lowercase();
    MODEL_FAMILIES
        .iter()
        .find(|(prefix, _)| bare.starts_with(prefix))
        .map(|(_, profile)| (*profile).to_string())
        .unwrap_or_else(|| "default".to_string())
}

#[cfg(test)]
mod auto_tests {
    use super::resolve_auto;

    #[test]
    fn auto_picks_profile_by_model_family() {
        assert_eq!(resolve_auto("auto", "gpt-5.1-codex"), "codex-like");
        assert_eq!(resolve_auto("auto", "openai/gpt-5"), "codex-like");
        assert_eq!(resolve_auto("auto", "claude-opus-5-5"), "default");
        assert_eq!(resolve_auto("auto", "qwen3"), "default");
        assert_eq!(resolve_auto("tolerant", "gpt-5"), "tolerant", "explicit profiles are never overridden");
    }
}
