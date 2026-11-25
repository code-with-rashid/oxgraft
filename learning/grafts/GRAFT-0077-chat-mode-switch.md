---
id: GRAFT-0077
title: Switch approval mode inside chat
kind: adopt
status: trial
seam: approval
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code cycles permission modes with Shift+Tab mid-session; Codex has `/approvals`.

## Why it matters

Users start cautious, then want edits to flow once they trust the direction (or the reverse) without losing the conversation.

## Hypothesis

Fewer restarts and fewer blanket full-auto sessions.

## Implementation

Chat builds one toolkit per mode (same profile and user rules) before the agent; `/mode X` points the agent at that toolkit; `/mode` shows the current one. Protected paths and risk rails apply in every mode.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `chat_mode_switches_approvals`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
