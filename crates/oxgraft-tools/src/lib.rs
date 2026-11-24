//! Built-in tools (SPEC-0002). Small and sharp: read, write, edit, ls, glob, grep, bash.

mod bash;
mod extra;
mod files;
mod search;
pub mod util;

use oxgraft_seams::Tool;

pub use bash::Bash;
pub use extra::{Skill, SkillInfo, Todo, discover_skills, skill_dirs, skills_prompt};
pub use files::{Edit, Ls, Read, Write};
pub use search::{Glob, Grep};

/// Default cap on characters returned to the model per tool call.
pub const DEFAULT_MAX_OUTPUT: usize = 30_000;

/// Every built-in tool, in the order they are advertised.
pub fn builtin() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(Read),
        Box::new(Write),
        Box::new(Edit),
        Box::new(Ls),
        Box::new(Glob),
        Box::new(Grep),
        Box::new(Bash),
        Box::new(Todo),
    ]
}

/// Built-in tools plus the `skill` tool when skills are installed, and the
/// system-prompt section that advertises them.
pub fn for_workspace(workspace: &std::path::Path) -> (Vec<Box<dyn Tool>>, Option<String>) {
    let mut tools = builtin();
    let skills = discover_skills(workspace);
    let prompt = skills_prompt(&skills);
    if !skills.is_empty() {
        tools.push(Box::new(Skill { skills }));
    }
    (tools, prompt)
}

#[cfg(test)]
pub(crate) mod testing {
    use std::path::PathBuf;
    use std::sync::Mutex;

    use oxgraft_seams::{FileSnapshot, Tool, ToolContext, ToolOutput};
    use oxgraft_strategies::edit::SearchReplace;
    use oxgraft_strategies::sandbox::NoSandbox;
    use serde_json::Value;

    pub struct Fixture {
        pub dir: PathBuf,
        pub snapshots: Mutex<Vec<FileSnapshot>>,
        pub cancel: std::sync::atomic::AtomicBool,
    }

    impl Fixture {
        pub fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("oxgraft-tools-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self { dir, snapshots: Mutex::new(Vec::new()), cancel: std::sync::atomic::AtomicBool::new(false) }
        }

        pub fn file(&self, rel: &str, content: &str) {
            let p = self.dir.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content).unwrap();
        }

        pub fn read(&self, rel: &str) -> String {
            std::fs::read_to_string(self.dir.join(rel)).unwrap()
        }

        pub fn run(&self, tool: &dyn Tool, input: Value) -> ToolOutput {
            let ctx = ToolContext {
                workspace: &self.dir,
                edit_format: &SearchReplace,
                sandbox: &NoSandbox,
                snapshots: &self.snapshots,
                max_output_chars: 2_000,
                cancel: &self.cancel,
            };
            tool.run(&input, &ctx)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxgraft_strategies::edit::SearchReplace;

    #[test]
    fn tool_names_are_unique_and_schemas_are_objects() {
        let mut names = std::collections::HashSet::new();
        for tool in builtin() {
            let spec = tool.spec(&SearchReplace);
            assert!(names.insert(spec.name.clone()), "duplicate tool {}", spec.name);
            assert_eq!(spec.input_schema["type"], "object", "{}", spec.name);
            assert!(!spec.description.is_empty());
        }
    }
}
