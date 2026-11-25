---
id: GRAFT-0090
title: Recover from replies with no text and no tool calls
kind: adopt
status: trial
seam: session
donor: goose
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

goose v1.61.0 release notes: "Recover from thinking-only provider turns instead of ending silently".

## Why it matters

With adaptive thinking (GRAFT-0071) a reply can hold only a thinking block; Oxgraft took it as the final answer and ended the run with nothing to show.

## Hypothesis

No run ends with an empty answer because of a thinking-only or empty turn.

## Implementation

When a reply has no tool calls and no visible text (and isn't a refusal), core adds a short continue prompt, sharing the max_tokens continuation budget (3 per run).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `empty_replies_are_not_final_answers`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
