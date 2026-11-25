---
id: GRAFT-0059
title: Remind the model of unfinished todo items before finishing
kind: adopt
status: trial
seam: session
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode and Claude Code keep the model's todo list visible and nudge it about open items.

## Why it matters

Models lay out a plan, do most of it, and stop with steps silently skipped.

## Hypothesis

Fewer runs end with planned steps left undone and unmentioned.

## Implementation

Core remembers the latest `todo` list; when the model replies without tool calls while items are not `done`, it adds one reminder per run listing them (finish, update the list, or explain), then accepts the next finish.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `unfinished_todo_items_get_one_reminder`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
