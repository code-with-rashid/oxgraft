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
                skips hidden files. Returns workspace-relative paths, sorted."
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
        let mut found: Vec<String> = walker(&base)
            .filter_map(Result::ok)
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter_map(|e| {
                let rel = e.path().strip_prefix(&base).ok()?.to_string_lossy().replace('\\', "/");
                matcher.is_match(&rel).then(|| display(ctx.workspace, e.path()))
            })
            .collect();
        found.sort();
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
                    "glob": {"type": "string", "description": "Only search files matching this glob"}
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
            for (i, line) in text.lines().enumerate() {
                if re.is_match(line) {
                    total += 1;
                    if matches.len() < MAX_MATCHES {
                        let line: String = line.trim_end().chars().take(MAX_MATCH_LINE).collect();
                        matches.push(format!("{shown}:{}: {line}", i + 1));
                    }
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
}
