---
id: GRAFT-0088
title: Show what a session changed as a patch
kind: adopt
status: trial
seam: session
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Aider's `/diff` shows the changes since the last message; Claude Code and Codex show diffs of their edits.

## Why it matters

The change summary (GRAFT-0048) lists files, but reviewing the actual change meant leaving Oxgraft, and shell-made changes don't show in the user's git diff if they were already committed.

## Hypothesis

Users review agent changes in place, including ones made by commands.

## Implementation

`Checkpoints::diff_patch`; `oxgraft diff [ID]` and chat `/diff` snapshot the workspace and print the patch from the session's first checkpoint (chat shows 400 lines, control characters neutralised).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `diff_shows_session_changes`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
