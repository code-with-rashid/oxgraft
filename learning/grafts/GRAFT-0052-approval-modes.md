---
id: GRAFT-0052
title: Approval modes: read-only, suggest, auto-edit, full-auto
kind: adopt
status: trial
seam: approval
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex (rust-v0.172.0) offers named approval modes (read-only, ask before edits/commands, auto-edit, full-auto); Claude Code has plan mode and acceptEdits.

## Why it matters

Writing rules for the common cases is tedious; users want one switch for 'just look', 'edit freely' and 'run unattended'.

## Hypothesis

Users pick the right autonomy per task without hand-written rules, while protected paths and risk rails keep the dangerous cases asked.

## Implementation

`--mode` / `approval_mode` / `OXGRAFT_APPROVAL_MODE` add rules between the built-in defaults and user rules: read-only denies edit/write/bash/MCP with a reason; auto-edit allows edit/write; full-auto also allows bash. Untrusted workspaces can't set it.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `approval_modes_change_what_runs_without_asking`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
