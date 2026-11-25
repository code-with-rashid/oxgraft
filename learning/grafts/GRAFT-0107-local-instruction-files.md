---
id: GRAFT-0107
title: Personal local instruction files
kind: adopt
status: trial
seam: core
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code reads `CLAUDE.local.md` for personal project notes that are not committed.

## Why it matters

Developers want per-repo preferences (local URLs, their own test commands) without editing the shared AGENTS.md.

## Hypothesis

Users keep personal notes out of shared files, and setups made for Claude Code carry over.

## Implementation

`AGENTS.local.md` and `CLAUDE.local.md` join the instruction file list after the shared files. While there, every root instruction file must resolve inside the workspace, so a symlinked AGENTS.md can't pull an outside file into the prompt (imports already had this check).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `local_instruction_files_come_last_and_symlinks_out_are_ignored`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
