use std::path::Path;

use globset::GlobBuilder;
use ignore::WalkBuilder;
use oxgraft_seams::{Action, EditFormat, Tool, ToolContext, ToolOutput, ToolSpec};
use regex::Regex;
use serde_json::{Value, json};

use crate::util::{display, looks_binary, opt_str, resolve, str_field, truncate};

const MAX_GLOB: usize = 1_000;
const MAX_MATCHES: usize = 500;
const MAX_MATCH_LINE: usize = 300;

macro_rules! try_tool {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(msg) => return ToolOutput::error(msg),
        }
    };
}

fn walker(root: &Path) -> ignore::Walk {
    WalkBuilder::new(root).hidden(true).git_ignore(true).git_global(false).require_git(false).build()
}

fn glob_matcher(pattern: &str) -> Result<globset::GlobMatcher, String> {
    // A bare pattern like `*.rs` should match at any depth.
    let pattern = if pattern.contains('/') { pattern.to_string() } else { format!("**/{pattern}") };
    GlobBuilder::new(&pattern)
        .literal_separator(true)
        .build()
        .map(|g| g.compile_matcher())
        .map_err(|e| format!("invalid glob `{pattern}`: {e}"))
}

// ------------------------------------------------------------------- glob

#[derive(Debug, Clone, Copy, Default)]
pub struct Glob;

impl Tool for Glob {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "glob".into(),
            description: "Find files by glob pattern (e.g. `**/*.rs`, `src/*.ts`). Respects .gitignore; \
                skips hidden files. Returns workspace-relative paths, most recently modified first."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Glob pattern"},
                    "path": {"type": "string", "description": "Directory to search (default: workspace root)"}
                },
                "required": ["pattern"]
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        Action { tool: "glob".into(), resource: input.get("pattern").and_then(Value::as_str).unwrap_or("").into() }
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let pattern = try_tool!(str_field(input, "pattern"));
        let base = try_tool!(resolve(ctx.workspace, try_tool!(opt_str(input, "path")).unwrap_or(".")));
        let matcher = try_tool!(glob_matcher(pattern));
        // Newest first (GRAFT-0096, as Claude Code's Glob): recently touched files are
        // usually the relevant ones. Ties (and unreadable times) sort by path.
        let mut entries: Vec<(std::time::SystemTime, String)> = walker(&base)
            .filter_map(Result::ok)
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter_map(|e| {
                let rel = e.path().strip_prefix(&base).ok()?.to_string_lossy().replace('\\', "/");
                if !matcher.is_match(&rel) {
                    return None;
                }
                let mtime = e.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(std::time::UNIX_EPOCH);
                Some((mtime, display(ctx.workspace, e.path())))
            })
            .collect();
        entries.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        let mut found: Vec<String> = entries.into_iter().map(|(_, p)| p).collect();
        let total = found.len();
        if total == 0 {
            return ToolOutput::ok(format!("no files match `{pattern}`"));
        }
        found.truncate(MAX_GLOB);
        let mut out = found.join("\n");
        if total > MAX_GLOB {
            out.push_str(&format!("\n… {} more; narrow the pattern", total - MAX_GLOB));
        }
        ToolOutput::ok(truncate(&out, ctx.max_output_chars))
    }
}

// ------------------------------------------------------------------- grep

#[derive(Debug, Clone, Copy, Default)]
pub struct Grep;

impl Tool for Grep {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "grep".into(),
            description: "Search file contents with a regular expression (Rust regex syntax). Respects \
                .gitignore. Returns `path:line: text`. Use `glob` to limit files (e.g. `*.py`)."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Regular expression"},
                    "path": {"type": "string", "description": "Directory or file (default: workspace root)"},
                    "glob": {"type": "string", "description": "Only search files matching this glob"},
                    "context": {"type": "integer", "description": "Lines of context before and after each match (0-5, default 0)"}
                },
                "required": ["pattern"]
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        Action { tool: "grep".into(), resource: input.get("pattern").and_then(Value::as_str).unwrap_or("").into() }
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let pattern = try_tool!(str_field(input, "pattern"));
        let re = try_tool!(Regex::new(pattern).map_err(|e| format!("invalid regex: {e}")));
        let base = try_tool!(resolve(ctx.workspace, try_tool!(opt_str(input, "path")).unwrap_or(".")));
        let filter = match try_tool!(opt_str(input, "glob")) {
            Some(g) => Some(try_tool!(glob_matcher(g))),
            None => None,
        };
        let context = input.get("context").and_then(Value::as_u64).unwrap_or(0).min(5) as usize;
        let mut matches = Vec::new();
        let mut total = 0usize;
        for entry in walker(&base).filter_map(Result::ok) {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            if let Some(f) = &filter {
                let rel = entry.path().strip_prefix(&base).unwrap_or(entry.path()).to_string_lossy().replace('\\', "/");
                let name = entry.file_name().to_string_lossy();
                if !f.is_match(&rel) && !f.is_match(name.as_ref()) {
                    continue;
                }
            }
            // Skip huge files (logs, data dumps) rather than loading them (stability).
            if entry.metadata().map(|m| m.len() > 5 * 1024 * 1024).unwrap_or(false) {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else { continue };
            if looks_binary(&bytes) {
                continue;
            }
            let text = String::from_utf8_lossy(&bytes);
            let shown = display(ctx.workspace, entry.path());
            let lines: Vec<&str> = text.lines().collect();
            // With context (GRAFT-0061, like `grep -C`): `path:N:` marks a match,
            // `path-N-` a context line; `--` separates groups.
            let mut last_shown: Option<usize> = None;
            for (i, line) in lines.iter().enumerate() {
                if !re.is_match(line) {
                    continue;
                }
                total += 1;
                if matches.len() >= MAX_MATCHES {
                    continue;
                }
                let from = i.saturating_sub(context);
                let to = (i + context).min(lines.len().saturating_sub(1));
                if context > 0 && last_shown.is_some_and(|l| from > l + 1) {
                    matches.push("--".to_string());
                }
                for (j, l) in lines.iter().enumerate().take(to + 1).skip(from) {
                    if last_shown.is_some_and(|shown_to| j <= shown_to) {
                        continue;
                    }
                    let l: String = l.trim_end().chars().take(MAX_MATCH_LINE).collect();
                    let is_match = j == i || (j > i && re.is_match(lines[j]));
                    if is_match && j != i {
                        break; // the next match prints itself, with its own context
                    }
                    matches.push(if j == i {
                        format!("{shown}:{}: {l}", j + 1)
                    } else {
                        format!("{shown}-{}- {l}", j + 1)
                    });
                    last_shown = Some(j);
                }
            }
        }
        if total == 0 {
            return ToolOutput::ok(format!("no matches for `{pattern}`"));
        }
        let mut out = matches.join("\n");
        if total > MAX_MATCHES {
            out.push_str(&format!("\n… {} more matches; narrow the pattern or path", total - MAX_MATCHES));
        }
        ToolOutput::ok(truncate(&out, ctx.max_output_chars))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    #[test]
    fn glob_respects_gitignore_and_depth() {
        let f = Fixture::new("glob");
        f.file("src/a.rs", "");
        f.file("src/deep/b.rs", "");
        f.file("target/c.rs", "");
        f.file(".gitignore", "target/\n");
        let out = f.run(&Glob, json!({"pattern": "*.rs"}));
        assert_eq!(out.content, "src/a.rs\nsrc/deep/b.rs");
        let out = f.run(&Glob, json!({"pattern": "src/*.rs"}));
        assert_eq!(out.content, "src/a.rs");
        assert!(f.run(&Glob, json!({"pattern": "*.py"})).content.starts_with("no files"));
    }

    // GRAFT-0096
    #[test]
    fn glob_lists_newest_first() {
        let f = Fixture::new("globmtime");
        f.file("old.rs", "");
        f.file("new.rs", "");
        let old = std::fs::File::options().write(true).open(f.dir.join("old.rs")).unwrap();
        old.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600)).unwrap();
        let out = f.run(&Glob, json!({"pattern": "*.rs"}));
        assert_eq!(out.content, "new.rs\nold.rs");
    }

    #[test]
    fn grep_finds_lines_with_filters() {
        let f = Fixture::new("grep");
        f.file("a.py", "def main():\n    return 1\n");
        f.file("b.rs", "fn main() {}\n");
        let out = f.run(&Grep, json!({"pattern": "main"}));
        assert!(out.content.contains("a.py:1: def main():"));
        assert!(out.content.contains("b.rs:1: fn main() {}"));
        let out = f.run(&Grep, json!({"pattern": "main", "glob": "*.rs"}));
        assert!(!out.content.contains("a.py"));
        assert!(f.run(&Grep, json!({"pattern": "("})).is_error);
    }

    // SPEC-0002 AC14 (GRAFT-0061)
    #[test]
    fn grep_shows_context_lines() {
        let f = Fixture::new("grepctx");
        f.file("a.txt", "one\ntwo\nTARGET three\nfour\nfive\nsix\nseven\nTARGET eight\nTARGET nine\nten\n");
        let out = f.run(&Grep, json!({"pattern": "TARGET", "context": 1}));
        let expect = "a.txt-2- two\na.txt:3: TARGET three\na.txt-4- four\n--\na.txt-7- seven\na.txt:8: TARGET eight\na.txt:9: TARGET nine\na.txt-10- ten";
        assert_eq!(out.content, expect);
        // Matches inside each other's windows all show (review finding).
        f.file("b.txt", "a\nb\nHIT 3\nHIT 4\nc\nd\ne\n");
        let out = f.run(&Grep, json!({"pattern": "HIT", "context": 2, "path": "b.txt"}));
        assert!(out.content.contains(":3: HIT 3") && out.content.contains(":4: HIT 4"), "{}", out.content);
        assert!(out.content.contains("-6- d") && !out.content.contains("-7-"), "{}", out.content);
        let plain = f.run(&Grep, json!({"pattern": "TARGET"}));
        assert_eq!(plain.content.lines().count(), 3);
    }

    // SPEC-0002 AC2: search never follows symlinks out of the workspace.
    #[cfg(unix)]
    #[test]
    fn search_skips_symlinks_out_of_the_workspace() {
        let f = Fixture::new("symlink");
        let outside = std::env::temp_dir().join(format!("oxgraft-outside-{}", std::process::id()));
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), "needle outside").unwrap();
        std::os::unix::fs::symlink(outside.join("secret.txt"), f.dir.join("link.txt")).unwrap();
        std::os::unix::fs::symlink(&outside, f.dir.join("linkdir")).unwrap();
        f.file("inside.txt", "needle inside");
        let out = f.run(&Grep, json!({"pattern": "needle"}));
        assert!(out.content.contains("inside.txt"), "{}", out.content);
        assert!(!out.content.contains("outside"), "{}", out.content);
        let out = f.run(&Glob, json!({"pattern": "*.txt"}));
        assert!(!out.content.contains("secret"), "{}", out.content);
        let _ = std::fs::remove_dir_all(&outside);
    }
}
