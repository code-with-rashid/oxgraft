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
    /// Size of the last top-level request (input + output tokens): how full the
    /// context is (GRAFT-0099).
    #[serde(default)]
    pub context_tokens: u64,
    /// The same tokens split by model (subagents and `/model` switches use others).
    #[serde(default)]
    pub usage_by_model: std::collections::BTreeMap<String, Usage>,
    #[serde(default)]
    pub changed_files: Vec<UndoRecord>,
    /// How many times the context was compacted, and how many messages were summarised away.
    #[serde(default)]
    pub compactions: u32,
    #[serde(default)]
    pub archived_messages: usize,
    /// Workspace checkpoints taken during the session (GRAFT-0046), oldest first.
    #[serde(default)]
    pub checkpoints: Vec<CheckpointRecord>,
    /// Index in `checkpoints` of the current run's starting state (not saved).
    #[serde(skip)]
    pub run_start: Option<usize>,
    /// The session this one was forked from (GRAFT-0049).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
}

/// Sessions hold prompts, file contents and command output: on Unix, make the
/// file readable by its owner only and the folder private (GRAFT-0095).
#[cfg(unix)]
fn private(file: &Path, dir: Option<&Path>) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))?;
    if let Some(dir) = dir {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn private(_file: &Path, _dir: Option<&Path>) -> std::io::Result<()> {
    Ok(())
}

/// One workspace checkpoint: the state after `label`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub label: String,
    pub commit: String,
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
            usage_by_model: Default::default(),
            context_tokens: 0,
            changed_files: Vec::new(),
            compactions: 0,
            archived_messages: 0,
            checkpoints: Vec::new(),
            forked_from: None,
            run_start: None,
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
        let _ = private(&tmp, path.parent()); // best effort: some filesystems refuse chmod
        std::fs::rename(&tmp, &path)
    }

    pub fn load(workspace: &Path, id: &str) -> std::io::Result<Self> {
        let path = Self::dir(workspace).join(format!("{id}.json"));
        let text = std::fs::read_to_string(&path)
            .map_err(|e| std::io::Error::new(e.kind(), format!("session {id} not found at {}: {e}", path.display())))?;
        serde_json::from_str(&text).map_err(|e| std::io::Error::other(format!("session {id} is corrupt: {e}")))
    }

    /// All sessions in a workspace, newest first.
    /// Delete all but the `keep` newest sessions (and their undo backups) in a
    /// workspace (GRAFT-0074). Files that aren't sessions are left alone. Returns
    /// the ids removed (or that would be, with `dry_run`).
    pub fn prune(workspace: &Path, keep: usize, dry_run: bool) -> std::io::Result<Vec<String>> {
        let dir = Self::dir(workspace);
        let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(Vec::new()) };
        let mut ids: Vec<String> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter(|p| {
                std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str::<Session>(&t).ok()).is_some()
            })
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .collect();
        ids.sort_by(|a, b| b.cmp(a));
        let old: Vec<String> = ids.into_iter().skip(keep).collect();
        if !dry_run {
            for id in &old {
                std::fs::remove_file(dir.join(format!("{id}.json")))?;
                let backups = dir.join(id);
                if backups.is_dir() {
                    std::fs::remove_dir_all(backups)?;
                }
            }
        }
        Ok(old)
    }

    pub fn list(workspace: &Path) -> Vec<Session> {
        let Ok(entries) = std::fs::read_dir(Self::dir(workspace)) else { return Vec::new() };
        let mut sessions: Vec<Session> = entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .filter_map(|t| serde_json::from_str::<Session>(&t).ok())
            // Subagent runs are internal; `last` must mean the user's session.
            .filter(|s| s.profile != "subagent")
            .collect();
        sessions.sort_by(|a, b| b.id.cmp(&a.id));
        sessions
    }

    /// Count tokens used by `model` in this session.
    pub fn add_usage(&mut self, model: &str, usage: Usage) {
        self.usage.add(usage);
        self.usage_by_model.entry(model.to_string()).or_default().add(usage);
    }

    /// A readable Markdown transcript (GRAFT-0066): task, replies, tool calls and
    /// shortened results. Provider-specific blocks (thinking) are left out.
    pub fn to_markdown(&self) -> String {
        use oxgraft_seams::{Block, Role};
        let short = |s: &str, n: usize| -> String {
            let t: String = s.chars().take(n).collect();
            if s.chars().count() > n { format!("{t}\n… ({} more characters)", s.chars().count() - n) } else { t }
        };
        let mut out = format!(
            "# {}\n\nSession `{}` · {} · model `{}` · profile `{}` · {} in / {} out tokens\n",
            if self.title.is_empty() { "Oxgraft session" } else { &self.title },
            self.id,
            self.created,
            self.model,
            self.profile,
            self.usage.input_tokens,
            self.usage.output_tokens
        );
        if let Some(from) = &self.forked_from {
            out.push_str(&format!("\nForked from `{from}`.\n"));
        }
        for m in &self.messages {
            for b in &m.content {
                match (m.role, b) {
                    (Role::User, Block::Text { text }) => out.push_str(&format!("\n## User\n\n{}\n", text.trim())),
                    (Role::Assistant, Block::Text { text }) if !text.trim().is_empty() => {
                        out.push_str(&format!("\n## Oxgraft\n\n{}\n", text.trim()))
                    }
                    (_, Block::ToolUse { name, input, .. }) => {
                        let input = serde_json::to_string(input).unwrap_or_default();
                        out.push_str(&format!("\n**→ {name}** `{}`\n", short(&input, 300).replace('`', "'")));
                    }
                    (_, Block::ToolResult { content, is_error, .. }) => {
                        let mark = if *is_error { "✗" } else { "✓" };
                        out.push_str(&format!(
                            "\n{mark}\n\n```text\n{}\n```\n",
                            short(content.trim(), 1_500).replace("```", "~~~")
                        ));
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// Message counts at which the history can be cut for a fork: right after a
    /// model reply that made no tool calls (a finished answer).
    pub fn fork_points(&self) -> Vec<usize> {
        self.messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.role == oxgraft_seams::Role::Assistant && m.tool_uses().is_empty())
            .map(|(i, _)| i + 1)
            .collect()
    }

    /// A new session continuing from this one's history (GRAFT-0049, after Pi's
    /// session tree and Goose's forks). `keep` = number of messages to copy (a fork
    /// point); `None` copies everything. The original session is not changed.
    pub fn fork(&self, keep: Option<usize>) -> Result<Session, String> {
        let keep = match keep {
            None => self.messages.len(),
            Some(n) if self.fork_points().contains(&n) => n,
            Some(n) => {
                let points: Vec<String> = self.fork_points().iter().map(usize::to_string).collect();
                return Err(format!(
                    "session {} can't be forked at message {n}; fork points (after a finished reply): {}",
                    self.id,
                    if points.is_empty() { "none".into() } else { points.join(", ") }
                ));
            }
        };
        let mut fork = Session::new(&self.model, &self.profile, &self.workspace);
        if fork.id == self.id {
            fork.id.push_str("-f");
        }
        fork.title = format!("fork of {}: {}", self.id, self.title);
        fork.messages = self.messages[..keep].to_vec();
        fork.compactions = self.compactions;
        fork.archived_messages = self.archived_messages;
        fork.forked_from = Some(self.id.clone());
        Ok(fork)
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

#[cfg(test)]
mod fork_tests {
    use super::*;
    use oxgraft_seams::Block;

    // SPEC-0004 AC9
    #[test]
    fn forks_copy_history_up_to_a_finished_reply() {
        let ws = std::env::temp_dir();
        let mut s = Session::new("m", "default", &ws);
        s.title = "original".into();
        s.messages = vec![
            Message::user("task 1"),
            Message::assistant(vec![Block::ToolUse {
                id: "t".into(),
                name: "ls".into(),
                input: serde_json::json!({}),
            }]),
            Message {
                role: oxgraft_seams::Role::User,
                content: vec![Block::ToolResult { tool_use_id: "t".into(), content: "a".into(), is_error: false }],
            },
            Message::assistant(vec![Block::text("answer 1")]),
            Message::user("task 2"),
            Message::assistant(vec![Block::text("answer 2")]),
        ];
        assert_eq!(s.fork_points(), vec![4, 6]);
        let f = s.fork(Some(4)).unwrap();
        assert_eq!(f.messages.len(), 4);
        assert_eq!(f.forked_from.as_deref(), Some(s.id.as_str()));
        assert_ne!(f.id, s.id);
        assert!(f.title.contains("original"));
        assert_eq!(s.fork(None).unwrap().messages.len(), 6);
        let err = s.fork(Some(2)).unwrap_err();
        assert!(err.contains("fork points") && err.contains("4, 6"), "{err}");
        assert_eq!(s.messages.len(), 6, "the original is unchanged");
    }
}

#[cfg(test)]
mod prune_tests {
    use super::*;

    // SPEC-0004 AC16 (GRAFT-0074)
    #[test]
    fn prune_keeps_the_newest_sessions() {
        let ws = std::env::temp_dir().join(format!("oxgraft-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(&ws).unwrap();
        let mut ids = Vec::new();
        for i in 0..4 {
            let mut s = Session::new("m", "default", &ws);
            s.id = format!("20261125-00000{i}-aaaa");
            s.save().unwrap();
            std::fs::create_dir_all(s.backup_dir()).unwrap();
            ids.push(s.id);
        }
        std::fs::write(Session::dir(&ws).join("notes.json"), "not a session").unwrap();
        let would = Session::prune(&ws, 2, true).unwrap();
        assert_eq!(would, vec![ids[1].clone(), ids[0].clone()]);
        assert_eq!(Session::list(&ws).len(), 4, "dry run deletes nothing");
        Session::prune(&ws, 2, false).unwrap();
        let left: Vec<String> = Session::list(&ws).into_iter().map(|s| s.id).collect();
        assert_eq!(left, vec![ids[3].clone(), ids[2].clone()]);
        assert!(!Session::dir(&ws).join(&ids[0]).exists(), "backups go too");
        assert!(Session::dir(&ws).join("notes.json").exists(), "non-session files stay");
        let _ = std::fs::remove_dir_all(&ws);
    }
}

#[cfg(all(test, unix))]
mod permission_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    // GRAFT-0095
    #[test]
    fn session_files_are_private() {
        let ws = std::env::temp_dir().join(format!("oxgraft-perm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(&ws).unwrap();
        let s = Session::new("m", "default", &ws);
        s.save().unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&s.path()), 0o600);
        assert_eq!(mode(&Session::dir(&ws)), 0o700);
        let _ = std::fs::remove_dir_all(&ws);
    }
}
