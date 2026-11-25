---
id: GRAFT-0080
title: Show that the model is working
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

Claude Code, Codex and Gemini CLI show a spinner or status line while waiting for the model.

## Why it matters

With adaptive thinking the first token can take many seconds; a silent terminal looks hung.

## Hypothesis

Fewer premature Ctrl-C interrupts during long thinking.

## Implementation

Core emits `turn_started { turn }` before each top-level model request (append-only protocol addition). The human renderer prints a dim "… thinking" line on stderr and clears it when the next event arrives; only on a terminal, on Unix or Windows Terminal, and not with NO_COLOR.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-protocol round-trip; e2e runs consume the event). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
