---
id: GRAFT-0056
title: Attach @file mentions to the task
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

Claude Code, Gemini CLI and OpenCode let users write `@path` in a prompt to attach that file's content.

## Why it matters

Users already know which file matters; making the model find and read it costs turns.

## Hypothesis

Tasks that name files start working on them in the first turn.

## Implementation

`prompt::expand_mentions` appends `<file path=…>` blocks for `@path` words that resolve to text files inside the workspace (max 10 files, 200 KB, 20k chars each); emails, decorators, missing, binary and outside paths stay plain text. Attached files count as read for the read-before-edit guard.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `mentions_attach_workspace_files_only`, `mentioned_files_are_attached_and_editable`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
