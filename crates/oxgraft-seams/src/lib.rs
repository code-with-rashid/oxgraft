//! Seams: the stable capability traits of the Oxgraft harness (the "rootstock").
//!
//! Every coding harness makes a choice about the same few problems: how edits
//! are expressed, what goes into context, what needs approval, how commands
//! are confined. Each problem is a trait here. Each harness's answer becomes a
//! *strategy* in `oxgraft-strategies`, tagged with where the idea came from.
//!
//! This crate must stay small and dependency-light. Changing a trait is an
//! architecture decision: write an ADR in `docs/adr/` first.

use std::fmt;

use serde::Serialize;

pub mod model;
pub mod tool;

pub use model::{
    Block, Completion, CompletionRequest, ErrorKind, Message, Provider, ProviderError, Role, StopReason, ToolSpec,
    Usage,
};
pub use tool::{FileSnapshot, Tool, ToolContext, ToolOutput};

/// A capability axis on which harnesses differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Seam {
    EditFormat,
    Context,
    Approval,
    Sandbox,
}

impl Seam {
    pub const ALL: [Seam; 4] = [Seam::EditFormat, Seam::Context, Seam::Approval, Seam::Sandbox];

    /// The key used in profiles, the event stream, and `learning/capability-matrix.md`.
    pub fn as_str(self) -> &'static str {
        match self {
            Seam::EditFormat => "edit_format",
            Seam::Context => "context",
            Seam::Approval => "approval",
            Seam::Sandbox => "sandbox",
        }
    }
}

impl fmt::Display for Seam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How a strategy relates to another harness's code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Our own design.
    Original,
    /// Code adapted from a permissively licensed harness (attribution in `NOTICE`).
    Adapted,
    /// Re-implemented from behaviour or docs only; no source copied.
    CleanRoom,
}

/// Where an idea came from. Required on every strategy; checked by tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Provenance {
    pub origin: Origin,
    /// Donor harness id from `learning/watch.toml`, e.g. `"aider"`.
    pub harness: Option<&'static str>,
    /// SPDX license of the donor, e.g. `"Apache-2.0"`.
    pub license: Option<&'static str>,
    /// Graft record that introduced it, e.g. `"GRAFT-0001"`.
    pub graft: Option<&'static str>,
    pub notes: &'static str,
}

impl Provenance {
    pub const fn original(notes: &'static str) -> Self {
        Self { origin: Origin::Original, harness: None, license: None, graft: None, notes }
    }
}

/// Identity and lineage of one strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StrategyInfo {
    /// Unique within its seam; used in profiles. `snake_case`.
    pub id: &'static str,
    pub seam: Seam,
    pub summary: &'static str,
    pub provenance: Provenance,
}

/// Implemented by every strategy, whatever its seam.
pub trait Strategy: Send + Sync {
    fn info(&self) -> &StrategyInfo;
}

// ---------------------------------------------------------------- edit format

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("search text not found: {0:?}")]
    NotFound(String),
    #[error("search text matched {0} times; expected exactly one")]
    Ambiguous(usize),
    #[error("malformed edit: {0}")]
    Malformed(String),
}

/// How the model expresses a change to a file.
pub trait EditFormat: Strategy {
    /// Apply `edit` (in this format) to `original`, returning the new content.
    fn apply(&self, original: &str, edit: &str) -> Result<String, EditError>;

    /// How to write an edit in this format; shown to the model in the `edit` tool description.
    fn instructions(&self) -> &'static str;
}

// -------------------------------------------------------------------- context

/// A candidate piece of context (file, message, tool result, summary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextItem {
    pub id: String,
    pub tokens: usize,
    /// Pinned items are always kept (system prompt, task, AGENTS.md).
    pub pinned: bool,
}

/// What goes into the model's context window.
pub trait ContextStrategy: Strategy {
    /// Return the ids to include, in order, within `budget` tokens where possible.
    fn select(&self, items: &[ContextItem], budget: usize) -> Vec<String>;
}

// ------------------------------------------------------------------- approval

/// An action the agent wants to take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// Tool name, e.g. `"read"`, `"edit"`, `"bash"`.
    pub tool: String,
    /// Path, command line, or URL the tool acts on.
    pub resource: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Ask,
    Deny,
}

/// Which actions run without asking the user.
pub trait ApprovalPolicy: Strategy {
    fn decide(&self, action: &Action) -> Decision;

    /// Why the decision was made, for the model (e.g. what to do instead of a denied action).
    fn explain(&self, _action: &Action) -> Option<String> {
        None
    }

    /// `Some(reason)` when this action must be confirmed every time: an earlier
    /// "always" answer does not cover it (ADR 0006). Only applies when `decide`
    /// returns `Ask`; a policy that allows outright (e.g. inside a sandbox) wins.
    fn needs_fresh_yes(&self, _action: &Action) -> Option<String> {
        None
    }
}

// -------------------------------------------------------------------- sandbox

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confinement {
    /// Runs with the user's full authority.
    None,
    /// Restricted to these writable paths, with or without network.
    Restricted { writable: Vec<String>, network: bool },
}

/// How shell commands are confined.
pub trait Sandbox: Strategy {
    fn confinement(&self, command: &Command) -> Confinement;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seam_keys_are_unique_and_snake_case() {
        let keys: Vec<_> = Seam::ALL.iter().map(|s| s.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len());
        assert!(keys.iter().all(|k| k.chars().all(|c| c.is_ascii_lowercase() || c == '_')));
    }
}
