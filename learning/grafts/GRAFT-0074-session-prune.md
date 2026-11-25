---
id: GRAFT-0074
title: Prune old sessions
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

OpenCode and Claude Code clean up old sessions (Claude Code deletes transcripts after a configurable period).

## Why it matters

Every run writes a session file and undo backups to .oxgraft/sessions; long-lived workspaces accumulate them without bound.

## Hypothesis

Disk use stays bounded without users deleting files by hand.

## Implementation

`Session::prune(keep, dry_run)` and `oxgraft prune [--keep 50] [--dry-run]` delete all but the newest sessions and their backup dirs; only files that parse as sessions are touched.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `prune_keeps_the_newest_sessions`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
