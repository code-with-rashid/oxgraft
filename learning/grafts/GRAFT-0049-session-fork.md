---
id: GRAFT-0049
title: Fork a session from a finished reply
kind: adopt
status: trial
seam: session
donor: pi
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Pi (v1.6.0) keeps sessions as a tree and lets users branch from any earlier message; Goose and OpenCode also fork sessions.

## Why it matters

Trying an alternative approach meant either polluting the current session with a failed attempt or starting over without context.

## Hypothesis

Users explore alternatives from a known-good point without re-explaining the task.

## Implementation

`Session::fork(keep)` copies the first N messages (N must follow an assistant reply without tool calls, so the copy needs no repair) into a new session with `forked_from`. CLI: `oxgraft run|chat --fork ID[@N]`, conflicting with `--resume`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `forks_copy_history_up_to_a_finished_reply`; oxgraft-cli e2e `sessions_can_be_forked`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
