---
id: GRAFT-0025
title: OS sandbox for shell commands (workspace-write, offline)
kind: adopt
status: trial
seam: sandbox
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: "workspace_write"
opened: 2026-11-25
decided: ""
---

## Observation

Codex runs commands in a workspace-write sandbox with network off by default (Landlock/seccomp on Linux, Seatbelt on macOS); Claude Code sandboxes bash with bubblewrap and Seatbelt. agentic-arena's reviews flag OpenCode and Pi for relying on permissions or an outer sandbox only.

## Why it matters

Approval prompts are not containment: a model-written script can write anywhere or exfiltrate data. An OS sandbox bounds the blast radius of `--yes` runs.

## Hypothesis

Zero writes outside the workspace and zero network access from bash in the sandboxed profile, with normal build/test commands still working.

## Implementation

Strategy `workspace_write` returns Restricted confinement; the bash tool wraps commands with bubblewrap (read-only root, workspace bind, private /tmp, --unshare-net) or a Seatbelt profile, probes that the sandbox works, and fails closed otherwise. Profile `sandboxed`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `confines_writes_to_the_workspace_when_available`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
