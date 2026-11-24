---
id: GRAFT-0007
title: Block repeated identical tool calls
kind: avoid
status: trial
seam: tools
donor: mini-swe-agent
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Minimal agent loops such as mini-SWE-agent rely on step limits to end runs; repeated identical actions are a common way agents burn their step and token budgets.

## Why it matters

A model that re-runs the same failing command wastes turns and money and never converges.

## Hypothesis

Fewer sessions ending at max turns, with no loss in pass rate.

## Implementation

The core blocks the fourth consecutive identical (tool, input) call and returns an explanatory error instead.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `blocks_repeated_identical_calls`; SPEC-0003 AC2). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
