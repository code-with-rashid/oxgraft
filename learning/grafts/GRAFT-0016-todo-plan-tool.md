---
id: GRAFT-0016
title: A visible plan tool
kind: adopt
status: trial
seam: tools
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's TodoWrite and Codex's plan updates let the model keep an explicit checklist during long tasks.

## Why it matters

An explicit plan keeps long tasks on track and shows the user progress in the event stream.

## Hypothesis

Higher completion on multi-step tasks; plan visible in JSONL for arena analysis.

## Implementation

Stateless `todo` tool: the model sends the full list; the tool renders it and warns on multiple in-progress items.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `todo_renders_plan`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
