---
id: GRAFT-0067
title: Standing instructions from config and --append-system-prompt
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

Claude Code has `--append-system-prompt` and user-level memory files; Codex has `instructions` in its config.

## Why it matters

Personal preferences (language, style, tools to avoid) shouldn't have to live in every repository's AGENTS.md.

## Hypothesis

Users state preferences once and every session follows them.

## Implementation

`instructions` in the user or workspace config and `--append-system-prompt TEXT` accumulate (user, workspace, flag) and are appended to the system prompt before the skills list. Prompt-only, so like AGENTS.md they need no trust.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli `instructions_accumulate_across_layers`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
