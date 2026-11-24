---
id: GRAFT-0015
title: Whitespace-tolerant SEARCH/REPLACE
kind: adopt
status: trial
seam: edit_format
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: "search_replace_tolerant"
opened: 2026-11-25
decided: ""
---

## Observation

Aider falls back to looser matching when SEARCH text differs from the file only in whitespace; Cline similarly retries diff application with relaxed matching.

## Why it matters

Exact matching fails on harmless indentation or trailing-space differences, a common cause of failed edits.

## Hypothesis

Fewer failed edits than `search_replace` with no increase in wrong-location edits (ambiguity is still an error).

## Implementation

New strategy `search_replace_tolerant`: exact, then trailing-whitespace-insensitive, then indentation-insensitive with re-indentation; profile `tolerant`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-strategies tolerant tests). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
