//! Built-in profiles. Kept identical to `profiles/*.toml` in the repo (tested).

pub const BUILTIN: &[(&str, &str)] = &[
    ("default", include_str!("../profiles/default.toml")),
    ("baseline", include_str!("../profiles/baseline.toml")),
    ("ci", include_str!("../profiles/ci.toml")),
    ("tolerant", include_str!("../profiles/tolerant.toml")),
];

/// Resolve a profile by path, by name in `<workspace>/profiles/`, or built-in (SPEC-0004 AC5).
pub fn load(workspace: &std::path::Path, name_or_path: &str) -> Result<(String, String), String> {
    let as_path = std::path::Path::new(name_or_path);
    if name_or_path.ends_with(".toml") || as_path.components().count() > 1 {
        return std::fs::read_to_string(as_path)
            .map(|t| (name_or_path.to_string(), t))
            .map_err(|e| format!("cannot read profile {name_or_path}: {e}"));
    }
    let local = workspace.join("profiles").join(format!("{name_or_path}.toml"));
    if let Ok(text) = std::fs::read_to_string(&local) {
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
