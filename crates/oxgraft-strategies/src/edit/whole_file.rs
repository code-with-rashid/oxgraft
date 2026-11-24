use oxgraft_seams::{EditError, EditFormat, Provenance, Seam, Strategy, StrategyInfo};

/// The edit is the complete new file content.
#[derive(Debug, Clone, Copy, Default)]
pub struct WholeFile;

static INFO: StrategyInfo = StrategyInfo {
    id: "whole_file",
    seam: Seam::EditFormat,
    summary: "The model writes the complete new file. Simple and robust, but token-heavy on large files.",
    provenance: Provenance::original("Baseline fallback most harnesses offer."),
};

impl Strategy for WholeFile {
    fn info(&self) -> &StrategyInfo {
        &INFO
    }
}

impl EditFormat for WholeFile {
    fn apply(&self, _original: &str, edit: &str) -> Result<String, EditError> {
        Ok(edit.to_owned())
    }

    fn instructions(&self) -> &'static str {
        "`edit` is the complete new content of the file. Include every line, not just the changed ones."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_everything() {
        assert_eq!(WholeFile.apply("old", "new").unwrap(), "new");
    }
}
