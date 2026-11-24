use oxgraft_seams::{Command, Confinement, Provenance, Sandbox, Seam, Strategy, StrategyInfo};

/// No confinement: commands run with the user's authority.
///
/// Candidate grafts: Codex's Landlock/seccomp (Linux) and Seatbelt (macOS)
/// sandboxes (Apache-2.0, Rust: adaptable), container workspaces (OpenHands).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSandbox;

static INFO: StrategyInfo = StrategyInfo {
    id: "none",
    seam: Seam::Sandbox,
    summary: "No confinement. Pair with an approval policy or an outer container.",
    provenance: Provenance::original("Baseline; matches Pi's and Aider's default host authority."),
};

impl Strategy for NoSandbox {
    fn info(&self) -> &StrategyInfo {
        &INFO
    }
}

impl Sandbox for NoSandbox {
    fn confinement(&self, _command: &Command) -> Confinement {
        Confinement::None
    }
}
