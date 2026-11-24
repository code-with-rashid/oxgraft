use std::path::{Component, Path, PathBuf};

use serde_json::Value;

/// Resolve `path` against the workspace and refuse anything outside it,
/// following symlinks for the parts that exist (SPEC-0002 AC2).
pub fn resolve(workspace: &Path, path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("`path` is empty".into());
    }
    let root =
        workspace.canonicalize().map_err(|e| format!("workspace {} is not accessible: {e}", workspace.display()))?;
    let joined = if Path::new(path).is_absolute() { PathBuf::from(path) } else { root.join(path) };
    let normalized = normalize(&joined);

    // Canonicalize the longest existing prefix (resolves symlinks), then re-append the rest.
    let mut existing = normalized.clone();
    let mut rest = Vec::new();
    while !existing.exists() {
        match (existing.file_name().map(|n| n.to_os_string()), existing.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                existing = parent.to_path_buf();
            }
            _ => break,
        }
    }
    let mut real = existing.canonicalize().unwrap_or(existing);
    for name in rest.into_iter().rev() {
        real.push(name);
    }
    if real.starts_with(&root) {
        Ok(real)
    } else {
        Err(format!("`{path}` is outside the workspace ({}); only files inside it can be used", root.display()))
    }
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Path relative to the workspace, with forward slashes, for display.
pub fn display(workspace: &Path, path: &Path) -> String {
    let root = workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf());
    path.strip_prefix(&root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// Keep the head and tail of long output (SPEC-0002 AC3).
pub fn truncate(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();
    if total <= max_chars {
        return text.to_string();
    }
    let head = max_chars * 6 / 10;
    let tail = max_chars - head;
    let head_part: String = text.chars().take(head).collect();
    let tail_part: String = text.chars().skip(total - tail).collect();
    format!("{head_part}\n\n... [{} characters truncated] ...\n\n{tail_part}", total - head - tail)
}

pub fn object(input: &Value) -> Result<&serde_json::Map<String, Value>, String> {
    input
        .as_object()
        .ok_or_else(|| format!("tool input must be a JSON object, got: {}", truncate(&input.to_string(), 200)))
}

pub fn str_field<'a>(input: &'a Value, name: &str) -> Result<&'a str, String> {
    match object(input)?.get(name) {
        Some(Value::String(s)) => Ok(s),
        Some(other) => Err(format!("`{name}` must be a string, got {other}")),
        None => Err(format!("missing required field `{name}`")),
    }
}

pub fn opt_str<'a>(input: &'a Value, name: &str) -> Result<Option<&'a str>, String> {
    match object(input)?.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(other) => Err(format!("`{name}` must be a string, got {other}")),
    }
}

pub fn opt_u64(input: &Value, name: &str) -> Result<Option<u64>, String> {
    match object(input)?.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
            .map(Some)
            .ok_or_else(|| format!("`{name}` must be a non-negative integer, got {v}")),
    }
}

/// Heuristic: a NUL byte in the first 8 KiB means binary.
pub fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|b| *b == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_keeps_head_and_tail() {
        let s = "a".repeat(50) + &"b".repeat(50);
        let t = truncate(&s, 20);
        assert!(t.starts_with("aaaaaaaaaaaa"));
        assert!(t.ends_with("bbbbbbbb"));
        assert!(t.contains("80 characters truncated"));
        assert_eq!(truncate("short", 20), "short");
    }

    #[test]
    fn field_validation() {
        let v = serde_json::json!({"path": 3, "n": "7"});
        assert!(str_field(&v, "path").unwrap_err().contains("must be a string"));
        assert!(str_field(&v, "missing").unwrap_err().contains("missing"));
        assert_eq!(opt_u64(&v, "n").unwrap(), Some(7));
        assert!(object(&serde_json::json!("{bad")).unwrap_err().contains("JSON object"));
    }
}
