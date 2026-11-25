//! Named subagents (GRAFT-0106, after OpenCode's agents and Claude Code's
//! `.claude/agents`): `<name>.md` in `.agents/agents/` or `.claude/agents/`
//! with `description` and optional `tools` in YAML front matter; the body is
//! the agent's instructions. Subagents stay read-only whatever `tools` says.
//! The user's own (home) agents always load and come first; a repository's
//! agents load only when the workspace is trusted, since the model may start
//! them on its own.

use std::path::{Path, PathBuf};

use oxgraft_core::SubagentType;

use crate::commands::{body, front_matter_field, home};

const DIRS: &[&str] = &[".agents/agents", ".claude/agents"];
const READ_ONLY: &[&str] = &["read", "ls", "glob", "grep", "map", "skill"];
const MAX_INSTRUCTIONS: usize = 8_000;

pub fn discover(workspace: &Path, trusted: bool) -> Vec<SubagentType> {
    let mut roots: Vec<PathBuf> = home().filter(|h| h != workspace).into_iter().collect();
    if trusted {
        roots.push(workspace.to_path_buf());
    }
    let mut found: Vec<SubagentType> = Vec::new();
    for root in roots {
        for dir in DIRS {
            let Ok(base) = root.join(dir).canonicalize() else { continue };
            let Ok(entries) = std::fs::read_dir(&base) else { continue };
            let mut paths: Vec<PathBuf> =
                entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "md")).collect();
            paths.sort();
            for path in paths {
                if !path.canonicalize().is_ok_and(|real| real.starts_with(&base) && real.is_file()) {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                let name = front_matter_field(&text, "name").unwrap_or(stem);
                let valid = !name.is_empty()
                    && name.len() <= 64
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
                if !valid || found.iter().any(|a| a.name == name) {
                    continue;
                }
                let instructions: String = body(&text).trim().chars().take(MAX_INSTRUCTIONS).collect();
                if instructions.is_empty() {
                    continue;
                }
                let description: String = front_matter_field(&text, "description")
                    .unwrap_or_else(|| instructions.lines().next().unwrap_or("").to_string())
                    .chars()
                    .take(200)
                    .collect();
                let mut tools: Vec<String> = Vec::new();
                for t in front_matter_field(&text, "tools").unwrap_or_default().split(',') {
                    let t = t.trim().trim_matches(['[', ']', '"', '\'']).to_ascii_lowercase();
                    if READ_ONLY.contains(&t.as_str()) && !tools.contains(&t) {
                        tools.push(t);
                    }
                }
                found.push(SubagentType { name, description, instructions, tools });
            }
        }
    }
    found
}

/// Agent files in the workspace itself (loaded or not), for `doctor`.
pub fn repo_files(workspace: &Path) -> usize {
    DIRS.iter()
        .filter_map(|d| std::fs::read_dir(workspace.join(d)).ok())
        .flat_map(|entries| entries.flatten())
        .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    // GRAFT-0106
    #[test]
    fn agents_load_from_trusted_workspaces_only() {
        let ws = std::env::temp_dir().join(format!("oxgraft-agents-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(ws.join(".claude/agents")).unwrap();
        std::fs::write(
            ws.join(".claude/agents/reviewer.md"),
            "---\ndescription: Reviews diffs\ntools: Read, Grep, Bash, Edit\n---\nYou review code.\n",
        )
        .unwrap();
        std::fs::write(ws.join(".claude/agents/bad name.md"), "x").unwrap();
        std::fs::write(ws.join(".claude/agents/empty.md"), "---\ndescription: nothing\n---\n").unwrap();
        let ws = ws.canonicalize().unwrap();
        let found = discover(&ws, true);
        let r = found.iter().find(|a| a.name == "reviewer").expect("reviewer loaded");
        assert_eq!(r.description, "Reviews diffs");
        assert_eq!(r.instructions, "You review code.");
        assert_eq!(r.tools, vec!["read".to_string(), "grep".to_string()], "write tools are dropped");
        assert!(!found.iter().any(|a| a.name == "bad name" || a.name == "empty"));
        assert!(!discover(&ws, false).iter().any(|a| a.name == "reviewer"), "untrusted repo agents are ignored");
        let _ = std::fs::remove_dir_all(&ws);
    }
}
