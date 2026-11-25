---
id: GRAFT-0104
title: Background shell commands
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

Claude Code's Bash tool has `run_in_background`, with BashOutput to read new output and KillShell to stop it.

## Why it matters

Dev servers, watchers and long builds block the agent until the timeout, so it can't start a server and then test against it.

## Hypothesis

The agent can run and check services (start server, curl it, read logs) in one task.

## Implementation

`bash` takes `background: true`: the command starts (same sandbox and scrubbed environment) and returns an id. `bash_output {id}` returns output since the last read and the status; `kill: true` stops it (the whole process group on Unix); without `id` it lists commands. At most 8 at once, 1 MB of output kept each; everything is stopped when the run or chat ends. Starting one needs the same approval as any bash command.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `background_commands_run_report_and_stop`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
