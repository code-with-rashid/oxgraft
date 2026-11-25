---
id: GRAFT-0071
title: Adaptive thinking for Anthropic models that support it
kind: adopt
status: trial
seam: provider
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code runs current Claude models with thinking on; Anthropic's guidance for Opus/Sonnet 4.6+ and Fable models is `thinking: {type: "adaptive"}`, letting the model decide per step how much to reason.

## Why it matters

Requests sent no `thinking` field, leaving multi-step coding work without extended reasoning.

## Hypothesis

Better plans and fewer wrong edits on hard tasks, at some token cost on easy ones (adaptive keeps that small).

## Implementation

The Anthropic provider adds `thinking: {type: adaptive}` for claude-opus/sonnet 4.6+ and claude-fable-*; Haiku, older and unknown models get no field. Thinking blocks already round-trip as opaque blocks. `thinking = false` in config turns it off. Not yet verified against the live API.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-providers `adaptive_thinking_for_supported_models`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
