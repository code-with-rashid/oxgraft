---
id: GRAFT-0110
title: Reading a directory lists it
kind: adopt
status: trial
seam: tools
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode's read tool returns a directory's entries when given a directory path.

## Why it matters

Models often call read on a folder; an error costs a turn and a retry with ls.

## Hypothesis

Fewer wasted turns at the start of exploration, especially with smaller models.

## Implementation

`read` on a directory runs the `ls` tool on it and returns `` `path` is a directory; its entries:`` followed by the listing.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `reading_a_directory_lists_it`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
