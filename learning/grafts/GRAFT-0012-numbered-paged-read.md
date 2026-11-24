---
id: GRAFT-0012
title: Numbered, paged file reads
kind: adopt
status: trial
seam: tools
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's and OpenCode's read tools return line-numbered content with offset and limit parameters.

## Why it matters

Line numbers let the model reference locations precisely; paging keeps huge files from flooding context.

## Hypothesis

Fewer edit failures from misremembered context on large files.

## Implementation

`read` returns `line<TAB>text`, default 2000 lines, long lines truncated, a continuation hint with the next offset; binary files are refused.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `read_numbers_lines_and_pages`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
