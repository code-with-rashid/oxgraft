---
id: GRAFT-0021
title: Token budget per run
kind: avoid
status: trial
seam: session
donor: openhands
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Unattended agent runs in many harnesses rely on turn limits alone, which do not bound cost when each turn carries a large context.

## Why it matters

A runaway session can spend far more than intended before hitting max turns.

## Hypothesis

No run exceeds its configured token budget by more than one turn.

## Implementation

`max_session_tokens` (config or OXGRAFT_MAX_SESSION_TOKENS) stops the run at the start of the next turn once exceeded.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `token_budget_stops_runaway_sessions`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
