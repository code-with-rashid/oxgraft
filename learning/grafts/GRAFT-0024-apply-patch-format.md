---
id: GRAFT-0024
title: Codex apply_patch edit format
kind: adopt
status: trial
seam: edit_format
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: "apply_patch"
opened: 2026-11-25
decided: ""
---

## Observation

Codex models are trained to edit with apply_patch hunks (@@ anchors, context/removed/added lines).

## Why it matters

Matching the format a model was trained on lowers edit failure rates for that model family.

## Hypothesis

With Codex-family models, `codex-like` has fewer failed edits than `default`.

## Implementation

Strategy `apply_patch` (envelope optional, anchors, exact-once matching ignoring trailing whitespace, insertions); profile `codex-like`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-strategies apply_patch tests). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
