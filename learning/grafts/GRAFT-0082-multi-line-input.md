---
id: GRAFT-0082
title: Multi-line chat messages
kind: adopt
status: trial
seam: session
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Aider supports `{` … `}` multi-line input; Claude Code accepts pasted multi-line text and backslash-newline.

## Why it matters

Each pasted line became a separate task, so pasting an error log or a spec started dozens of runs.

## Hypothesis

Pasting logs and specs into chat just works.

## Implementation

`read_message`: a line that is just `"""` starts a block ending at the next `"""`; a line ending in a backslash continues on the next line. Plain lines are unchanged.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli `multi_line_messages`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
