//! Per-turn workspace checkpoints in a private git repository (GRAFT-0046, after
//! Cline's and Kilo Code's shadow-git checkpoints; clean-room).
//!
//! The repository uses the workspace as its work tree, so it sees every change
//! (including ones made by shell commands) but never touches the user's own
//! `.git`, index, branches or history. `.gitignore` rules in the workspace apply,
//! plus a few heavy directories excluded by default.
//!
//! Security: the repository lives in the user's data directory, never inside the
//! workspace, so a cloned repo can't plant a git dir with hooks or config of its
//! choosing. Git runs with only Oxgraft's own config: system and global config
//! are ignored (no filters, fsmonitor or hooks from anywhere else), hooks point
//! at an empty directory and fsmonitor is off.

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

/// Where Oxgraft keeps per-user data: `OXGRAFT_DATA_DIR`, else the platform data dir.
pub fn data_dir() -> Option<PathBuf> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(d) = var("OXGRAFT_DATA_DIR") {
        return Some(d);
    }
    if cfg!(windows) {
        return var("LOCALAPPDATA").or_else(|| var("APPDATA")).map(|d| d.join("oxgraft"));
    }
    if cfg!(target_os = "macos") {
        return var("HOME").map(|h| h.join("Library/Application Support/oxgraft"));
    }
    var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share"))).map(|d| d.join("oxgraft"))
}

/// `\\?\C:\x` → `C:\x`: Git for Windows mishandles verbatim paths from `canonicalize`.
pub fn plain_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// Canonicalize the longest existing prefix of `path` and re-append the rest.
fn canonical_prefix(path: &Path) -> PathBuf {
    let mut existing = path.to_path_buf();
    let mut rest = Vec::new();
    while !existing.exists() {
        match (existing.file_name().map(|n| n.to_os_string()), existing.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                existing = parent.to_path_buf();
            }
            _ => return path.to_path_buf(),
        }
    }
    let mut real = existing.canonicalize().unwrap_or(existing);
    for name in rest.into_iter().rev() {
        real.push(name);
    }
    real
}

/// FNV-1a: a stable name for a workspace's checkpoint repo across runs and builds.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

impl Checkpoints {
    /// Whether checkpoints can work in `workspace`, without creating anything.
    pub fn check(workspace: &Path) -> Result<(), String> {
        Self::refuse_dangerous(workspace)?;
        data_dir().ok_or("no data directory (set OXGRAFT_DATA_DIR)")?;
        match Command::new("git").arg("--version").output() {
            Ok(o) if o.status.success() => Ok(()),
            _ => Err("git is not installed".into()),
        }
    }

    fn refuse_dangerous(workspace: &Path) -> Result<(), String> {
        if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            && same(workspace, Path::new(&home))
        {
            return Err("checkpoints are off when the workspace is the home directory".into());
        }
        if workspace.parent().is_none() {
            return Err("checkpoints are off when the workspace is a filesystem root".into());
        }
        Ok(())
    }

    /// The checkpoint repo for `workspace`, under the data dir.
    pub fn location(workspace: &Path) -> Result<PathBuf, String> {
        let root = data_dir().ok_or("no data directory (set OXGRAFT_DATA_DIR)")?;
        let ws = plain_path(&workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf()));
        let name: String = ws
            .file_name()
            .map(|n| n.to_string_lossy().chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(40).collect())
            .unwrap_or_default();
        Ok(root.join("checkpoints").join(format!("{name}-{:016x}.git", fnv(&ws.to_string_lossy()))))
    }

    /// Open (creating if needed) the checkpoint repository for `workspace`.
    pub fn open(workspace: &Path) -> Result<Self, String> {
        Self::refuse_dangerous(workspace)?;
        Self::open_at(workspace, &Self::location(workspace)?)
    }

    /// Open (creating if needed) a checkpoint repository at `git_dir`, which must be
    /// outside `workspace`.
    pub fn open_at(workspace: &Path, git_dir: &Path) -> Result<Self, String> {
        let workspace = plain_path(&workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf()));
        // Compare real paths: symlinks (/var -> /private/var) and Windows short names
        // (RUNNER~1) must not let a git dir inside the workspace look outside it.
        let cp = Self { git_dir: plain_path(&canonical_prefix(git_dir)), workspace };
        if cp.git_dir.starts_with(&cp.workspace) {
            return Err("the checkpoint repository must be outside the workspace".into());
        }
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
        std::fs::create_dir_all(cp.git_dir.join("oxgraft-no-hooks")).map_err(|e| e.to_string())?;
        std::fs::write(cp.git_dir.join("oxgraft-empty.gitconfig"), "").map_err(|e| e.to_string())?;
        Ok(cp)
    }

    fn git(&self, args: &[&str]) -> Result<String, String> {
        let hooks = self.git_dir.join("oxgraft-no-hooks");
        let out = Command::new("git")
            .arg("-c")
            .arg(format!("core.hooksPath={}", hooks.display()))
            .args(["-c", "core.fsmonitor=false", "-c", "core.untrackedCache=false"])
            .arg("--git-dir")
            .arg(&self.git_dir)
            .arg("--work-tree")
            .arg(&self.workspace)
            .args(args)
            .current_dir(&self.workspace)
            // The user's environment must not redirect these commands to their repo,
            // and no system or global config (filters, hooks, aliases) may apply.
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_CONFIG")
            .env_remove("GIT_CONFIG_PARAMETERS")
            .env_remove("GIT_CONFIG_COUNT")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.git_dir.join("oxgraft-empty.gitconfig"))
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
        let unchanged = has_head && self.git(&["diff", "--no-ext-diff", "--cached", "--quiet"]).is_ok();
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
        self.git(&["diff", "--no-ext-diff", "--no-textconv", "--stat", from, to])
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

        let git_dir = std::env::temp_dir().join(format!("oxgraft-cp-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&git_dir);
        let cp = Checkpoints::open_at(&ws, &git_dir).unwrap();
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
        let _ = std::fs::remove_dir_all(&git_dir);
    }

    // SPEC-0004 AC7: a repository can't plant the checkpoint repo or its hooks.
    #[test]
    fn checkpoint_repo_is_never_inside_the_workspace() {
        if !git_available() {
            return;
        }
        let ws = std::env::temp_dir().join(format!("oxgraft-cp-planted-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        // What an attacker would ship: an old-style repo whose config runs a command.
        let planted = ws.join(".oxgraft/checkpoints.git");
        std::fs::create_dir_all(planted.join("hooks")).unwrap();
        std::fs::write(planted.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(planted.join("config"), "[core]\n\tfsmonitor = touch PWNED\n").unwrap();
        std::fs::write(ws.join("a.txt"), "a").unwrap();
        assert!(Checkpoints::open_at(&ws, &planted).is_err(), "a git dir inside the workspace is refused");
        let location = Checkpoints::location(&ws).unwrap();
        assert!(!location.starts_with(&ws), "{}", location.display());

        let git_dir = std::env::temp_dir().join(format!("oxgraft-cp-planted-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&git_dir);
        let cp = Checkpoints::open_at(&ws, &git_dir).unwrap();
        cp.snapshot("start").unwrap();
        assert!(!ws.join("PWNED").exists());
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_dir_all(&git_dir);
    }

    #[test]
    fn verbatim_windows_paths_are_simplified() {
        assert_eq!(plain_path(Path::new(r"\\?\C:\x\y")), PathBuf::from(r"C:\x\y"));
        assert_eq!(plain_path(Path::new(r"\\?\UNC\srv\x")), PathBuf::from(r"\\?\UNC\srv\x"));
        assert_eq!(plain_path(Path::new("/tmp/x")), PathBuf::from("/tmp/x"));
    }
}
