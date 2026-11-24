---
id: GRAFT-0009
title: One neutral model, many providers
kind: adopt
status: trial
seam: provider
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode, goose and Aider all support many model vendors behind one internal interface; most vendors and local servers speak the OpenAI Chat Completions format.

## Why it matters

Users should be able to run Oxgraft with any model they have access to, including local ones.

## Hypothesis

Any OpenAI-compatible endpoint works with no code changes; Anthropic models keep their native features (thinking blocks, fallbacks).

## Implementation

`oxgraft-seams::model` neutral types with `Opaque` blocks for provider-specific content; `oxgraft-providers` with native Anthropic, ten OpenAI-compatible presets, and a mock.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-providers request/response mapping tests; SPEC-0001 AC1-AC7). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
