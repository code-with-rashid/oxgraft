//! Custom commands (GRAFT-0057, after Claude Code's and Gemini CLI's prompt-file
//! commands): `<name>.md` in `.agents/commands/` or `.claude/commands/` (workspace,
//! then home) becomes `/name` in chat and `oxgraft run /name …`. `$ARGUMENTS` in
//! the file is replaced by the rest of the line; YAML front matter is dropped.
//! Commands are prompts, not code, so they need no workspace trust.

use std::path::{Path, PathBuf};

const DIRS: &[&str] = &[".agents/commands", ".claude/commands"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomCommand {
    pub name: String,
    pub path: PathBuf,
    pub description: String,
}

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

/// Commands available in `workspace`. The user's own (home) commands come first,
/// so a repository can't shadow them; files that resolve outside their commands
/// directory (symlinks to secrets) are ignored.
pub fn discover(workspace: &Path) -> Vec<CustomCommand> {
    let mut roots: Vec<PathBuf> = home().filter(|h| h != workspace).into_iter().collect();
    roots.push(workspace.to_path_buf());
    let mut found: Vec<CustomCommand> = Vec::new();
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
                let Some(name) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
                let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
                if !valid || found.iter().any(|c| c.name == name) {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let description = front_matter_field(&text, "description")
                    .or_else(|| body(&text).lines().find(|l| !l.trim().is_empty()).map(|l| l.trim().to_string()))
                    .unwrap_or_default()
                    .chars()
                    .take(80)
                    .collect();
                found.push(CustomCommand { name, path, description });
            }
        }
    }
    found
}

fn front_matter(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    Some((&rest[..end], rest[end + 4..].trim_start_matches(['-', '\r', '\n'])))
}

fn front_matter_field(text: &str, key: &str) -> Option<String> {
    let (fm, _) = front_matter(text)?;
    fm.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == key).then(|| v.trim().trim_matches(['"', '\'']).to_string())
    })
}

fn body(text: &str) -> &str {
    front_matter(text).map_or(text, |(_, b)| b)
}

/// If `line` is `/name args` for a known command, the expanded prompt.
pub fn expand(workspace: &Path, line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix('/')?;
    let (name, args) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let command = discover(workspace).into_iter().find(|c| c.name == name)?;
    let text = std::fs::read_to_string(&command.path).ok()?;
    let body = body(&text).trim();
    let args = args.trim();
    Some(if body.contains("$ARGUMENTS") {
        body.replace("$ARGUMENTS", args)
    } else if args.is_empty() {
        body.to_string()
    } else {
        format!("{body}\n\n{args}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // SPEC-0004 AC11
    #[test]
    fn commands_expand_with_arguments() {
        let ws = std::env::temp_dir().join(format!("oxgraft-cmds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(ws.join(".agents/commands")).unwrap();
        std::fs::create_dir_all(ws.join(".claude/commands")).unwrap();
        std::fs::write(
            ws.join(".agents/commands/review.md"),
            "---\ndescription: Review a file\n---\nReview $ARGUMENTS for bugs. Be brief.\n",
        )
        .unwrap();
        std::fs::write(ws.join(".claude/commands/review.md"), "shadowed\n").unwrap();
        std::fs::write(ws.join(".claude/commands/tests.md"), "Run the tests and fix failures.\n").unwrap();
        std::fs::write(ws.join(".claude/commands/bad name.md"), "x").unwrap();
        let found = discover(&ws);
        let names: Vec<&str> = found.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"review") && names.contains(&"tests") && !names.contains(&"bad name"), "{names:?}");
        assert_eq!(found.iter().find(|c| c.name == "review").unwrap().description, "Review a file");
        assert_eq!(expand(&ws, "/review src/lib.rs").unwrap(), "Review src/lib.rs for bugs. Be brief.");
        assert_eq!(expand(&ws, "/tests").unwrap(), "Run the tests and fix failures.");
        assert_eq!(expand(&ws, "/tests only unit").unwrap(), "Run the tests and fix failures.\n\nonly unit");
        assert!(expand(&ws, "/nope").is_none());
        assert!(expand(&ws, "not a command").is_none());
        #[cfg(unix)]
        {
            let secret = ws.with_extension("secret");
            std::fs::write(&secret, "TOP SECRET").unwrap();
            std::os::unix::fs::symlink(&secret, ws.join(".agents/commands/leak.md")).unwrap();
            assert!(expand(&ws, "/leak").is_none(), "symlinks out of the commands dir are ignored");
            let _ = std::fs::remove_file(&secret);
        }
        let _ = std::fs::remove_dir_all(&ws);
    }
}
