//! Planning and skills tools.

use std::path::{Path, PathBuf};

use oxgraft_seams::{Action, EditFormat, Tool, ToolContext, ToolOutput, ToolSpec};
use serde_json::{Value, json};

// ------------------------------------------------------------------- todo

/// A visible plan the model maintains for multi-step work (GRAFT-0016: Claude Code's
/// TodoWrite, Codex's update_plan). Stateless: the model sends the whole list each time.
#[derive(Debug, Clone, Copy, Default)]
pub struct Todo;

impl Tool for Todo {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "todo".into(),
            description: "Record your plan for multi-step tasks. Send the complete list every time; keep exactly \
                one item in_progress while working. Use it for tasks with 3+ steps; skip it for trivial ones."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "text": {"type": "string"},
                                "status": {"type": "string", "enum": ["pending", "in_progress", "done"]}
                            },
                            "required": ["text", "status"]
                        }
                    }
                },
                "required": ["items"]
            }),
        }
    }

    fn action(&self, _: &Value) -> Action {
        Action { tool: "todo".into(), resource: String::new() }
    }

    fn run(&self, input: &Value, _: &ToolContext<'_>) -> ToolOutput {
        let Some(items) = input.get("items").and_then(Value::as_array) else {
            return ToolOutput::error("`items` must be an array of {text, status}");
        };
        let mut out = String::new();
        let mut in_progress = 0;
        for item in items {
            let text = item.get("text").and_then(Value::as_str).unwrap_or("").trim();
            let status = item.get("status").and_then(Value::as_str).unwrap_or("pending");
            let mark = match status {
                "done" => "[x]",
                "in_progress" => {
                    in_progress += 1;
                    "[>]"
                }
                _ => "[ ]",
            };
            out.push_str(&format!("{mark} {text}\n"));
        }
        if items.is_empty() {
            out.push_str("(plan cleared)\n");
        }
        if in_progress > 1 {
            out.push_str("note: more than one item is in progress; finish one before starting the next\n");
        }
        ToolOutput::ok(out)
    }
}

// ----------------------------------------------------------------- skills

/// A discovered Agent Skill (`<dir>/<name>/SKILL.md` with name/description frontmatter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
}

/// Skill folders, in priority order: workspace first, then the user's.
pub fn skill_dirs(workspace: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![workspace.join(".agents/skills"), workspace.join(".claude/skills")];
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if let Some(home) = home {
        let home = PathBuf::from(home);
        dirs.push(home.join(".agents/skills"));
        dirs.push(home.join(".claude/skills"));
    }
    dirs
}

/// Discover skills (GRAFT-0017: the open Agent Skills format used by Claude Code, Codex,
/// Gemini CLI, Cursor, Copilot). First definition of a name wins.
pub fn discover_skills(workspace: &Path) -> Vec<SkillInfo> {
    let mut found: Vec<SkillInfo> = Vec::new();
    for dir in skill_dirs(workspace) {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path().join("SKILL.md");
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let Some((name, description)) = frontmatter(&text) else { continue };
            if !found.iter().any(|s| s.name == name) {
                found.push(SkillInfo { name, description, path });
            }
        }
    }
    found
}

fn frontmatter(text: &str) -> Option<(String, String)> {
    let text = text.replace("\r\n", "\n");
    let body = text.strip_prefix("---\n")?;
    let end = body.find("\n---")?;
    let mut name = None;
    let mut description = None;
    for line in body[..end].lines() {
        if let Some(v) = line.strip_prefix("name:") {
            name = Some(v.trim().trim_matches('"').to_string());
        } else if let Some(v) = line.strip_prefix("description:") {
            description = Some(v.trim().trim_matches('"').to_string());
        }
    }
    Some((name.filter(|n| !n.is_empty())?, description.unwrap_or_default()))
}

/// System-prompt section listing available skills (progressive disclosure: names and
/// descriptions only; the model loads a skill's body with the `skill` tool).
pub fn skills_prompt(skills: &[SkillInfo]) -> Option<String> {
    if skills.is_empty() {
        return None;
    }
    let mut out = String::from(
        "Skills: reusable instructions for specific tasks. When a task matches a skill's description, load it \
         with the `skill` tool before starting and follow it.\n",
    );
    for s in skills {
        let desc: String = s.description.chars().take(300).collect();
        out.push_str(&format!("- {}: {desc}\n", s.name));
    }
    Some(out)
}

#[derive(Debug, Clone, Default)]
pub struct Skill {
    pub skills: Vec<SkillInfo>,
}

impl Tool for Skill {
    fn spec(&self, _: &dyn EditFormat) -> ToolSpec {
        ToolSpec {
            name: "skill".into(),
            description: "Load the full instructions of a skill listed in the system prompt, by name.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {"name": {"type": "string", "description": "Skill name"}},
                "required": ["name"]
            }),
        }
    }

    fn action(&self, input: &Value) -> Action {
        Action { tool: "skill".into(), resource: input.get("name").and_then(Value::as_str).unwrap_or("").into() }
    }

    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput {
        let Some(name) = input.get("name").and_then(Value::as_str) else {
            return ToolOutput::error("missing required field `name`");
        };
        let Some(skill) = self.skills.iter().find(|s| s.name == name) else {
            let names: Vec<_> = self.skills.iter().map(|s| s.name.as_str()).collect();
            return ToolOutput::error(format!("no skill named `{name}`; available: {}", names.join(", ")));
        };
        match std::fs::read_to_string(&skill.path) {
            Ok(text) => {
                let dir = skill.path.parent().map(|p| p.display().to_string()).unwrap_or_default();
                ToolOutput::ok(crate::util::truncate(
                    &format!("Skill `{name}` (files relative to {dir}):\n\n{text}"),
                    ctx.max_output_chars,
                ))
            }
            Err(e) => ToolOutput::error(format!("cannot read skill `{name}`: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    #[test]
    fn todo_renders_plan() {
        let f = Fixture::new("todo");
        let out = f.run(
            &Todo,
            json!({"items": [{"text": "a", "status": "done"}, {"text": "b", "status": "in_progress"}, {"text": "c", "status": "pending"}]}),
        );
        assert_eq!(out.content, "[x] a\n[>] b\n[ ] c\n");
        assert!(f.run(&Todo, json!({})).is_error);
    }

    #[test]
    fn discovers_and_loads_workspace_skills() {
        let f = Fixture::new("skills");
        f.file(".agents/skills/deploy/SKILL.md", "---\nname: deploy\ndescription: Ship it safely\n---\n\nSteps...");
        f.file(".claude/skills/deploy/SKILL.md", "---\nname: deploy\ndescription: shadowed\n---\n");
        f.file(".claude/skills/broken/SKILL.md", "no frontmatter");
        let skills: Vec<_> = discover_skills(&f.dir).into_iter().filter(|s| s.path.starts_with(&f.dir)).collect();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].description, "Ship it safely");
        assert!(skills_prompt(&skills).unwrap().contains("- deploy: Ship it safely"));
        let tool = Skill { skills };
        assert!(f.run(&tool, json!({"name": "deploy"})).content.contains("Steps..."));
        assert!(f.run(&tool, json!({"name": "nope"})).is_error);
    }
}
