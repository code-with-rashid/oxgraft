---
id: GRAFT-0108
title: Ephemeral runs that save nothing
kind: adopt
status: trial
seam: cli
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex `exec --ephemeral` and Claude Code `--no-session-persistence` run without writing session files.

## Why it matters

Scripted and CI runs pile up session files nobody resumes, and some tasks touch data that shouldn't stay on disk.

## Hypothesis

Automation can run Oxgraft without cleaning up `.oxgraft/sessions` afterwards.

## Implementation

`Session.ephemeral` (not serialised) makes `save` a no-op; `run --ephemeral` sets it, turns checkpoints off and prints `ephemeral run: session not saved` instead of the resume hint. It conflicts with `--worktree`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `ephemeral_runs_save_no_session`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
