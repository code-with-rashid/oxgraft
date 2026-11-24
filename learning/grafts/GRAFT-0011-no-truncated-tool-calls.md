---
id: GRAFT-0011
title: Never execute tool calls cut off by the output limit
kind: avoid
status: trial
seam: tools
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

When a model's response hits the output token limit, the last tool call's input can be incomplete; harnesses that apply it anyway can write half a file.

## Why it matters

A truncated `write` silently corrupts a file.

## Hypothesis

Zero partial writes caused by max_tokens; the model retries in smaller steps.

## Implementation

If the stop reason is max_tokens, tool calls in that response return an error asking for smaller steps; text-only truncation asks the model to continue (bounded to 3).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `handles_max_tokens`; SPEC-0003 AC5). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
