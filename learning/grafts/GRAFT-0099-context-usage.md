---
id: GRAFT-0099
title: Show how full the context is
kind: adopt
status: trial
seam: context
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's /context (and its context indicator) shows how much of the window the conversation uses.

## Why it matters

Users can't tell when a chat is close to automatic compaction or when `/compact` would help.

## Hypothesis

Users compact or start fresh at the right time.

## Implementation

The loop stores the last top-level request's input + output tokens in `Session.context_tokens`; chat `/cost` prints it against `context_window` with a percentage.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli `context_usage_lines`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
