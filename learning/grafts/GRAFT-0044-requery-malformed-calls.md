---
id: GRAFT-0044
title: Re-ask malformed tool calls outside the history
kind: adopt
status: trial
seam: session
donor: swe-agent
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

SWE-agent (v1.1.0) re-samples the model on format errors (requery) with the error message, without keeping the failed attempt in the trajectory history.

## Why it matters

Malformed calls left in the history teach the model its own mistakes, cost tokens on every later turn, and count towards the mistake breaker.

## Hypothesis

Weaker models recover from format slips in the same turn and later turns stay clean.

## Implementation

Core validates each reply's calls (known tool, JSON object input, required fields; edit's old_string fallback accepted) and re-asks up to 2 times with per-call errors in a private trail. The trail is kept only if the corrected reply carries thinking blocks, since those were produced after the bad attempt.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `malformed_tool_calls_are_requeried_out_of_history`, `unknown_tools_and_bad_input_become_errors`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
