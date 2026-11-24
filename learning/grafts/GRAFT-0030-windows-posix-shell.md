---
id: GRAFT-0030
title: Prefer a POSIX shell on Windows, then PowerShell
kind: avoid
status: trial
seam: tools
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Models emit POSIX shell syntax by default. On Windows, Claude Code runs commands through Git Bash and Codex through PowerShell; running model commands through cmd.exe fails on common syntax (`&&` chains with quoting, `$VAR`, `ls`, `cat`).

## Why it matters

Windows users get failed commands and wasted turns if the harness uses cmd.exe.

## Hypothesis

Fewer failed shell commands on Windows.

## Implementation

`bash` uses `sh` from Git for Windows when found (PATH, standard install paths, or OXGRAFT_SHELL), otherwise PowerShell with -NoProfile -NonInteractive, and tells the model which syntax to use in the tool description.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools bash tests run on Windows CI with the detected shell). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
