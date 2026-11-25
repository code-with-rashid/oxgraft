---
id: GRAFT-0094
title: Keep secrets out of model-run shell commands
kind: adopt
status: trial
seam: sandbox
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex's `shell_environment_policy` by default excludes variables matching *KEY*, *SECRET* and *TOKEN* from the commands the model runs.

## Why it matters

The harness process holds provider API keys and often cloud and Git tokens; a prompt-injected `env | curl …` could send them anywhere, sandbox or not.

## Hypothesis

Commands the model runs never see credentials unless the user passes one on purpose.

## Implementation

The bash tool removes variables whose names contain KEY, TOKEN, SECRET, PASSWORD, PASSWD or CREDENTIAL (any case) from the child environment; names listed in `OXGRAFT_SHELL_ENV_KEEP` (comma-separated) are kept. Hooks and verify_command (the user's own commands) keep the full environment.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `secret_looking_variables_are_hidden`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
