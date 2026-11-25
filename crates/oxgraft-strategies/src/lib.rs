//! Strategies (the "scions"): one module per idea, grouped by seam.
//!
//! Adding a strategy learned from another harness:
//! 1. Open a graft record in `learning/grafts/` (see `.agents/skills/propose-graft`).
//! 2. Add a module under the matching seam folder with full [`Provenance`].
//! 3. Register it in the matching function below.
//! 4. Select it in a profile (`profiles/*.toml`) and evaluate before promoting.
//!
//! [`Provenance`]: oxgraft_seams::Provenance

pub mod approval;
pub mod context;
pub mod edit;
pub mod sandbox;

use oxgraft_seams::{ApprovalPolicy, ContextStrategy, EditFormat, Sandbox, StrategyInfo};

pub fn edit_formats() -> Vec<Box<dyn EditFormat>> {
    vec![Box::new(edit::WholeFile), Box::new(edit::SearchReplace), Box::new(edit::Tolerant), Box::new(edit::ApplyPatch)]
}

pub fn contexts() -> Vec<Box<dyn ContextStrategy>> {
    vec![Box::new(context::PinnedThenInOrder)]
}

pub fn approvals() -> Vec<Box<dyn ApprovalPolicy>> {
    vec![
        Box::new(approval::OrderedRules::default()),
        Box::new(approval::InertReads::default()),
        Box::new(approval::AllowAll),
    ]
}

pub fn sandboxes() -> Vec<Box<dyn Sandbox>> {
    vec![Box::new(sandbox::NoSandbox), Box::new(sandbox::WorkspaceWrite)]
}

/// Every registered strategy's identity and lineage.
pub fn catalog() -> Vec<StrategyInfo> {
    let mut all = Vec::new();
    all.extend(edit_formats().iter().map(|s| s.info().clone()));
    all.extend(contexts().iter().map(|s| s.info().clone()));
    all.extend(approvals().iter().map(|s| s.info().clone()));
    all.extend(sandboxes().iter().map(|s| s.info().clone()));
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxgraft_seams::{Origin, Seam};
    use std::collections::HashSet;

    #[test]
    fn every_seam_has_a_strategy() {
        let seams: HashSet<_> = catalog().iter().map(|i| i.seam).collect();
        for seam in Seam::ALL {
            assert!(seams.contains(&seam), "no strategy registered for {seam}");
        }
    }

    #[test]
    fn ids_are_unique_within_a_seam() {
        let mut seen = HashSet::new();
        for info in catalog() {
            assert!(seen.insert((info.seam, info.id)), "duplicate id {} in {}", info.id, info.seam);
        }
    }

    #[test]
    fn borrowed_ideas_name_their_donor_and_license() {
        for info in catalog() {
            if info.provenance.origin != Origin::Original {
                assert!(info.provenance.harness.is_some(), "{} lacks a donor harness", info.id);
                assert!(info.provenance.license.is_some(), "{} lacks a donor license", info.id);
                assert!(info.provenance.graft.is_some(), "{} lacks a graft record", info.id);
            }
        }
    }
}
