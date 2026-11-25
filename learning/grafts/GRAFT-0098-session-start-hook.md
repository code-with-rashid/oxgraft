---
id: GRAFT-0098
title: session_start hooks that add context
kind: adopt
status: trial
seam: extension
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's SessionStart hooks can add context (recent issues, branch notes) when a session begins.

## Why it matters

user_prompt hooks (GRAFT-0062) fire on every task; start-of-session context such as the current ticket should be added once.

## Hypothesis

Teams inject per-session context without repeating it every turn.

## Implementation

Hook event `session_start` runs before the first task of a new session (not on resume or later chat turns); stdout is appended as context, exit 2 stops the run. Same trust gate as other hooks.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `session_start_hooks_run_once_per_session`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
