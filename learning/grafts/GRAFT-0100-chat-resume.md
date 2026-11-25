---
id: GRAFT-0100
title: Switch sessions inside chat
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

Claude Code's `/resume` picks an earlier conversation from inside a running session.

## Why it matters

Returning to earlier work meant quitting chat and starting `oxgraft chat --resume ID`.

## Hypothesis

Users move between threads of work without restarting.

## Implementation

Chat `/resume [ID]` (default `last`) loads the saved session (the current one is saved after every turn) and shows its title, size and last reply.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `chat_resume_switches_session`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
