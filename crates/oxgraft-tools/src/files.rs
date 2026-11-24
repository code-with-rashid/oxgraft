use oxgraft_seams::{Action, EditFormat, Tool, ToolContext, ToolOutput, ToolSpec};
use serde_json::{Value, json};

use crate::util::{display, looks_binary, opt_str, opt_u64, resolve, str_field, truncate};

const MAX_LINE_CHARS: usize = 2_000;
const DEFAULT_READ_LINES: u64 = 2_000;

fn action(tool: &str, input: &Value, field: &str) -> Action {
    Action { tool: tool.into(), resource: input.get(field).and_then(Value::as_str).unwrap_or("").into() }
}

macro_rules! try_tool {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(msg) => return ToolOutput::error(msg),
        }
    };
}

// ------------------------------------------------------------------- read

#[derive(Debug, Clone, Copy, Default)]
pub struct Read;

impl Tool for Read {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "read".into(),
            description: "Read a text file in the workspace. Returns numbered lines (`line<TAB>text`). \
                Use `offset` (1-based line) and `limit` to page through large files."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path, relative to the workspace"},
                    "offset": {"type": "integer", "description": "First line to return (1-based)", "minimum": 1},
                    "limit": {"type": "integer", "description": "Maximum lines to return (default 2000)", "minimum": 1}
                },
                "required": ["path"]
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        action("read", input, "path")
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let path = try_tool!(str_field(input, "path"));
        let offset = try_tool!(opt_u64(input, "offset")).unwrap_or(1).max(1);
        let limit = try_tool!(opt_u64(input, "limit")).unwrap_or(DEFAULT_READ_LINES).max(1);
        let full = try_tool!(resolve(ctx.workspace, path));
        if full.is_dir() {
            return ToolOutput::error(format!("`{path}` is a directory; use `ls` to list it"));
        }
        let bytes = match std::fs::read(&full) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return ToolOutput::error(format!("`{path}` does not exist"));
            }
            Err(e) => return ToolOutput::error(format!("cannot read `{path}`: {e}")),
        };
        if looks_binary(&bytes) {
            return ToolOutput::error(format!("`{path}` looks like a binary file ({} bytes); not shown", bytes.len()));
        }
        let text = String::from_utf8_lossy(&bytes);
        if text.is_empty() {
            return ToolOutput::ok(format!("(`{path}` is empty)"));
        }
        let lines: Vec<&str> = text.lines().collect();
        let start = (offset - 1) as usize;
        if start >= lines.len() {
            return ToolOutput::error(format!("`{path}` has {} lines; offset {offset} is past the end", lines.len()));
        }
        let end = (start + limit as usize).min(lines.len());
        let mut out = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            let line: String = if line.chars().count() > MAX_LINE_CHARS {
                line.chars().take(MAX_LINE_CHARS).collect::<String>() + " … [line truncated]"
            } else {
                (*line).to_string()
            };
            out.push_str(&format!("{:>6}\t{}\n", start + i + 1, line));
        }
        if end < lines.len() {
            out.push_str(&format!("… {} more lines; continue with offset {}\n", lines.len() - end, end + 1));
        }
        ToolOutput::ok(truncate(&out, ctx.max_output_chars))
    }
}

// ------------------------------------------------------------------ write

#[derive(Debug, Clone, Copy, Default)]
pub struct Write;

impl Tool for Write {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "write".into(),
            description: "Create or overwrite a file in the workspace with `content`. Parent directories are \
                created. Prefer `edit` for changing existing files."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path, relative to the workspace"},
                    "content": {"type": "string", "description": "Complete file content"}
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        action("write", input, "path")
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let path = try_tool!(str_field(input, "path"));
        let content = try_tool!(str_field(input, "content"));
        let full = try_tool!(resolve(ctx.workspace, path));
        if full.is_dir() {
            return ToolOutput::error(format!("`{path}` is a directory"));
        }
        ctx.snapshot(&full);
        if let Some(parent) = full.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            return ToolOutput::error(format!("cannot create directory for `{path}`: {e}"));
        }
        match std::fs::write(&full, content) {
            Ok(()) => ToolOutput::ok(format!(
                "wrote {} bytes ({} lines) to {}",
                content.len(),
                content.lines().count(),
                display(ctx.workspace, &full)
            )),
            Err(e) => ToolOutput::error(format!("cannot write `{path}`: {e}")),
        }
    }
}

// ------------------------------------------------------------------- edit

#[derive(Debug, Clone, Copy, Default)]
pub struct Edit;

impl Tool for Edit {
    fn spec(&self, format: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "edit".into(),
            description: format!(
                "Change an existing file. Read the file first. {}\nOn failure the file is left unchanged.",
                format.instructions()
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path, relative to the workspace"},
                    "edit": {"type": "string", "description": "The edit, in the format described above"}
                },
                "required": ["path", "edit"]
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        action("edit", input, "path")
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let path = try_tool!(str_field(input, "path"));
        let edit = try_tool!(str_field(input, "edit"));
        let full = try_tool!(resolve(ctx.workspace, path));
        let original = match std::fs::read(&full) {
            Ok(b) if looks_binary(&b) => return ToolOutput::error(format!("`{path}` is binary; cannot edit")),
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(_) => return ToolOutput::error(format!("`{path}` does not exist; use `write` to create it")),
        };
        // Models often send \r\n-free edits for CRLF files: normalise, then restore.
        let crlf = original.contains("\r\n");
        let base = if crlf { original.replace("\r\n", "\n") } else { original.clone() };
        let updated = match ctx.edit_format.apply(&base, &edit.replace("\r\n", "\n")) {
            Ok(text) => text,
            Err(e) => return ToolOutput::error(format!("edit failed, `{path}` unchanged: {e}")),
        };
        if updated == base {
            return ToolOutput::error(format!("edit made no change to `{path}`"));
        }
        let updated = if crlf { updated.replace('\n', "\r\n") } else { updated };
        ctx.snapshot(&full);
        match std::fs::write(&full, &updated) {
            Ok(()) => {
                let (before, after) = (base.lines().count(), updated.lines().count());
                ToolOutput::ok(format!("edited {} ({before} → {after} lines)", display(ctx.workspace, &full)))
            }
            Err(e) => ToolOutput::error(format!("cannot write `{path}`: {e}")),
        }
    }
}

// --------------------------------------------------------------------- ls

#[derive(Debug, Clone, Copy, Default)]
pub struct Ls;

const MAX_LS: usize = 1_000;

impl Tool for Ls {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "ls".into(),
            description: "List one directory level in the workspace. Directories end with `/`.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {"path": {"type": "string", "description": "Directory (default: workspace root)"}},
                "required": []
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        action("ls", input, "path")
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let path = try_tool!(opt_str(input, "path")).unwrap_or(".");
        let full = try_tool!(resolve(ctx.workspace, path));
        let entries = match std::fs::read_dir(&full) {
            Ok(e) => e,
            Err(e) => return ToolOutput::error(format!("cannot list `{path}`: {e}")),
        };
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) { format!("{name}/") } else { name }
            })
            .collect();
        names.sort();
        let total = names.len();
        names.truncate(MAX_LS);
        let mut out = names.join("\n");
        if total > MAX_LS {
            out.push_str(&format!("\n… {} more entries", total - MAX_LS));
        }
        if out.is_empty() {
            out = "(empty directory)".into();
        }
        ToolOutput::ok(truncate(&out, ctx.max_output_chars))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    // SPEC-0002 AC1, AC5
    #[test]
    fn read_numbers_lines_and_pages() {
        let f = Fixture::new("read");
        f.file("a.txt", "one\ntwo\nthree\n");
        let out = f.run(&Read, json!({"path": "a.txt", "offset": 2, "limit": 1}));
        assert!(!out.is_error);
        assert!(out.content.contains("     2\ttwo"));
        assert!(out.content.contains("continue with offset 3"));
        assert!(f.run(&Read, json!({"path": "missing.txt"})).content.contains("does not exist"));
        assert!(f.run(&Read, json!({})).is_error);
        std::fs::write(f.dir.join("bin"), [0u8, 1, 2]).unwrap();
        assert!(f.run(&Read, json!({"path": "bin"})).content.contains("binary"));
    }

    // SPEC-0002 AC2
    #[test]
    fn refuses_paths_outside_workspace() {
        let f = Fixture::new("escape");
        for p in ["../outside.txt", "a/../../outside.txt"] {
            let out = f.run(&Write, json!({"path": p, "content": "x"}));
            assert!(out.is_error && out.content.contains("outside the workspace"), "{p}: {}", out.content);
        }
        let abs = std::env::temp_dir().join("oxgraft-definitely-outside.txt");
        let out = f.run(&Read, json!({"path": abs.to_string_lossy()}));
        assert!(out.content.contains("outside the workspace"), "{}", out.content);
    }

    // SPEC-0002 AC6, AC7
    #[test]
    fn edit_applies_format_and_snapshots() {
        let f = Fixture::new("edit");
        f.file("src/lib.rs", "fn a() {}\nfn b() {}\n");
        let edit = "<<<<<<< SEARCH\nfn b() {}\n=======\nfn c() {}\n>>>>>>> REPLACE";
        let out = f.run(&Edit, json!({"path": "src/lib.rs", "edit": edit}));
        assert!(!out.is_error, "{}", out.content);
        assert_eq!(f.read("src/lib.rs"), "fn a() {}\nfn c() {}\n");
        let bad = "<<<<<<< SEARCH\nnope\n=======\nx\n>>>>>>> REPLACE";
        let out = f.run(&Edit, json!({"path": "src/lib.rs", "edit": bad}));
        assert!(out.is_error && out.content.contains("unchanged"));
        assert_eq!(f.read("src/lib.rs"), "fn a() {}\nfn c() {}\n");
        let snaps = f.snapshots.lock().unwrap();
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].original.as_deref(), Some("fn a() {}\nfn b() {}\n".as_bytes()));
    }

    #[test]
    fn edit_preserves_crlf() {
        let f = Fixture::new("crlf");
        f.file("w.txt", "a\r\nb\r\n");
        let edit = "<<<<<<< SEARCH\nb\n=======\nc\n>>>>>>> REPLACE";
        assert!(!f.run(&Edit, json!({"path": "w.txt", "edit": edit})).is_error);
        assert_eq!(f.read("w.txt"), "a\r\nc\r\n");
    }

    #[test]
    fn write_creates_dirs_and_ls_lists() {
        let f = Fixture::new("write");
        assert!(!f.run(&Write, json!({"path": "x/y/z.txt", "content": "hi"})).is_error);
        assert_eq!(f.read("x/y/z.txt"), "hi");
        assert_eq!(f.snapshots.lock().unwrap()[0].original, None);
        let out = f.run(&Ls, json!({"path": "x"}));
        assert_eq!(out.content, "y/");
    }
}
