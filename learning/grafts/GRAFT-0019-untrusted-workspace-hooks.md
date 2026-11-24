---
id: GRAFT-0019
title: Don't run a repository's hooks until the user trusts it
kind: avoid
status: trial
seam: extension
donor: goose
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

agentic-arena's goose review notes that a recipe can add executable extensions and must be treated as code; project-level hook files in any harness are code from whoever wrote the repo.

## Why it matters

Cloning a repo and running the agent must not execute that repo's commands without consent.

## Hypothesis

No hook from an untrusted workspace ever runs.

## Implementation

Workspace `.oxgraft/config.toml` hooks load only if the workspace is listed in the user config's `trusted_workspaces`; otherwise a warning explains how to enable them.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft CLI `workspace_config_and_flags_layer`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
