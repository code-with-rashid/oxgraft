//! Per-turn workspace checkpoints in a private git repository (GRAFT-0046, after
//! Cline's and Kilo Code's shadow-git checkpoints; clean-room).
//!
//! The repository lives in `.oxgraft/checkpoints.git` and uses the workspace as its
//! work tree, so it sees every change (including ones made by shell commands) but
//! never touches the user's own `.git`, index, branches or history. `.gitignore`
//! rules in the workspace apply, plus a few heavy directories excluded by default.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Directories never snapshotted, in addition to the workspace's `.gitignore`.
const EXCLUDES: &[&str] =
    &[".oxgraft/", "node_modules/", "target/", ".venv/", "venv/", "__pycache__/", "dist/", "build/", ".next/"];

#[derive(Debug, Clone)]
pub struct Checkpoints {
    git_dir: PathBuf,
    workspace: PathBuf,
}

impl Checkpoints {
    /// Open (creating if needed) the checkpoint repository for `workspace`.
    pub fn open(workspace: &Path) -> Result<Self, String> {
        if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            && same(workspace, Path::new(&home))
        {
            return Err("checkpoints are off when the workspace is the home directory".into());
        }
        if workspace.parent().is_none() {
            return Err("checkpoints are off when the workspace is a filesystem root".into());
        }
        let cp =
            Self { git_dir: workspace.join(".oxgraft").join("checkpoints.git"), workspace: workspace.to_path_buf() };
        if !cp.git_dir.join("HEAD").is_file() {
            std::fs::create_dir_all(&cp.git_dir).map_err(|e| format!("cannot create checkpoint repo: {e}"))?;
            cp.git(&["init", "-q"])?;
            for (key, value) in [
                ("user.name", "oxgraft"),
                ("user.email", "checkpoints@oxgraft.invalid"),
                ("core.autocrlf", "false"),
                ("commit.gpgsign", "false"),
                ("gc.auto", "0"),
            ] {
                cp.git(&["config", key, value])?;
            }
            let info = cp.git_dir.join("info");
            std::fs::create_dir_all(&info).map_err(|e| e.to_string())?;
            std::fs::write(info.join("exclude"), EXCLUDES.join("\n") + "\n").map_err(|e| e.to_string())?;
        }
        Ok(cp)
    }

    fn git(&self, args: &[&str]) -> Result<String, String> {
        let out = Command::new("git")
            .arg("--git-dir")
            .arg(&self.git_dir)
            .arg("--work-tree")
            .arg(&self.workspace)
            .args(args)
            .current_dir(&self.workspace)
            // The user's environment must not redirect these commands to their repo.
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|e| format!("git is not available: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            Err(format!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
        }
    }

    /// Record the current state of the workspace. Returns the commit id; if nothing
    /// changed since the last checkpoint, returns that checkpoint's id.
    pub fn snapshot(&self, message: &str) -> Result<String, String> {
        self.git(&["add", "-A", "--ignore-errors", "."])?;
        let has_head = self.git(&["rev-parse", "-q", "--verify", "HEAD"]).is_ok();
        let unchanged = has_head && self.git(&["diff", "--cached", "--quiet"]).is_ok();
        if !unchanged {
            self.git(&["commit", "-q", "--no-verify", "--allow-empty", "-m", message])?;
        }
        self.git(&["rev-parse", "HEAD"])
    }

    /// Put the workspace back to `commit`. Takes a safety checkpoint first, so the
    /// restore itself can be undone; returns that checkpoint's id. Files the
    /// checkpoints never saw (ignored ones) are left alone.
    pub fn restore(&self, commit: &str) -> Result<String, String> {
        self.git(&["rev-parse", "-q", "--verify", &format!("{commit}^{{commit}}")])
            .map_err(|_| format!("no checkpoint `{commit}`"))?;
        let safety = self.snapshot("before restore")?;
        self.git(&["reset", "-q", "--hard", commit])?;
        Ok(safety)
    }

    /// `git diff --stat` between two checkpoints.
    pub fn diff_stat(&self, from: &str, to: &str) -> Result<String, String> {
        self.git(&["diff", "--stat", from, to])
    }
}

fn same(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_available() -> bool {
        Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success())
    }

    // SPEC-0004 AC7
    #[test]
    fn snapshots_and_restores_without_touching_the_users_repo() {
        if !git_available() {
            eprintln!("git not installed; skipping");
            return;
        }
        let ws = std::env::temp_dir().join(format!("oxgraft-cp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(ws.join("src")).unwrap();
        std::fs::create_dir_all(ws.join("node_modules/x")).unwrap();
        std::fs::write(ws.join("src/a.txt"), "one\n").unwrap();
        std::fs::write(ws.join("src/old.txt"), "old\n").unwrap();
        std::fs::write(ws.join(".gitignore"), "secret.log\n").unwrap();
        std::fs::write(ws.join("secret.log"), "keep me\n").unwrap();
        std::fs::write(ws.join("node_modules/x/big.js"), "x").unwrap();

        let cp = Checkpoints::open(&ws).unwrap();
        let base = cp.snapshot("turn 0").unwrap();
        assert_eq!(cp.snapshot("turn 0 again").unwrap(), base, "no change, no new checkpoint");

        // Changes made any way (edits, new files, deletes) are captured.
        std::fs::write(ws.join("src/a.txt"), "two\n").unwrap();
        std::fs::write(ws.join("src/new.txt"), "new\n").unwrap();
        std::fs::remove_file(ws.join("src/old.txt")).unwrap();
        let after = cp.snapshot("turn 1").unwrap();
        assert_ne!(after, base);
        assert!(cp.diff_stat(&base, &after).unwrap().contains("a.txt"));

        let safety = cp.restore(&base).unwrap();
        assert_eq!(std::fs::read_to_string(ws.join("src/a.txt")).unwrap(), "one\n");
        assert!(!ws.join("src/new.txt").exists(), "files added after the checkpoint are removed");
        assert!(ws.join("src/old.txt").exists(), "deleted files come back");
        assert_eq!(std::fs::read_to_string(ws.join("secret.log")).unwrap(), "keep me\n", "ignored files untouched");
        assert!(ws.join("node_modules/x/big.js").exists());
        assert!(!ws.join(".git").exists(), "the user's repo is never created or touched");

        // The restore can itself be undone.
        cp.restore(&safety).unwrap();
        assert_eq!(std::fs::read_to_string(ws.join("src/a.txt")).unwrap(), "two\n");
        assert!(cp.restore("nonexistent").is_err());
        let _ = std::fs::remove_dir_all(&ws);
    }
}
