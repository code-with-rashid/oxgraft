---
id: GRAFT-0008
title: Non-interactive shell environment
kind: adopt
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

Harnesses that run shell commands for a model disable pagers and prompts so commands cannot block waiting for a human.

## Why it matters

`git log` opening a pager or `git push` asking for credentials hangs an unattended agent until timeout.

## Hypothesis

No bash timeouts caused by pagers or credential prompts.

## Implementation

`bash` sets PAGER=cat, GIT_PAGER=cat, GIT_TERMINAL_PROMPT=0, NO_COLOR=1, TERM=dumb, null stdin, a hard timeout, and does not wait forever on pipes held by grandchildren.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `runs_in_workspace_and_reports_exit_codes`, `times_out_without_hanging`; SPEC-0002 AC4). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
