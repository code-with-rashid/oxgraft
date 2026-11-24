---
id: GRAFT-0033
title: pre_tool hooks fail closed
kind: avoid
status: trial
seam: extension
donor: goose
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

The goose review (v1.60.2) found PreToolUse hooks default to `on_failure: allow` and its adversary reviewer fails open. Oxgraft's own pre_tool hooks had the same flaw: a crashing or timed-out hook let the call run.

## Why it matters

A guard that silently disappears when it breaks is worse than no guard: users believe they are protected.

## Hypothesis

No tool call ever runs when its pre_tool guard could not run.

## Implementation

A pre_tool hook that cannot start, exits non-zero (other than 2) or times out now blocks the call with an explanatory error; post_tool/session_end failures stay warnings.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `failing_pre_tool_hooks_fail_closed`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
