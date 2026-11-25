use oxgraft_seams::{Action, EditFormat, Tool, ToolContext, ToolOutput, ToolSpec};
use serde_json::{Value, json};

use crate::util::{display, looks_binary, opt_str, opt_u64, resolve, str_field, truncate};

const MAX_LINE_CHARS: usize = 2_000;
const DEFAULT_READ_LINES: u64 = 2_000;
/// Files above these sizes are refused instead of loaded into memory (stability).
const MAX_READ_BYTES: u64 = 50 * 1024 * 1024;
const MAX_EDIT_BYTES: u64 = 10 * 1024 * 1024;

fn too_large(path: &std::path::Path, limit: u64, shown: &str) -> Option<String> {
    let len = std::fs::metadata(path).ok()?.len();
    (len > limit).then(|| {
        format!(
            "`{shown}` is {:.1} MB, over the {} MB limit for this tool; use grep or bash (head, tail, sed -n) to inspect parts of it",
            len as f64 / 1_048_576.0,
            limit / 1_048_576
        )
    })
}

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
        if let Some(msg) = too_large(&full, MAX_READ_BYTES, path) {
            return ToolOutput::error(msg);
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
                "Change an existing file. Read the file first. {}\nTo make several exact replacements in one \
                 call, send `edits: [{{\"old_string\": ..., \"new_string\": ...}}, ...]` instead of `edit`; each is \
                 matched against the original file and all apply or none do.\nOn failure the file is left unchanged.",
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
        // Models trained on other harnesses often send `old_string`/`new_string`
        // (Claude Code, OpenCode). Accept that too rather than failing (GRAFT-0014).
        let legacy = input.get("edit").is_none() && input.get("old_string").is_some();
        // Several exact replacements in one call, each matched against the original
        // file and applied together or not at all (GRAFT-0050, after Pi's multi-edit).
        let batch = input.get("edit").is_none() && input.get("edits").is_some();
        let legacy = legacy && !batch;
        let edit = if legacy || batch { "" } else { try_tool!(str_field(input, "edit")) };
        let full = try_tool!(resolve(ctx.workspace, path));
        if let Some(msg) = too_large(&full, MAX_EDIT_BYTES, path) {
            return ToolOutput::error(msg);
        }
        let original = match std::fs::read(&full) {
            Ok(b) if looks_binary(&b) => return ToolOutput::error(format!("`{path}` is binary; cannot edit")),
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(_) => return ToolOutput::error(format!("`{path}` does not exist; use `write` to create it")),
        };
        // Models often send \r\n-free edits for CRLF files: normalise, then restore.
        let crlf = original.contains("\r\n");
        let base = if crlf { original.replace("\r\n", "\n") } else { original.clone() };
        let result = if batch {
            apply_batch(&base, &input["edits"])
        } else if legacy {
            let old = try_tool!(str_field(input, "old_string")).replace("\r\n", "\n");
            let new = try_tool!(str_field(input, "new_string")).replace("\r\n", "\n");
            let all = input.get("replace_all").and_then(Value::as_bool).unwrap_or(false);
            match base.matches(old.as_str()).count() {
                _ if old.is_empty() => Err("`old_string` is empty".to_string()),
                0 => whitespace_tolerant_replace(&base, &old, &new)
                    .ok_or_else(|| format!("`old_string` not found: {:?}", truncate(&old, 200))),
                1 => Ok(base.replacen(old.as_str(), &new, 1)),
                _ if all => Ok(base.replace(old.as_str(), &new)),
                n => Err(format!("`old_string` matches {n} times; add context to make it unique or set replace_all")),
            }
        } else {
            ctx.edit_format.apply(&base, &edit.replace("\r\n", "\n")).map_err(|e| e.to_string())
        };
        let updated = match result {
            Ok(text) => text,
            Err(e) => {
                let probe = if legacy {
                    input["old_string"].as_str().unwrap_or("")
                } else if batch {
                    ""
                } else {
                    edit
                };
                let hint = nearest_lines(&base, probe).map(|h| format!("\n{h}")).unwrap_or_default();
                return ToolOutput::error(format!("edit failed, `{path}` unchanged: {e}{hint}"));
            }
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

/// Fallback when `old_string` doesn't match exactly (GRAFT-0086, after OpenCode's
/// line-trimmed and indentation-flexible replacers): compare whole lines ignoring
/// trailing whitespace and a uniform extra indentation. Only a unique match is
/// replaced; `new_string` gets the same extra indentation.
fn whitespace_tolerant_replace(base: &str, old: &str, new: &str) -> Option<String> {
    let old_lines: Vec<&str> = old.trim_matches('\n').lines().map(str::trim_end).collect();
    if old_lines.iter().all(|l| l.is_empty()) {
        return None;
    }
    let lines: Vec<&str> = base.split_inclusive('\n').collect();
    let indent = |l: &str| l.len() - l.trim_start().len();
    let mut found: Option<(usize, String)> = None;
    for start in 0..lines.len().saturating_sub(old_lines.len() - 1) {
        let window = &lines[start..start + old_lines.len()];
        // The extra indentation, from the first non-blank old line.
        let k = old_lines.iter().position(|l| !l.is_empty())?;
        let b = window[k].trim_end();
        let o = old_lines[k];
        if b.len() < o.len() || indent(b) < indent(o) {
            continue;
        }
        let prefix = &b[..indent(b) - indent(o)];
        if !prefix.chars().all(|c| c == ' ' || c == '\t') {
            continue;
        }
        let matches = window.iter().zip(&old_lines).all(|(b, o)| {
            let b = b.trim_end();
            if o.is_empty() { b.is_empty() } else { b.strip_prefix(prefix) == Some(o) }
        });
        if matches {
            if found.is_some() {
                return None; // ambiguous: never guess
            }
            found = Some((start, prefix.to_string()));
        }
    }
    let (start, prefix) = found?;
    let end = start + old_lines.len();
    let mut out: String = lines[..start].concat();
    let replacement: Vec<String> =
        new.lines().map(|l| if l.trim().is_empty() { String::new() } else { format!("{prefix}{l}") }).collect();
    out.push_str(&replacement.join("\n"));
    if lines[end - 1].ends_with('\n') && !replacement.is_empty() {
        out.push('\n');
    }
    out.push_str(&lines[end..].concat());
    Some(out)
}

/// Apply `[{old_string, new_string}, ...]` to `base`: every `old_string` must occur
/// exactly once in the original text and the matches must not overlap.
fn apply_batch(base: &str, edits: &Value) -> Result<String, String> {
    let Some(list) = edits.as_array().filter(|l| !l.is_empty()) else {
        return Err("`edits` must be a non-empty list of {old_string, new_string}".into());
    };
    let mut spans = Vec::with_capacity(list.len());
    for (i, e) in list.iter().enumerate() {
        let n = i + 1;
        let (Some(old), Some(new)) = (e["old_string"].as_str(), e["new_string"].as_str()) else {
            return Err(format!("edit {n}: needs string `old_string` and `new_string`"));
        };
        let old = old.replace("\r\n", "\n");
        if old.is_empty() {
            return Err(format!("edit {n}: `old_string` is empty"));
        }
        let Some(start) = base.find(old.as_str()) else {
            return Err(format!("edit {n}: `old_string` not found: {:?}", truncate(&old, 200)));
        };
        // Look again from the next character, so overlapping repeats (`aa` in `aaa`) count.
        let next = start + base[start..].chars().next().map_or(1, char::len_utf8);
        if base[next..].contains(old.as_str()) {
            return Err(format!("edit {n}: `old_string` matches more than once; add context to make it unique"));
        }
        spans.push((start, start + old.len(), new.replace("\r\n", "\n"), n));
    }
    spans.sort_by_key(|s| s.0);
    for pair in spans.windows(2) {
        if pair[1].0 < pair[0].1 {
            return Err(format!("edits {} and {} overlap; merge them into one", pair[0].3, pair[1].3));
        }
    }
    let mut out = String::with_capacity(base.len());
    let mut at = 0;
    for (start, end, new, _) in &spans {
        out.push_str(&base[at..*start]);
        out.push_str(new);
        at = *end;
    }
    out.push_str(&base[at..]);
    Ok(out)
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
    fn edit_accepts_old_new_string_inputs() {
        let f = Fixture::new("legacy");
        f.file("a.txt", "x = 1\ny = 1\n");
        let out = f.run(&Edit, json!({"path": "a.txt", "old_string": "y = 1", "new_string": "y = 2"}));
        assert!(!out.is_error, "{}", out.content);
        assert_eq!(f.read("a.txt"), "x = 1\ny = 2\n");
        let out = f.run(&Edit, json!({"path": "a.txt", "old_string": "= ", "new_string": "=="}));
        assert!(out.content.contains("matches 2 times"));
        let out = f.run(&Edit, json!({"path": "a.txt", "old_string": "= ", "new_string": "=", "replace_all": true}));
        assert!(!out.is_error);
        assert_eq!(f.read("a.txt"), "x =1\ny =2\n");
    }

    // SPEC-0002 AC9 (GRAFT-0050): batched edits match the original and are all-or-nothing.
    #[test]
    fn batched_edits_apply_together_or_not_at_all() {
        let f = Fixture::new("batch");
        f.file("a.rs", "fn a() { 1 }\nfn b() { 2 }\nfn c() { 3 }\n");
        // The second edit's text only exists in the original, not after the first edit:
        // matching against the original (not sequentially) is what makes this work.
        let out = f.run(
            &Edit,
            json!({"path": "a.rs", "edits": [
                {"old_string": "fn c() { 3 }", "new_string": "fn c() { 30 }"},
                {"old_string": "fn a() { 1 }", "new_string": "fn a() { 10 }\nfn c() { 3 }"},
            ]}),
        );
        assert!(!out.is_error, "{}", out.content);
        assert_eq!(f.read("a.rs"), "fn a() { 10 }\nfn c() { 3 }\nfn b() { 2 }\nfn c() { 30 }\n");

        let before = f.read("a.rs");
        for (edits, expect) in [
            (
                json!([{"old_string": "fn b() { 2 }", "new_string": "x"}, {"old_string": "missing", "new_string": "y"}]),
                "edit 2: `old_string` not found",
            ),
            (json!([{"old_string": "() {", "new_string": "x"}]), "matches more than once"),
            (
                json!([{"old_string": "fn b()", "new_string": "x"}, {"old_string": "b() { 2", "new_string": "y"}]),
                "overlap",
            ),
            (json!([]), "non-empty"),
            (json!([{"old_string": "fn b()"}]), "needs string"),
        ] {
            let out = f.run(&Edit, json!({"path": "a.rs", "edits": edits}));
            assert!(out.is_error && out.content.contains(expect), "{expect}: {}", out.content);
            assert_eq!(f.read("a.rs"), before, "a failed batch changes nothing");
        }
        // Overlapping repeats count as more than one match (`aa` occurs twice in `aaa`).
        f.file("b.txt", "aaa\n");
        let out = f.run(&Edit, json!({"path": "b.txt", "edits": [{"old_string": "aa", "new_string": "x"}]}));
        assert!(out.is_error && out.content.contains("matches more than once"), "{}", out.content);
    }

    // GRAFT-0086: whitespace-tolerant fallback for old_string.
    #[test]
    fn old_string_tolerates_trailing_space_and_indentation() {
        let f = Fixture::new("tolerant-old");
        f.file("a.py", "class A:\n    def f(self):   \n        return 1\n\n    def g(self):\n        return 2\n");
        // Model sent the method without its class indentation and without trailing spaces.
        let out = f.run(&Edit, json!({"path": "a.py", "old_string": "def f(self):\n    return 1", "new_string": "def f(self):\n    return 10"}));
        assert!(!out.is_error, "{}", out.content);
        assert_eq!(
            f.read("a.py"),
            "class A:\n    def f(self):\n        return 10\n\n    def g(self):\n        return 2\n"
        );
        // Ambiguous tolerant matches are refused.
        f.file("b.py", "  x = 1\n    x = 1\n");
        let out = f.run(&Edit, json!({"path": "b.py", "old_string": "x = 1", "new_string": "x = 2"}));
        assert!(out.is_error, "{}", out.content);
        // Different text still fails.
        let out = f.run(&Edit, json!({"path": "a.py", "old_string": "def h(self):", "new_string": "x"}));
        assert!(out.is_error && out.content.contains("not found"));
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

#[cfg(test)]
mod limits {
    use super::*;
    use crate::testing::Fixture;

    #[test]
    fn huge_files_are_refused_not_loaded() {
        let f = Fixture::new("huge");
        let file = std::fs::File::create(f.dir.join("big.log")).unwrap();
        file.set_len(MAX_EDIT_BYTES + 1).unwrap(); // sparse: no real disk use
        let out = f.run(&Edit, json!({"path": "big.log", "edit": "x"}));
        assert!(out.is_error && out.content.contains("MB limit"), "{}", out.content);
    }
}

/// "Did you mean" hint for a failed edit (GRAFT-0035, after Aider and goose): find the
/// first line of the edit's search text that exists in the file (ignoring whitespace)
/// and show it with surrounding lines and numbers.
fn nearest_lines(file: &str, edit: &str) -> Option<String> {
    let is_marker = |l: &str| {
        let t = l.trim();
        t.starts_with("<<<<<<<")
            || t.starts_with("=======")
            || t.starts_with(">>>>>>>")
            || t.starts_with("***")
            || t.starts_with("@@")
    };
    let lines: Vec<&str> = file.lines().collect();
    for candidate in edit.lines().filter(|l| !is_marker(l)) {
        let wanted = candidate.trim_start_matches(['+', '-', ' ']).trim();
        if wanted.len() < 4 {
            continue;
        }
        if let Some(i) = lines.iter().position(|l| l.trim() == wanted) {
            let start = i.saturating_sub(2);
            let end = (i + 3).min(lines.len());
            let shown: Vec<String> = (start..end).map(|n| format!("{:>6}\t{}", n + 1, lines[n])).collect();
            return Some(format!("Closest match in the file (copy these lines exactly):\n{}", shown.join("\n")));
        }
    }
    None
}

#[cfg(test)]
mod hints {
    use super::*;
    use crate::testing::Fixture;

    #[test]
    fn failed_edits_show_the_nearest_real_lines() {
        let f = Fixture::new("hint");
        f.file("a.py", "def f():\n    return 1\n\ndef g():\n    return 2\n");
        let edit = "<<<<<<< SEARCH\ndef g():\n  return 2\n=======\ndef g():\n  return 3\n>>>>>>> REPLACE";
        let out = f.run(&Edit, json!({"path": "a.py", "edit": edit}));
        assert!(out.is_error);
        assert!(out.content.contains("Closest match") && out.content.contains("     4\tdef g():"), "{}", out.content);
    }
}
