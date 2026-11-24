---
id: GRAFT-0038
title: `auto` profile picks the edit format by model family
kind: adopt
status: trial
seam: edit_format
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode (v1.19.6, verified in registry.ts) swaps in apply_patch for GPT-5-class models and str_replace-style edits otherwise; its limitation is ad-hoc substring checks on the model id.

## Why it matters

Models do best with the edit format they were trained on.

## Hypothesis

Codex-family models fail fewer edits under `auto` than under `default`.

## Implementation

New default profile name `auto` resolves through a model-family table (gpt-5, codex, o3, o4 -> codex-like; everything else -> default). Explicit profiles are never overridden. Not yet evaluated, so this is a trial default.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft CLI `auto_picks_profile_by_model_family`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
