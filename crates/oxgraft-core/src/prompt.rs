//! System prompt and project-instruction discovery (SPEC-0003 step 1, GRAFT-0006).

use std::path::Path;

/// Files harnesses use for project instructions, in priority order.
pub const INSTRUCTION_FILES: &[&str] = &["AGENTS.md", "CLAUDE.md", "GEMINI.md", ".github/copilot-instructions.md"];
const MAX_INSTRUCTIONS_CHARS: usize = 40_000;

const BASE: &str = "You are Oxgraft, an autonomous coding agent working in a local repository through tools.

How to work:
- Explore before changing: use ls, glob, grep and read to understand the relevant code.
- Read a file before editing it. Make focused, minimal changes that match the existing style.
- Verify your work: run the project's build, tests or linters with bash when they exist, and fix what you broke.
- Prefer the dedicated file tools over shell commands for reading and editing files.
- If a tool fails, read the error and adjust; do not repeat the same failing call.
- Do not ask the user questions unless you are truly blocked; make reasonable assumptions and state them.
- When finished, reply with a short summary of what you changed and how you verified it.";

/// Build the system prompt for a workspace.
pub fn system_prompt(workspace: &Path, today: &str) -> String {
    let mut prompt = format!(
        "{BASE}\n\nEnvironment:\n- Workspace: {}\n- OS: {}\n- Date: {today}\n",
        workspace.display(),
        std::env::consts::OS
    );
    if let Some(git) = git_summary(workspace) {
        prompt.push_str(&git);
    }
    let instructions = project_instructions(workspace);
    if !instructions.is_empty() {
        prompt.push_str(
            "\nProject instructions (from the repository; follow them unless they conflict with the user):\n",
        );
        prompt.push_str(&instructions);
    }
    prompt
}

/// Branch and a short `git status` (GRAFT-0013, as Claude Code and Codex give the model repo state up front).
pub fn git_summary(workspace: &Path) -> Option<String> {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(workspace)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string())
    };
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    let status = git(&["status", "--porcelain"]).unwrap_or_default();
    let lines: Vec<&str> = status.lines().collect();
    let mut out = format!("- Git branch: {branch}\n");
    if lines.is_empty() {
        out.push_str("- Git status: clean\n");
    } else {
        out.push_str(&format!("- Git status ({} changed paths):\n", lines.len()));
        for l in lines.iter().take(20) {
            out.push_str(&format!("    {l}\n"));
        }
        if lines.len() > 20 {
            out.push_str(&format!("    … {} more\n", lines.len() - 20));
        }
    }
    Some(out)
}

/// Concatenate the instruction files that exist, resolving `@path` import lines
/// (the Claude Code / Gemini CLI convention) one level deep and skipping duplicates.
pub fn project_instructions(workspace: &Path) -> String {
    let mut seen = Vec::<String>::new();
    let mut out = String::new();
    for name in INSTRUCTION_FILES {
        let Ok(text) = std::fs::read_to_string(workspace.join(name)) else { continue };
        let expanded = expand_imports(workspace, &text, &mut seen);
        if seen.contains(&name.to_string()) || expanded.trim().is_empty() {
            continue;
        }
        seen.push(name.to_string());
        out.push_str(&format!("\n<file path=\"{name}\">\n{}\n</file>\n", expanded.trim()));
    }
    if out.chars().count() > MAX_INSTRUCTIONS_CHARS {
        out = out.chars().take(MAX_INSTRUCTIONS_CHARS).collect::<String>() + "\n… [instructions truncated]";
    }
    out
}

fn expand_imports(workspace: &Path, text: &str, seen: &mut Vec<String>) -> String {
    text.lines()
        .map(|line| {
            let t = line.trim();
            let is_import = t.starts_with('@') && !t.contains(' ') && t.len() > 1;
            if !is_import {
                return line.to_string();
            }
            let target = t[1..].trim_start_matches("./").to_string();
            if seen.contains(&target) {
                return String::new();
            }
            match std::fs::read_to_string(workspace.join(&target)) {
                Ok(content) if !target.contains("..") => {
                    seen.push(target);
                    content
                }
                _ => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `YYYY-MM-DD` for a Unix timestamp (UTC), without a date library.
pub fn civil_date(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    // Howard Hinnant's days_from_civil inverse.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn today() -> String {
    civil_date(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
}

/// Just-in-time instructions (GRAFT-0037, after Gemini CLI): instruction files in the
/// directories between a touched file and the workspace root (exclusive), each shown
/// once per run, appended to the tool result so history stays append-only.
pub fn nested_instructions(
    workspace: &Path,
    file: &Path,
    shown: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Option<String> {
    let root = workspace.canonicalize().ok()?;
    let mut dir = file.parent()?.to_path_buf();
    let mut found = Vec::new();
    while dir.starts_with(&root) && dir != root {
        for name in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
            let candidate = dir.join(name);
            if candidate.is_file() && shown.insert(candidate.clone()) {
                found.push(candidate);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    if found.is_empty() {
        return None;
    }
    let mut out = String::new();
    for path in found.iter().rev() {
        let text: String = std::fs::read_to_string(path).unwrap_or_default().chars().take(8_000).collect();
        let rel = path.strip_prefix(&root).unwrap_or(path).to_string_lossy().replace('\\', "/");
        out.push_str(&format!("\n\n[Instructions from {rel} apply to files in its directory:]\n{}", text.trim()));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(951_782_400), "2000-02-29");
        assert_eq!(civil_date(1_795_564_800), "2026-11-25");
    }

    #[test]
    fn discovers_instructions_and_resolves_imports_once() {
        let dir = std::env::temp_dir().join(format!("oxgraft-prompt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("AGENTS.md"), "Use pnpm.").unwrap();
        std::fs::write(dir.join("CLAUDE.md"), "@AGENTS.md\nClaude extra.").unwrap();
        let text = project_instructions(&dir);
        assert_eq!(text.matches("Use pnpm.").count(), 1, "{text}");
        assert!(text.contains("Claude extra."));
        let prompt = system_prompt(&dir, "2026-11-25");
        assert!(prompt.contains("Date: 2026-11-25"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
