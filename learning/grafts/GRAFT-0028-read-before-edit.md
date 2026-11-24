---
id: GRAFT-0028
title: Read before edit; refuse edits to files changed since the last read
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

Claude Code requires a file to be read before Edit/Write may change it, and rejects edits when the file changed after it was read; OpenCode has similar staleness checks.

## Why it matters

Blind edits based on memory, or on a version the user has since changed, silently destroy work.

## Hypothesis

Zero overwrites of user changes made during a session; fewer edit failures from stale context.

## Implementation

The core tracks canonical paths read or written this run with their mtime; `edit` or `write` on an existing file is refused unless it was read, and refused again if the mtime changed since.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `edits_require_a_fresh_read`, `created_files_can_be_edited_and_paths_normalise`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
