---
id: GRAFT-0026
title: Read-only explore subagent
kind: adopt
status: trial
seam: delegation
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode offers named subagents with their own policies; Claude Code's Task tool runs subagents in a fresh context, including a read-only explorer.

## Why it matters

Searching a large codebase fills the main context with file dumps; a subagent returns only the conclusion.

## Hypothesis

Lower main-context token use on exploration-heavy tasks at equal pass rate.

## Implementation

`task` tool at depth 0 runs a nested loop with read, ls, glob, grep and skill only (no writes, so no undo bookkeeping); its events stay internal and its usage is added to the session; subagent sessions are hidden from `sessions`/`last`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `subagent_explores_read_only_and_reports`, `subagent_sessions_are_not_listed`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
