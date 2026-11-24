---
id: GRAFT-0020
title: Ctrl-C interrupts cleanly; resumed sessions are repaired
kind: adopt
status: trial
seam: session
donor: cline
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

agentic-arena's Cline review highlights durable abort recovery; interactive harnesses (Claude Code, Codex) let users interrupt a running turn.

## Why it matters

Users must be able to stop a runaway command or turn without killing the process or corrupting the session; an interrupted session must still resume, which requires every tool call to have a result.

## Hypothesis

Interrupts stop within ~200 ms of the next check; zero resume failures from dangling tool calls.

## Implementation

Shared cancel flag (first Ctrl-C interrupts, second exits); checked each turn and tool; bash polls it every 200 ms and kills the process; `Session::repair` closes dangling tool calls on resume.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `cancellation_stops_before_the_next_turn`, `resume_repairs_dangling_tool_calls`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
