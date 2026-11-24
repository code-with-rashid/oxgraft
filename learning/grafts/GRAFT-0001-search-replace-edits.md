---
id: GRAFT-0001
title: Use exact SEARCH/REPLACE blocks for edits
kind: adopt
status: trial
seam: edit_format
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: search_replace
opened: 2026-11-24
decided: ""
---

## Observation

Aider's `diff` edit format has the model write SEARCH/REPLACE blocks naming
the exact existing text and its replacement, instead of rewriting whole files.
Aider publishes edit-format benchmarks comparing formats per model.

## Why it matters

Whole-file rewrites cost tokens in proportion to file size and invite
accidental changes to untouched code. Exact blocks keep edits small and
reviewable.

## Hypothesis

Against `whole_file`: fewer output tokens per successful edit on files over
200 lines, with an edit failure rate no worse than 5 percentage points.

## Implementation

`crates/oxgraft-strategies/src/edit/search_replace.rs`. Clean-room from the
format's documentation. Stricter than the donor in one way: a SEARCH text
that matches more than once is an error, not a guess. That strictness is
itself a hypothesis to evaluate; fuzzy matching is a candidate follow-up graft.

## Evaluation

Pending: needs the model loop (roadmap M2) and arena tasks (M4).
`profiles/default.toml` uses this strategy **provisionally** so the scaffold
has a working default; it is not yet "adopted".

## Decision

Pending.
