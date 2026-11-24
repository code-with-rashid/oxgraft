---
id: GRAFT-0003
title: Confine file tools to the workspace
kind: avoid
status: trial
seam: tools
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

agentic-arena's reviews note that OpenCode's permission rules govern tools but are not an outer sandbox, and that Pi has no built-in filesystem restriction (relying on an outer sandbox).

## Why it matters

A model that writes `../../.ssh/config` or an absolute path should be stopped by the harness, not only by an approval prompt the user may click through.

## Hypothesis

Zero writes outside the workspace in boundary tasks, with no extra approval prompts.

## Implementation

`oxgraft-tools::util::resolve` normalises `..`, resolves symlinks on the existing prefix, and refuses anything outside the canonical workspace for read, write, edit, ls, glob and grep. Shell commands are not confined yet (sandbox seam).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `refuses_paths_outside_workspace`; SPEC-0002 AC2). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
