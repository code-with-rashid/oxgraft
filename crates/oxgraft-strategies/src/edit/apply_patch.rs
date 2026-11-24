use oxgraft_seams::{EditError, EditFormat, Origin, Provenance, Seam, Strategy, StrategyInfo};

/// Codex-style patch hunks for one file.
///
/// ```text
/// *** Begin Patch
/// *** Update File: src/lib.rs
/// @@ fn main
///  context line
/// -removed line
/// +added line
/// *** End Patch
/// ```
///
/// The envelope lines are optional (the `edit` tool already names the file).
/// Each hunk's old text (context + removed lines) must match exactly one place,
/// searching after the `@@` anchor line when one is given; trailing whitespace is ignored.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApplyPatch;

static INFO: StrategyInfo = StrategyInfo {
    id: "apply_patch",
    seam: Seam::EditFormat,
    summary: "Codex-style patch hunks (@@ anchor, ' ' context, '-' removed, '+' added lines).",
    provenance: Provenance {
        origin: Origin::CleanRoom,
        harness: Some("codex"),
        license: Some("Apache-2.0"),
        graft: Some("GRAFT-0024"),
        notes: "The apply_patch format Codex models are trained to emit; re-implemented from its documented grammar.",
    },
};

impl Strategy for ApplyPatch {
    fn info(&self) -> &StrategyInfo {
        &INFO
    }
}

struct Hunk {
    anchor: Option<String>,
    old: Vec<String>,
    new: Vec<String>,
}

fn parse(edit: &str) -> Result<Vec<Hunk>, EditError> {
    let mut hunks = Vec::new();
    let mut current: Option<Hunk> = None;
    for line in edit.lines() {
        if line.starts_with("*** ") {
            if line.starts_with("*** Add File") || line.starts_with("*** Delete File") {
                return Err(EditError::Malformed(
                    "use the `write` tool to add files; only Update hunks apply here".into(),
                ));
            }
            continue; // Begin/End Patch, Update File, Move to, End of File
        }
        if let Some(rest) = line.strip_prefix("@@") {
            if let Some(h) = current.take()
                && (!h.old.is_empty() || !h.new.is_empty())
            {
                hunks.push(h);
            }
            let anchor = rest.trim();
            current = Some(Hunk {
                anchor: (!anchor.is_empty()).then(|| anchor.to_string()),
                old: Vec::new(),
                new: Vec::new(),
            });
            continue;
        }
        let hunk = current.get_or_insert(Hunk { anchor: None, old: Vec::new(), new: Vec::new() });
        match line.chars().next() {
            Some('+') => hunk.new.push(line[1..].to_string()),
            Some('-') => hunk.old.push(line[1..].to_string()),
            Some(' ') => {
                hunk.old.push(line[1..].to_string());
                hunk.new.push(line[1..].to_string());
            }
            None => {
                // A bare empty line inside a hunk is an empty context line.
                hunk.old.push(String::new());
                hunk.new.push(String::new());
            }
            Some(_) => return Err(EditError::Malformed(format!("unexpected patch line: {line:?}"))),
        }
    }
    if let Some(h) = current
        && (!h.old.is_empty() || !h.new.is_empty())
    {
        hunks.push(h);
    }
    if hunks.is_empty() {
        return Err(EditError::Malformed("no hunks found".into()));
    }
    Ok(hunks)
}

fn apply_hunk(lines: &mut Vec<String>, hunk: &Hunk) -> Result<(), EditError> {
    let start = match &hunk.anchor {
        Some(a) => lines
            .iter()
            .position(|l| l.trim() == a.trim() || l.contains(a.trim()))
            .map(|i| i + 1)
            .ok_or_else(|| EditError::NotFound(format!("@@ {a}")))?,
        None => 0,
    };
    if hunk.old.is_empty() {
        // Pure insertion: after the anchor, or at the end of the file.
        let at = if hunk.anchor.is_some() { start } else { lines.len() };
        lines.splice(at..at, hunk.new.iter().cloned());
        return Ok(());
    }
    // Trailing blank context lines are often dropped by models; match without them too.
    let eq = |a: &str, b: &str| a.trim_end() == b.trim_end();
    let n = hunk.old.len();
    let found: Vec<usize> = (start..=lines.len().saturating_sub(n))
        .filter(|&i| i + n <= lines.len() && hunk.old.iter().enumerate().all(|(j, o)| eq(&lines[i + j], o)))
        .collect();
    match found.as_slice() {
        [i] => {
            lines.splice(*i..*i + n, hunk.new.iter().cloned());
            Ok(())
        }
        [] => Err(EditError::NotFound(hunk.old.join("\n"))),
        many => Err(EditError::Ambiguous(many.len())),
    }
}

impl EditFormat for ApplyPatch {
    fn apply(&self, original: &str, edit: &str) -> Result<String, EditError> {
        let trailing_newline = original.ends_with('\n') || original.is_empty();
        let mut lines: Vec<String> = original.lines().map(str::to_string).collect();
        for hunk in parse(edit)? {
            apply_hunk(&mut lines, &hunk)?;
        }
        let mut out = lines.join("\n");
        if trailing_newline && !out.is_empty() {
            out.push('\n');
        }
        Ok(out)
    }

    fn instructions(&self) -> &'static str {
        "`edit` is a patch in apply_patch format (file header lines are optional):\n*** Begin Patch\n*** Update File: <path>\n@@ <a line just above the change, optional>\n unchanged context line (leading space)\n-removed line\n+added line\n*** End Patch\nInclude 1-3 lines of context so each hunk matches exactly one place. Use several @@ hunks for several changes."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "fn a() {\n    1\n}\n\nfn b() {\n    1\n}\n";

    #[test]
    fn applies_hunks_with_envelope_and_anchor() {
        let patch = "*** Begin Patch\n*** Update File: x.rs\n@@ fn b() {\n-    1\n+    2\n*** End Patch";
        assert_eq!(ApplyPatch.apply(FILE, patch).unwrap(), "fn a() {\n    1\n}\n\nfn b() {\n    2\n}\n");
    }

    #[test]
    fn context_disambiguates_and_ambiguity_is_an_error() {
        let patch = " fn a() {\n-    1\n+    10\n";
        assert!(ApplyPatch.apply(FILE, patch).unwrap().starts_with("fn a() {\n    10\n"));
        assert_eq!(ApplyPatch.apply(FILE, "-    1\n+    2\n"), Err(EditError::Ambiguous(2)));
    }

    #[test]
    fn insertion_and_errors() {
        let out = ApplyPatch.apply("a\nb\n", "@@ a\n+inserted\n").unwrap();
        assert_eq!(out, "a\ninserted\nb\n");
        assert!(matches!(ApplyPatch.apply("a\n", "-zzz\n+y\n"), Err(EditError::NotFound(_))));
        assert!(matches!(ApplyPatch.apply("a\n", "*** Add File: n.rs\n+x\n"), Err(EditError::Malformed(_))));
        assert!(matches!(ApplyPatch.apply("a\n", "garbage line\n"), Err(EditError::Malformed(_))));
    }
}
