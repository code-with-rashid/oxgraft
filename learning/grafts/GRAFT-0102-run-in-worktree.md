---
id: GRAFT-0102
title: Run a task in its own git worktree
kind: adopt
status: trial
seam: session
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code (`--worktree`, worktree subagents) and Codex (cloud tasks on branches) isolate agent work on a separate branch and checkout.

## Why it matters

Letting an agent loose on the working copy mixes its changes with the user's uncommitted work; reviewing or discarding them is fiddly.

## Hypothesis

Users run bolder unattended tasks because discarding them is one command.

## Implementation

`run --worktree` creates `.oxgraft/worktrees/<id>` on branch `oxgraft/<id>` from HEAD (git repo with a commit required; `.oxgraft/.gitignore` keeps it out of the user's status), runs the task there, and prints how to review, merge or remove it. Note: the worktree path is not in `trusted_workspaces`, so repo-level hooks and MCP servers don't load there.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `worktree_runs_leave_the_checkout_alone`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
