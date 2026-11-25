---
id: GRAFT-0046
title: Per-turn workspace checkpoints in a private git repo
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

Cline (v4.1.27) and Kilo Code (v7.8.32) snapshot the workspace into a shadow git repository after each step, separate from the user's repo, and let the user restore any checkpoint.

## Why it matters

`oxgraft undo` only knows files changed through edit/write; changes made by shell commands (generators, `sed -i`, `rm`) were not recoverable.

## Hypothesis

Every change a session makes, by any tool, can be rolled back to any turn without touching the user's git state.

## Implementation

`oxgraft_core::checkpoint::Checkpoints`: git with `--git-dir .oxgraft/checkpoints.git --work-tree <workspace>`, user env GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE removed, workspace .gitignore plus default heavy-dir excludes. Snapshot at start and after turns using non-read-only tools; records in `Session.checkpoints`. `oxgraft checkpoints` / `oxgraft restore N` (takes a pre-restore checkpoint first). Off for home dir and filesystem roots; failures are warnings.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `snapshots_and_restores_without_touching_the_users_repo`; oxgraft-cli e2e `checkpoints_capture_shell_changes_and_restore`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
