---
id: GRAFT-0005
title: Per-session undo of file changes
kind: adopt
status: trial
seam: session
donor: cline
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Cline offers Git-backed workspace checkpoints with restore; Aider uses Git commits so users can undo agent changes.

## Why it matters

Users trust an agent more when any change can be rolled back in one command, including in repos without git or with uncommitted work.

## Hypothesis

`oxgraft undo` restores 100% of files touched by a session, including deleting files the session created, without depending on git.

## Implementation

`write` and `edit` snapshot a file's bytes before the first change; the core stores backups under `.oxgraft/sessions/<id>/` and `Session::undo` restores them. CLI: `oxgraft undo [id]`, chat `/undo`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `saves_loads_and_undoes`; CLI e2e `run_reads_edits_runs_and_reports`; SPEC-0004 AC4). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
