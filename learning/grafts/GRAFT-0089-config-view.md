---
id: GRAFT-0089
title: Show the resolved configuration
kind: adopt
status: trial
seam: session
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code (`/config`, `/status`) and Codex show which settings are in effect and where they come from.

## Why it matters

With user, workspace, environment and flag layers plus trust gating, users couldn't tell why a setting was or wasn't applied.

## Hypothesis

Configuration problems are diagnosed without reading source.

## Implementation

`oxgraft config` resolves the layers exactly as a run would (including `--mode`, `--plan`, `--append-system-prompt`) and prints each setting, the files that contributed, whether the workspace is trusted, every warning, and the user-config and data-dir paths.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `config_command_shows_resolved_settings`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
