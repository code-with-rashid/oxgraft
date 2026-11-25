---
id: GRAFT-0086
title: Whitespace-tolerant old_string matching
kind: adopt
status: trial
seam: edit_format
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode's edit tool tries a chain of replacers (exact, line-trimmed, indentation-flexible, …) before failing.

## Why it matters

The most common edit failure is an old_string that differs only by trailing spaces or by the surrounding block's indentation; the model then re-reads and retries.

## Hypothesis

Fewer failed edits and retries, with no wrong replacements (unique matches only).

## Implementation

When the exact old_string isn't found, compare whole lines ignoring trailing whitespace and a uniform extra indentation (taken from the first non-blank line). A unique match is replaced and new_string gets the same extra indentation; ambiguous or partial matches still fail with the usual hint.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `old_string_tolerates_trailing_space_and_indentation`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
