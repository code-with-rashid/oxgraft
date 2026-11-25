//! `map`: a ranked outline of the repository under a size budget (GRAFT-0053,
//! after Aider's repo map; clean-room and much simpler: regex definitions instead
//! of tree-sitter, and reference counts instead of PageRank).

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use oxgraft_seams::{Action, EditFormat, Tool, ToolContext, ToolOutput, ToolSpec};
use regex::Regex;
use serde_json::{Value, json};

use crate::util::{display, looks_binary, resolve};

const DEFAULT_CHARS: usize = 6_000;
const MAX_CHARS: usize = 40_000;
const MAX_FILES: usize = 3_000;
const MAX_FILE_BYTES: u64 = 1_000_000;
const MAX_DEFS_PER_FILE: usize = 40;

#[derive(Debug, Clone, Copy, Default)]
pub struct RepoMap;

/// (extensions, definition regex). Group `name` is the defined identifier.
fn languages() -> &'static [(&'static [&'static str], Regex)] {
    static LANGS: OnceLock<Vec<(&'static [&'static str], Regex)>> = OnceLock::new();
    LANGS.get_or_init(|| {
        let re = |s: &str| Regex::new(s).expect("valid definition regex");
        vec![
            (
                &["rs"][..],
                re(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+|const\s+|unsafe\s+)*(?:fn|struct|enum|trait|type|mod|union)\s+(?P<name>[A-Za-z_]\w*)|^\s*impl(?:<[^>]*>)?\s+(?:[\w:<>, ]+\s+for\s+)?(?P<impl>[A-Za-z_][\w:]*)"),
            ),
            (&["py"][..], re(r"^(?: {0,4})(?:async\s+)?(?:def|class)\s+(?P<name>[A-Za-z_]\w*)")),
            (
                &["js", "jsx", "ts", "tsx", "mjs", "cjs"][..],
                re(r"^\s*(?:export\s+)?(?:default\s+)?(?:declare\s+)?(?:abstract\s+)?(?:async\s+)?(?:function\*?|class|interface|type|enum)\s+(?P<name>[A-Za-z_$][\w$]*)|^\s*export\s+(?:const|let|var)\s+(?P<export>[A-Za-z_$][\w$]*)"),
            ),
            (&["go"][..], re(r"^(?:func\s+(?:\([^)]*\)\s*)?|type\s+)(?P<name>[A-Za-z_]\w*)")),
            (
                &["java", "kt", "kts", "cs", "swift", "scala"][..],
                re(r"^\s*(?:(?:public|private|protected|internal|static|final|abstract|open|sealed|data|override|partial|async)\s+)*(?:class|interface|enum|record|object|struct|fun|func|trait)\s+(?P<name>[A-Za-z_]\w*)"),
            ),
            (&["rb"][..], re(r"^\s*(?:def|class|module)\s+(?:self\.)?(?P<name>[A-Za-z_]\w*[?!]?)")),
            (&["php"][..], re(r"^\s*(?:(?:public|private|protected|static|abstract|final)\s+)*(?:function|class|interface|trait)\s+(?P<name>[A-Za-z_]\w*)")),
        ]
    })
}

struct FileMap {
    path: String,
    defs: Vec<(String, String)>, // (name, signature line)
    words: HashSet<String>,
}

fn identifiers(text: &str) -> HashSet<String> {
    static WORD: OnceLock<Regex> = OnceLock::new();
    let word = WORD.get_or_init(|| Regex::new(r"[A-Za-z_][A-Za-z0-9_]{2,}").expect("valid"));
    word.find_iter(text).map(|m| m.as_str().to_string()).collect()
}

/// Build the outline text for `base`, ranked and cut to `budget` characters.
pub fn outline(workspace: &std::path::Path, base: &std::path::Path, budget: usize) -> String {
    let mut files = Vec::new();
    let mut skipped = 0usize;
    let walker =
        ignore::WalkBuilder::new(base).hidden(true).git_ignore(true).git_global(false).require_git(false).build();
    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let Some((_, re)) = languages().iter().find(|(exts, _)| exts.contains(&ext.as_str())) else { continue };
        if files.len() >= MAX_FILES || entry.metadata().map(|m| m.len() > MAX_FILE_BYTES).unwrap_or(true) {
            skipped += 1;
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else { continue };
        if looks_binary(&bytes) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        let mut defs = Vec::new();
        for line in text.lines() {
            // Rust unit tests sit in a trailing `mod tests`; they are noise in an outline.
            if ext == "rs" && line.trim_start().starts_with("mod tests") {
                break;
            }
            if let Some(c) = re.captures(line) {
                let name = c.name("name").or_else(|| c.name("impl")).or_else(|| c.name("export"));
                if let Some(name) = name {
                    let sig = line.trim().trim_end_matches('{').trim_end().chars().take(140).collect();
                    defs.push((name.as_str().rsplit("::").next().unwrap_or("").to_string(), sig));
                }
            }
        }
        if defs.is_empty() {
            continue;
        }
        files.push(FileMap { path: display(workspace, path), defs, words: identifiers(&text) });
    }

    // Rank: a file matters when other files use the names it defines.
    let mut used_in: HashMap<&str, usize> = HashMap::new();
    for f in &files {
        for w in &f.words {
            *used_in.entry(w.as_str()).or_default() += 1;
        }
    }
    let mut ranked: Vec<(usize, &FileMap)> = files
        .iter()
        .map(|f| {
            let names: HashSet<&str> = f.defs.iter().map(|(n, _)| n.as_str()).collect();
            let score = names.iter().map(|n| used_in.get(n).copied().unwrap_or(1).saturating_sub(1)).sum();
            (score, f)
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));

    let mut out = String::new();
    let mut shown = 0;
    for (_, f) in &ranked {
        let mut block = format!("{}\n", f.path);
        for (_, sig) in f.defs.iter().take(MAX_DEFS_PER_FILE) {
            block.push_str(&format!("  {sig}\n"));
        }
        if f.defs.len() > MAX_DEFS_PER_FILE {
            block.push_str(&format!("  … {} more\n", f.defs.len() - MAX_DEFS_PER_FILE));
        }
        if out.len() + block.len() > budget {
            break;
        }
        out.push_str(&block);
        shown += 1;
    }
    if out.is_empty() {
        return "No source files with recognised definitions found.".into();
    }
    if shown < ranked.len() {
        out.push_str(&format!(
            "[{shown} of {} files shown, most-referenced first; raise max_chars or map a subdirectory for more]\n",
            ranked.len()
        ));
    }
    if skipped > 0 {
        out.push_str(&format!("[{skipped} large or excess files skipped]\n"));
    }
    out
}

impl Tool for RepoMap {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "map".into(),
            description: "Outline of the repository: source files with their top-level definitions (functions, \
                types, classes), files whose names are used most elsewhere first, cut to a size budget. Use it \
                first in an unfamiliar codebase, then `read` the files that matter."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory to map (default: workspace root)"},
                    "max_chars": {"type": "integer", "description": "Size budget (default 6000, max 40000)"}
                },
                "required": []
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        Action { tool: "map".into(), resource: input.get("path").and_then(Value::as_str).unwrap_or(".").into() }
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let path = input.get("path").and_then(Value::as_str).unwrap_or(".");
        let base = match resolve(ctx.workspace, path) {
            Ok(b) if b.is_dir() => b,
            Ok(_) => return ToolOutput::error(format!("`{path}` is not a directory")),
            Err(e) => return ToolOutput::error(e),
        };
        let budget = input
            .get("max_chars")
            .and_then(Value::as_u64)
            .map_or(DEFAULT_CHARS, |n| (n as usize).clamp(500, MAX_CHARS));
        ToolOutput::ok(outline(ctx.workspace, &base, budget))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    // SPEC-0002 AC10
    #[test]
    fn maps_definitions_ranked_by_use() {
        let f = Fixture::new("map");
        f.file(
            "src/core.rs",
            "pub struct Engine {}\nimpl Engine {\n    pub fn start(&self) {}\n}\nfn helper() {}\n#[cfg(test)]\nmod tests {\n    fn hidden_test() {}\n}\n",
        );
        f.file("src/a.rs", "use crate::core::Engine;\nfn a() { Engine {}.start(); }\n");
        f.file("src/b.rs", "fn b() { let e = Engine {}; }\n");
        f.file("web/app.ts", "export function render() {}\nexport const VERSION = 1;\nclass View {}\n");
        f.file("tool.py", "class Runner:\n    def run(self):\n        pass\n\ndef main():\n    pass\n");
        f.file("README.md", "# not code\n");
        let out = f.run(&RepoMap, json!({}));
        assert!(!out.is_error, "{}", out.content);
        let text = out.content;
        assert!(text.starts_with("src/core.rs"), "most-referenced file first:\n{text}");
        for expect in [
            "pub struct Engine",
            "impl Engine",
            "pub fn start(&self)",
            "export function render()",
            "export const VERSION",
            "class Runner:",
            "def run(self):",
            "def main():",
        ] {
            assert!(text.contains(expect), "missing {expect}:\n{text}");
        }
        assert!(!text.contains("README"));
        assert!(!text.contains("hidden_test"), "test modules are skipped");

        let small = f.run(&RepoMap, json!({"max_chars": 500}));
        assert!(small.content.len() < 700, "{}", small.content);
        let sub = f.run(&RepoMap, json!({"path": "web"}));
        assert!(sub.content.contains("app.ts") && !sub.content.contains("core.rs"));
        assert!(f.run(&RepoMap, json!({"path": "tool.py"})).is_error);
        assert!(f.run(&RepoMap, json!({"path": "../"})).is_error);
    }
}
