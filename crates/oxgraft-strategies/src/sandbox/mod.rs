use oxgraft_seams::{Command, Confinement, Origin, Provenance, Sandbox, Seam, Strategy, StrategyInfo};

/// No confinement: commands run with the user's authority.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSandbox;

static NONE: StrategyInfo = StrategyInfo {
    id: "none",
    seam: Seam::Sandbox,
    summary: "No confinement. Pair with an approval policy or an outer container.",
    provenance: Provenance::original("Baseline; matches Pi's and Aider's default host authority."),
};

impl Strategy for NoSandbox {
    fn info(&self) -> &StrategyInfo {
        &NONE
    }
}

impl Sandbox for NoSandbox {
    fn confinement(&self, _command: &Command) -> Confinement {
        Confinement::None
    }
}

/// Shell commands may write only inside the workspace (and a private /tmp), with
/// no network. Enforced by bubblewrap on Linux and Seatbelt on macOS; where neither
/// is available the bash tool refuses to run (fails closed).
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspaceWrite;

static WORKSPACE_WRITE: StrategyInfo = StrategyInfo {
    id: "workspace_write",
    seam: Seam::Sandbox,
    summary: "OS sandbox: writes only inside the workspace, no network (bubblewrap / Seatbelt); fails closed.",
    provenance: Provenance {
        origin: Origin::CleanRoom,
        harness: Some("codex"),
        license: Some("Apache-2.0"),
        graft: Some("GRAFT-0025"),
        notes: "Codex's workspace-write sandbox mode; Claude Code also sandboxes bash with bubblewrap / Seatbelt.",
    },
};

impl Strategy for WorkspaceWrite {
    fn info(&self) -> &StrategyInfo {
        &WORKSPACE_WRITE
    }
}

impl Sandbox for WorkspaceWrite {
    fn confinement(&self, _command: &Command) -> Confinement {
        Confinement::Restricted { writable: Vec::new(), network: false }
    }
}
