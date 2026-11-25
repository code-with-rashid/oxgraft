---
id: GRAFT-0085
title: Ring the bell when a long task finishes
kind: adopt
status: trial
seam: session
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code notifies (terminal bell or OS notification) when it finishes or needs input; Aider can ring a bell too.

## Why it matters

Long agent runs make users switch windows; they come back late or poll the terminal.

## Hypothesis

Less idle time between a run finishing and the user reacting.

## Implementation

After a run or chat turn that took at least 30 s, print BEL on stderr if it is a terminal; `bell = false` turns it off. Custom notifications remain possible with a `session_end` hook.

## Evaluation

Unit and end-to-end tests cover the behaviour (none automated (terminal-only behaviour)). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
