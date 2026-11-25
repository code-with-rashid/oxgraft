---
id: GRAFT-0050
title: Batched multi-edits matched against the original file
kind: adopt
status: trial
seam: edit_format
donor: pi
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Pi (v1.6.0) applies several edits in one call, each matched against the original file rather than the result of the previous edit.

## Why it matters

Sequential edits cascade: an early replacement can create or destroy the text a later one targets, and a failure half-way leaves the file half-edited.

## Hypothesis

Fewer tool calls and fewer partial edits on multi-site changes.

## Implementation

The `edit` tool accepts `edits: [{old_string, new_string}]`; each must match exactly once in the original, matches must not overlap, and all apply or none do. The malformed-call check (GRAFT-0044) accepts `edits` in place of `edit`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `batched_edits_apply_together_or_not_at_all`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
