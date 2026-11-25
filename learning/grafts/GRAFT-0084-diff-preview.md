---
id: GRAFT-0084
title: Show old and new text in edit approvals
kind: adopt
status: trial
seam: approval
donor: cline
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Cline and Claude Code show a diff of the proposed change in the approval UI.

## Why it matters

For old/new_string edits the preview showed only the new text, so users approved changes without seeing what was replaced.

## Hypothesis

Users catch wrong edits at the prompt instead of after the fact.

## Implementation

`preview_lines` renders old/new pairs (and each batched edit) as `- old` / `+ new` lines, edit-format text as sent, and write content, capped at 40 lines with a count of the rest.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli `edit_previews_show_old_and_new`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
