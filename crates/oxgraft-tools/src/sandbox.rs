//! OS-level confinement for shell commands (GRAFT-0025).
//! Linux: bubblewrap. macOS: Seatbelt (`sandbox-exec`). Elsewhere: unavailable.

use std::path::{Path, PathBuf};
use std::process::Command;

fn find_in_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|p| p.join(name)).find(|p| p.is_file()))
}

/// Build a command that runs `command` with writes limited to the workspace (plus
/// `writable`) and, unless `network`, no network access.
pub fn confined(command: &str, workspace: &Path, writable: &[String], network: bool) -> Result<Command, String> {
    let ws = workspace.canonicalize().map_err(|e| format!("workspace: {e}"))?;
    if cfg!(target_os = "linux") {
        let bwrap = find_in_path("bwrap")
            .ok_or("bubblewrap (`bwrap`) is not installed; install it or use sandbox = \"none\"")?;
        // Probe once: user namespaces can be disabled (e.g. some CI kernels / AppArmor).
        let probe = Command::new(&bwrap)
            .args(["--ro-bind", "/", "/", "--dev", "/dev", "--die-with-parent", "true"])
            .output()
            .map_err(|e| format!("bwrap failed to start: {e}"))?;
        if !probe.status.success() {
            return Err(format!(
                "bwrap cannot create a sandbox here: {}",
                String::from_utf8_lossy(&probe.stderr).trim()
            ));
        }
        let mut c = Command::new(bwrap);
        // A private PID namespace: /proc shows only sandboxed processes, so a command
        // can't read the harness's environment through /proc/<pid>/environ.
        c.args(["--ro-bind", "/", "/", "--dev", "/dev", "--unshare-pid", "--proc", "/proc", "--tmpfs", "/tmp"]);
        let ws_s = ws.to_string_lossy().to_string();
        c.args(["--bind", &ws_s, &ws_s]);
        for w in writable {
            c.args(["--bind-try", w, w]);
        }
        if !network {
            c.arg("--unshare-net");
        }
        c.args(["--die-with-parent", "--chdir", &ws_s, "--", "sh", "-c", command]);
        return Ok(c);
    }
    if cfg!(target_os = "macos") {
        let exec = Path::new("/usr/bin/sandbox-exec");
        if !exec.exists() {
            return Err("sandbox-exec is not available".into());
        }
        let quote = |p: &str| p.replace('\\', "\\\\").replace('"', "\\\"");
        let mut allow = format!("(subpath \"{}\")", quote(&ws.to_string_lossy()));
        for w in writable {
            allow.push_str(&format!(" (subpath \"{}\")", quote(w)));
        }
        let mut profile = format!(
            "(version 1)(allow default)(deny file-write*)\
             (allow file-write* {allow} (subpath \"/private/tmp\") (subpath \"/private/var/folders\") \
             (literal \"/dev/null\") (literal \"/dev/tty\") (literal \"/dev/stdout\") (literal \"/dev/stderr\"))"
        );
        if !network {
            profile.push_str("(deny network*)(allow network* (local unix))");
        }
        let mut c = Command::new(exec);
        c.args(["-p", &profile, "sh", "-c", command]);
        return Ok(c);
    }
    Err("no OS sandbox is supported on this platform (Linux: bubblewrap, macOS: Seatbelt)".into())
}

/// Check that the OS sandbox can actually run a command here.
pub fn check(workspace: &Path) -> Result<(), String> {
    let mut c = confined("true", workspace, &[], false)?;
    let out = c.current_dir(workspace).output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn confines_writes_to_the_workspace_when_available() {
        let ws = std::env::temp_dir().join(format!("oxgraft-sbx-{}", std::process::id()));
        std::fs::create_dir_all(&ws).unwrap();
        let outside = dirs_outside();
        let cmd = format!("echo in > inside.txt; echo out > '{}' 2>/dev/null; echo done", outside.display());
        let mut c = match confined(&cmd, &ws, &[], false) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("skipping: {e}");
                return;
            }
        };
        let out = c.current_dir(&ws).output().unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).contains("done"), "{out:?}");
        assert!(ws.join("inside.txt").exists(), "workspace must stay writable");
        assert!(!outside.exists(), "write outside the workspace must be blocked");
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn dirs_outside() -> PathBuf {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/var/tmp"));
        home.join(format!("oxgraft-sandbox-escape-{}", std::process::id()))
    }

    #[cfg(windows)]
    #[test]
    fn unavailable_on_windows() {
        assert!(confined("echo", &std::env::temp_dir(), &[], false).is_err());
    }
}
