---
id: GRAFT-0023
title: Append-only context compaction
kind: adopt
status: trial
seam: context
donor: pi
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Pi, OpenCode and Claude Code compact long sessions by summarising earlier context so work can continue past the context window.

## Why it matters

Long tasks otherwise fail with context-length errors, especially on local models with small windows.

## Hypothesis

Long tasks continue past the window with no context-length errors; quality loss is small compared with failure.

## Implementation

Above 80% of `context_window` (default 200k, configurable, 0 disables) the model writes a handover summary; the session restarts from task + summary. History is replaced, never edited in place, so provider-specific blocks (e.g. Anthropic thinking) stay valid.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `compacts_context_and_continues`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
