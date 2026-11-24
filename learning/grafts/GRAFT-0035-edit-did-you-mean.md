---
id: GRAFT-0035
title: Failed edits show the nearest real lines
kind: adopt
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

Aider (v0.86.0, verified) and goose (v1.60.2, verified) return 'Did you mean' context when a search block does not match.

## Why it matters

Without the actual lines, models retry with the same wrong text and burn turns.

## Hypothesis

Fewer consecutive failed edits on the same file.

## Implementation

On an edit failure the edit tool finds the first line of the search text present in the file (ignoring whitespace) and returns it with two lines of context and line numbers.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `failed_edits_show_the_nearest_real_lines`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
