---
id: GRAFT-0013
title: Give the model branch and git status up front
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

Claude Code includes repository state (branch, status) in its environment context at session start.

## Why it matters

The model can see uncommitted work before changing anything and avoids clobbering it, without spending a turn on `git status`.

## Hypothesis

One fewer exploratory turn per session in git repos; fewer edits to files the user had modified.

## Implementation

`prompt::git_summary` adds the branch and up to 20 porcelain status lines; silently skipped outside git or without git installed.

## Evaluation

Unit and end-to-end tests cover the behaviour (exercised by prompt construction in agent tests). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
