//! A persisted conversation (SPEC-0004 AC3).

use std::path::{Path, PathBuf};

use oxgraft_seams::{Message, Usage};
use serde::{Deserialize, Serialize};

/// A file a session changed, and where its original content is backed up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoRecord {
    pub path: PathBuf,
    /// Backup file name inside the session's backup dir; `None` = file did not exist.
    pub backup: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created: String,
    pub model: String,
    pub profile: String,
    pub workspace: PathBuf,
    #[serde(default)]
    pub title: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub usage: Usage,
    #[serde(default)]
    pub changed_files: Vec<UndoRecord>,
    /// How many times the context was compacted, and how many messages were summarised away.
    #[serde(default)]
    pub compactions: u32,
    #[serde(default)]
    pub archived_messages: usize,
}

impl Session {
    pub fn new(model: &str, profile: &str, workspace: &Path) -> Self {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let secs = now.as_secs();
        let id = format!(
            "{}-{:02}{:02}{:02}-{:04x}",
            crate::prompt::civil_date(secs).replace('-', ""),
            (secs / 3600) % 24,
            (secs / 60) % 60,
            secs % 60,
            now.subsec_nanos() & 0xffff
        );
        Self {
            id,
            created: crate::prompt::civil_date(secs),
            model: model.into(),
            profile: profile.into(),
            workspace: workspace.to_path_buf(),
            title: String::new(),
            messages: Vec::new(),
            usage: Usage::default(),
            changed_files: Vec::new(),
            compactions: 0,
            archived_messages: 0,
        }
    }

    /// Directory holding sessions for a workspace.
    pub fn dir(workspace: &Path) -> PathBuf {
        workspace.join(".oxgraft").join("sessions")
    }

    pub fn path(&self) -> PathBuf {
        Self::dir(&self.workspace).join(format!("{}.json", self.id))
    }

    pub fn backup_dir(&self) -> PathBuf {
        Self::dir(&self.workspace).join(&self.id)
    }

    /// Atomic save: write a temp file, then rename over the old one.
    pub fn save(&self) -> std::io::Result<()> {
        let path = self.path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            // Keep sessions out of git without asking the user to edit .gitignore.
            let ignore = parent.parent().map(|p| p.join(".gitignore"));
            if let Some(ignore) = ignore
                && !ignore.exists()
            {
                let _ = std::fs::write(ignore, "*\n");
            }
        }
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &path)
    }

    pub fn load(workspace: &Path, id: &str) -> std::io::Result<Self> {
        let path = Self::dir(workspace).join(format!("{id}.json"));
        let text = std::fs::read_to_string(&path)
            .map_err(|e| std::io::Error::new(e.kind(), format!("session {id} not found at {}: {e}", path.display())))?;
        serde_json::from_str(&text).map_err(|e| std::io::Error::other(format!("session {id} is corrupt: {e}")))
    }

    /// All sessions in a workspace, newest first.
    pub fn list(workspace: &Path) -> Vec<Session> {
        let Ok(entries) = std::fs::read_dir(Self::dir(workspace)) else { return Vec::new() };
        let mut sessions: Vec<Session> = entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .filter_map(|t| serde_json::from_str(&t).ok())
            .collect();
        sessions.sort_by(|a, b| b.id.cmp(&a.id));
        sessions
    }

    /// Make the history valid after a crash or interrupt: every tool call in the last
    /// assistant message must be followed by a result. Returns how many were closed.
    pub fn repair(&mut self) -> usize {
        let Some(last) = self.messages.last() else { return 0 };
        if last.role != oxgraft_seams::Role::Assistant {
            return 0;
        }
        let ids: Vec<String> = last.tool_uses().into_iter().map(|(id, _, _)| id.to_string()).collect();
        if ids.is_empty() {
            return 0;
        }
        let results = ids
            .iter()
            .map(|id| oxgraft_seams::Block::ToolResult {
                tool_use_id: id.clone(),
                content: "Not executed: the previous run was interrupted.".into(),
                is_error: true,
            })
            .collect();
        self.messages.push(Message { role: oxgraft_seams::Role::User, content: results });
        ids.len()
    }

    /// Record a file's original content before its first change in this session.
    pub fn record_original(&mut self, path: &Path, original: Option<&[u8]>) -> std::io::Result<()> {
        if self.changed_files.iter().any(|r| r.path == path) {
            return Ok(());
        }
        let backup = match original {
            Some(bytes) => {
                let name = format!("{}.bak", self.changed_files.len());
                std::fs::create_dir_all(self.backup_dir())?;
                std::fs::write(self.backup_dir().join(&name), bytes)?;
                Some(name)
            }
            None => None,
        };
        self.changed_files.push(UndoRecord { path: path.to_path_buf(), backup });
        Ok(())
    }

    /// Restore every changed file to its pre-session state (SPEC-0004 AC4).
    pub fn undo(&self) -> Vec<String> {
        let mut report = Vec::new();
        for record in self.changed_files.iter().rev() {
            let shown = record.path.display();
            let result = match &record.backup {
                Some(name) => std::fs::read(self.backup_dir().join(name))
                    .and_then(|bytes| std::fs::write(&record.path, bytes))
                    .map(|_| format!("restored {shown}")),
                None => match std::fs::remove_file(&record.path) {
                    Ok(()) => Ok(format!("removed {shown} (created by the session)")),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(format!("{shown} already gone")),
                    Err(e) => Err(e),
                },
            };
            report.push(result.unwrap_or_else(|e| format!("FAILED to restore {shown}: {e}")));
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // SPEC-0004 AC3, AC4
    #[test]
    fn saves_loads_and_undoes() {
        let ws = std::env::temp_dir().join(format!("oxgraft-session-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(&ws).unwrap();
        let mut s = Session::new("mock/x", "default", &ws);
        s.messages.push(Message::user("hello"));
        let existing = ws.join("a.txt");
        std::fs::write(&existing, "old").unwrap();
        s.record_original(&existing, Some(b"old")).unwrap();
        let created = ws.join("new.txt");
        s.record_original(&created, None).unwrap();
        s.save().unwrap();
        std::fs::write(&existing, "new").unwrap();
        std::fs::write(&created, "x").unwrap();

        let loaded = Session::load(&ws, &s.id).unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(Session::list(&ws).len(), 1);
        let report = loaded.undo();
        assert_eq!(report.len(), 2, "{report:?}");
        assert_eq!(std::fs::read_to_string(&existing).unwrap(), "old");
        assert!(!created.exists());
        assert!(ws.join(".oxgraft/.gitignore").exists());
        assert!(Session::load(&ws, "missing").is_err());
        let _ = std::fs::remove_dir_all(&ws);
    }
}
