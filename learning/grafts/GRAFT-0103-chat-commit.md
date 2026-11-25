---
id: GRAFT-0103
title: Commit with a model-written message
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

Aider auto-commits its edits and has `/commit` to commit pending changes with a generated message.

## Why it matters

Writing commit messages for agent-made changes is tedious; leaving them uncommitted mixes them with later work.

## Hypothesis

More small, well-described commits from agent sessions.

## Implementation

Chat `/commit` commits the staged changes (or, if none, all changes) with a message the current model writes from the staged diff (subject under 72 chars, optional body). Co-authored-by, Signed-off-by and 'Generated with' lines are removed; the user's git identity is used. Not automatic: the user decides when.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `chat_commit_writes_a_message`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
