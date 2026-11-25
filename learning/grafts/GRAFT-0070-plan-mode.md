---
id: GRAFT-0070
title: Plan mode: read-only research that ends in a plan
kind: adopt
status: trial
seam: approval
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's plan mode and Codex's read-only mode let the agent explore and propose before touching anything.

## Why it matters

For risky or large changes users want to review the approach first, then let the agent execute it with the same context.

## Hypothesis

Fewer wasted edits on misunderstood tasks.

## Implementation

`--plan` = read-only approval mode (GRAFT-0052) plus a system instruction to explore and reply with a numbered plan (goal, files, risks, verification). The session is saved, so `oxgraft run --resume last "go ahead"` carries it out. Conflicts with `--mode`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `plan_mode_changes_nothing`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
