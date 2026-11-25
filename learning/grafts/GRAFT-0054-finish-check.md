---
id: GRAFT-0054
title: Finish only when an independent check passes
kind: adopt
status: trial
seam: session
donor: qwen-code
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Qwen Code (v0.28.4, docs goals.md) has /goal: the agent keeps working until an independent verifier accepts the result, within a visible budget.

## Why it matters

Models declare victory early; the user then finds the tests still failing.

## Hypothesis

Unattended runs end in a verified state more often, at the cost of a few extra turns.

## Implementation

`oxgraft run --until "<command>"`: when the model replies without tool calls, core runs the command in the bash tool's shell; on failure its output is sent back and the loop continues, up to 3 rounds, then the run stops with a clear reason. Command-line only (not configurable by a workspace). The verifier is a command, not a second model.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `finish_check_sends_failures_back_until_it_passes`; oxgraft-cli e2e `run_until_a_check_passes`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
