---
id: GRAFT-0057
title: Custom commands from Markdown prompt files
kind: adopt
status: trial
seam: extension
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code (`.claude/commands/*.md`, `$ARGUMENTS`) and Gemini CLI (custom commands) turn prompt files into slash commands.

## Why it matters

Teams repeat the same prompts (review this, write tests for that); typing them is slow and inconsistent.

## Hypothesis

Repeated tasks become one short command shared through the repo.

## Implementation

`commands.rs`: `<name>.md` in `.agents/commands/` or `.claude/commands/` (workspace first, then home) becomes `/name` in chat and `oxgraft run /name …`; `$ARGUMENTS` is replaced (or arguments appended), YAML front matter dropped, `description` shown in /help. Prompts only, so no trust needed.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli `commands_expand_with_arguments`; e2e `custom_commands_expand_in_run`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
