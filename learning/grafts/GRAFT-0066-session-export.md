---
id: GRAFT-0066
title: Export a session as a Markdown transcript
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

OpenCode can share or export sessions; Claude Code and Codex users routinely paste transcripts into reviews and issues.

## Why it matters

Session JSON is unreadable for humans; reviewing what an agent did, or reporting a bug, needs a readable transcript.

## Hypothesis

Users share and review agent work without screenshots.

## Implementation

`Session::to_markdown` (title, model, tokens, user and assistant text, tool calls, results shortened to 1,500 characters; thinking blocks omitted) and `oxgraft export [ID] [-o file]`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `sessions_export_as_markdown`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
