---
id: GRAFT-0034
title: Untrusted workspaces cannot choose the model
kind: adopt
status: trial
seam: provider
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex (rust-v0.172.0 docs) forbids project-scoped config from setting `model_provider`, `openai_base_url` or `notify`, so a cloned repo cannot redirect API traffic.

## Why it matters

A repository's `.oxgraft/config.toml` could otherwise send your prompts, code and API keys to a provider of its choosing.

## Hypothesis

No untrusted workspace can change where requests go.

## Implementation

`model` in workspace config is ignored with a warning unless the workspace is in the user's `trusted_workspaces`; warnings are printed before model errors so the reason is visible.

## Evaluation

Unit and end-to-end tests cover the behaviour (CLI e2e `workspace_config_and_profiles`, config unit test). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
