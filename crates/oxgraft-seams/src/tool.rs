//! The tool seam (SPEC-0002). Tools live in `oxgraft-tools`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;

use crate::model::ToolSpec;
use crate::{Action, EditFormat, Sandbox};

/// What a tool returns to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
}

impl ToolOutput {
    pub fn ok(content: impl Into<String>) -> Self {
        Self { content: content.into(), is_error: false }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self { content: content.into(), is_error: true }
    }
}

/// A file's content before the session first touched it (`None` = did not exist).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub original: Option<Vec<u8>>,
}

/// Everything a tool may use while running.
pub struct ToolContext<'a> {
    pub workspace: &'a Path,
    pub edit_format: &'a dyn EditFormat,
    pub sandbox: &'a dyn Sandbox,
    /// Pre-change snapshots for undo (first write per file wins).
    pub snapshots: &'a Mutex<Vec<FileSnapshot>>,
    /// Maximum characters of output returned to the model.
    pub max_output_chars: usize,
}

impl std::fmt::Debug for ToolContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolContext").field("workspace", &self.workspace).finish_non_exhaustive()
    }
}

impl ToolContext<'_> {
    /// Record a file's current state before the first modification in this session.
    pub fn snapshot(&self, path: &Path) {
        let mut snaps = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        if snaps.iter().any(|s| s.path == path) {
            return;
        }
        let original = std::fs::read(path).ok();
        snaps.push(FileSnapshot { path: path.to_path_buf(), original });
    }
}

pub trait Tool: Send + Sync {
    fn spec(&self, edit_format: &dyn EditFormat) -> ToolSpec;
    /// Describe the call for the approval policy (tool name + resource).
    fn action(&self, input: &Value) -> Action;
    fn run(&self, input: &Value, ctx: &ToolContext<'_>) -> ToolOutput;
}
