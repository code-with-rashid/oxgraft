---
id: GRAFT-0087
title: Token use per model across sessions
kind: adopt
status: trial
seam: session
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode (`stats`) and Claude Code (`/cost`, usage views) show token use beyond the current session.

## Why it matters

Users switching between models and providers can't see where their tokens go without adding up session files.

## Hypothesis

Users notice expensive habits and pick models more deliberately.

## Implementation

`oxgraft stats [--json]` sums sessions, input and output tokens per model from the workspace's saved sessions (subagent runs are counted in their parent session). No prices: they change too often to hard-code.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `stats_sum_usage_per_model`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
