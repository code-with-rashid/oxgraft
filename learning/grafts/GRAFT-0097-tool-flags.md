---
id: GRAFT-0097
title: Allow or deny tools for one run from the command line
kind: adopt
status: trial
seam: approval
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code has `--allowedTools` and `--disallowedTools` (with patterns such as `Bash(npm test)`) for a single invocation.

## Why it matters

Scripts and CI want one-off permissions (allow `cargo test`, forbid web access) without editing config files.

## Hypothesis

Headless runs get exactly the permissions they need, no broader.

## Implementation

Repeatable `--allow-tool` / `--deny-tool` with `TOOL` or `TOOL:PATTERN` become rules added after mode and config rules (so they win over allows); denies carry a justification; any deny still beats any allow. They also apply to every chat `/mode` toolkit.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `tool_flags_allow_and_deny_for_a_run`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
