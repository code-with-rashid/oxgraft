---
id: GRAFT-0096
title: Glob results newest first
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

Claude Code's Glob tool returns matching paths sorted by modification time, newest first.

## Why it matters

Alphabetical order buries the files the user and agent just touched; with the 1,000-result cap they can be cut off entirely.

## Hypothesis

The relevant file is in the first few results more often.

## Implementation

`glob` sorts matches by modification time (newest first, ties by path) before the cap.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `glob_lists_newest_first`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
