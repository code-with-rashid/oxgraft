use oxgraft_seams::{EditError, EditFormat, Origin, Provenance, Seam, Strategy, StrategyInfo};

use super::search_replace::parse;

/// SEARCH/REPLACE with whitespace-tolerant matching.
///
/// Tries, in order, and stops at the first level with exactly one match:
/// 1. exact text;
/// 2. line-by-line, ignoring trailing whitespace;
/// 3. line-by-line, ignoring indentation, re-indenting the replacement by
///    the difference between the file's and the SEARCH block's indentation.
///
/// More than one match at a level is an error, never a guess.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tolerant;

static INFO: StrategyInfo = StrategyInfo {
    id: "search_replace_tolerant",
    seam: Seam::EditFormat,
    summary: "SEARCH/REPLACE that also matches when only whitespace or indentation differs; still never guesses.",
    provenance: Provenance {
        origin: Origin::CleanRoom,
        harness: Some("aider"),
        license: Some("Apache-2.0"),
        graft: Some("GRAFT-0015"),
        notes: "Aider and Cline both fall back to looser matching when a model's SEARCH text differs only in whitespace.",
    },
};

impl Strategy for Tolerant {
    fn info(&self) -> &StrategyInfo {
        &INFO
    }
}

fn indent(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

/// Find unique line-window matches of `needle` in `hay` under `eq`.
fn find_lines(hay: &[&str], needle: &[&str], eq: impl Fn(&str, &str) -> bool) -> Vec<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return Vec::new();
    }
    (0..=hay.len() - needle.len()).filter(|&i| needle.iter().enumerate().all(|(j, n)| eq(hay[i + j], n))).collect()
}

fn apply_one(text: &str, search: &str, replace: &str) -> Result<String, EditError> {
    match text.matches(search).count() {
        1 => return Ok(text.replacen(search, replace, 1)),
        n if n > 1 => return Err(EditError::Ambiguous(n)),
        _ => {}
    }
    let trailing_newline = text.ends_with('\n');
    let hay: Vec<&str> = text.lines().collect();
    let needle: Vec<&str> = search.lines().collect();
    let rebuild = |start: usize, replacement: Vec<String>| {
        let mut out: Vec<String> = hay[..start].iter().map(|s| s.to_string()).collect();
        out.extend(replacement);
        out.extend(hay[start + needle.len()..].iter().map(|s| s.to_string()));
        let mut s = out.join("\n");
        if trailing_newline {
            s.push('\n');
        }
        s
    };

    let trimmed = find_lines(&hay, &needle, |a, b| a.trim_end() == b.trim_end());
    match trimmed.len() {
        1 => return Ok(rebuild(trimmed[0], replace.lines().map(str::to_string).collect())),
        n if n > 1 => return Err(EditError::Ambiguous(n)),
        _ => {}
    }

    let loose = find_lines(&hay, &needle, |a, b| a.trim() == b.trim());
    match loose.len() {
        1 => {
            let start = loose[0];
            let file_indent = indent(hay[start]);
            let first_search = needle.iter().find(|l| !l.trim().is_empty()).copied().unwrap_or("");
            let search_indent = indent(first_search);
            let reindented = replace
                .lines()
                .map(|l| {
                    if l.trim().is_empty() {
                        String::new()
                    } else if let Some(rest) = l.strip_prefix(search_indent) {
                        format!("{file_indent}{rest}")
                    } else {
                        format!("{file_indent}{}", l.trim_start())
                    }
                })
                .collect();
            Ok(rebuild(start, reindented))
        }
        0 => Err(EditError::NotFound(search.to_string())),
        n => Err(EditError::Ambiguous(n)),
    }
}

impl EditFormat for Tolerant {
    fn apply(&self, original: &str, edit: &str) -> Result<String, EditError> {
        let mut text = original.to_owned();
        for (search, replace) in parse(edit)? {
            text = apply_one(&text, &search, &replace)?;
        }
        Ok(text)
    }

    fn instructions(&self) -> &'static str {
        "`edit` is one or more SEARCH/REPLACE blocks:\n<<<<<<< SEARCH\nexisting lines\n=======\nreplacement lines\n>>>>>>> REPLACE\nCopy the SEARCH lines from the file; whitespace differences are tolerated but the lines must identify exactly one place. Blocks apply in order."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(s: &str, r: &str) -> String {
        format!("<<<<<<< SEARCH\n{s}\n=======\n{r}\n>>>>>>> REPLACE\n")
    }

    #[test]
    fn exact_match_still_works() {
        assert_eq!(Tolerant.apply("a\nb\n", &block("b", "c")).unwrap(), "a\nc\n");
    }

    #[test]
    fn tolerates_trailing_whitespace() {
        let file = "fn a() {   \n    x();\n}\n";
        let out = Tolerant.apply(file, &block("fn a() {\n    x();", "fn a() {\n    y();")).unwrap();
        assert_eq!(out, "fn a() {\n    y();\n}\n");
    }

    #[test]
    fn reindents_when_indentation_differs() {
        let file = "impl A {\n    fn f() {\n        old();\n    }\n}\n";
        let out = Tolerant.apply(file, &block("fn f() {\n    old();\n}", "fn f() {\n    new();\n}")).unwrap();
        assert_eq!(out, "impl A {\n    fn f() {\n        new();\n    }\n}\n");
    }

    #[test]
    fn never_guesses_between_candidates() {
        let file = "  x\n    x\n";
        assert_eq!(Tolerant.apply(file, &block("x", "y")), Err(EditError::Ambiguous(2)));
        assert!(matches!(Tolerant.apply("a\n", &block("zzz", "y")), Err(EditError::NotFound(_))));
    }
}
