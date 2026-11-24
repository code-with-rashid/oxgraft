use oxgraft_seams::{EditError, EditFormat, Origin, Provenance, Seam, Strategy, StrategyInfo};

/// SEARCH/REPLACE blocks: each block names exact existing text and its replacement.
///
/// ```text
/// <<<<<<< SEARCH
/// old lines
/// =======
/// new lines
/// >>>>>>> REPLACE
/// ```
///
/// Each SEARCH text must match exactly once; ambiguity is an error rather
/// than a guess. Fuzzy matching is a candidate future graft.
#[derive(Debug, Clone, Copy, Default)]
pub struct SearchReplace;

static INFO: StrategyInfo = StrategyInfo {
    id: "search_replace",
    seam: Seam::EditFormat,
    summary: "Exact SEARCH/REPLACE blocks; each search text must match exactly once.",
    provenance: Provenance {
        origin: Origin::CleanRoom,
        harness: Some("aider"),
        license: Some("Apache-2.0"),
        graft: Some("GRAFT-0001"),
        notes: "Block format popularised by Aider's diff edit format; re-implemented from its documentation.",
    },
};

const START: &str = "<<<<<<< SEARCH";
const DIVIDER: &str = "=======";
const END: &str = ">>>>>>> REPLACE";

fn parse(edit: &str) -> Result<Vec<(String, String)>, EditError> {
    let mut blocks = Vec::new();
    let mut lines = edit.lines();
    while let Some(line) = lines.next() {
        if line.trim_end() != START {
            continue;
        }
        let mut search = Vec::new();
        let mut divided = false;
        for l in lines.by_ref() {
            if l.trim_end() == DIVIDER {
                divided = true;
                break;
            }
            search.push(l);
        }
        if !divided {
            return Err(EditError::Malformed(format!("missing `{DIVIDER}`")));
        }
        let mut replace = Vec::new();
        let mut ended = false;
        for l in lines.by_ref() {
            if l.trim_end() == END {
                ended = true;
                break;
            }
            replace.push(l);
        }
        if !ended {
            return Err(EditError::Malformed(format!("missing `{END}`")));
        }
        if search.is_empty() {
            return Err(EditError::Malformed("empty SEARCH text".into()));
        }
        blocks.push((search.join("\n"), replace.join("\n")));
    }
    if blocks.is_empty() {
        return Err(EditError::Malformed("no SEARCH/REPLACE blocks".into()));
    }
    Ok(blocks)
}

impl Strategy for SearchReplace {
    fn info(&self) -> &StrategyInfo {
        &INFO
    }
}

impl EditFormat for SearchReplace {
    fn apply(&self, original: &str, edit: &str) -> Result<String, EditError> {
        let mut text = original.to_owned();
        for (search, replace) in parse(edit)? {
            match text.matches(search.as_str()).count() {
                0 => return Err(EditError::NotFound(search)),
                1 => text = text.replacen(search.as_str(), &replace, 1),
                n => return Err(EditError::Ambiguous(n)),
            }
        }
        Ok(text)
    }

    fn instructions(&self) -> &'static str {
        "`edit` is one or more SEARCH/REPLACE blocks:\n<<<<<<< SEARCH\nexact existing lines\n=======\nreplacement lines\n>>>>>>> REPLACE\nThe SEARCH text must match the file exactly (including indentation) and occur exactly once; include enough surrounding lines to make it unique. Blocks apply in order."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(search: &str, replace: &str) -> String {
        format!("{START}\n{search}\n{DIVIDER}\n{replace}\n{END}\n")
    }

    #[test]
    fn replaces_a_unique_match() {
        let out = SearchReplace.apply("fn a() {}\nfn b() {}\n", &block("fn b() {}", "fn c() {}")).unwrap();
        assert_eq!(out, "fn a() {}\nfn c() {}\n");
    }

    #[test]
    fn applies_blocks_in_order() {
        let edit = format!("{}{}", block("one", "two"), block("two", "three"));
        assert_eq!(SearchReplace.apply("one", &edit).unwrap(), "three");
    }

    #[test]
    fn rejects_missing_and_ambiguous_matches() {
        assert!(matches!(SearchReplace.apply("abc", &block("xyz", "q")), Err(EditError::NotFound(_))));
        assert_eq!(SearchReplace.apply("x x", &block("x", "y")), Err(EditError::Ambiguous(2)));
    }

    #[test]
    fn rejects_malformed_edits() {
        assert!(matches!(SearchReplace.apply("a", "no blocks"), Err(EditError::Malformed(_))));
        assert!(matches!(SearchReplace.apply("a", &format!("{START}\na\n")), Err(EditError::Malformed(_))));
    }
}
