---
id: GRAFT-0065
title: Short model aliases
kind: adopt
status: trial
seam: provider
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Aider (`--model sonnet`) and Claude Code (`/model opus`) accept short names for current models.

## Why it matters

Full model ids are long and change with every release; users mistype them.

## Hypothesis

Fewer configuration errors on first use.

## Implementation

`oxgraft_providers::ALIASES` maps opus, sonnet, haiku and fable to the current Anthropic ids (case-insensitive); `oxgraft models` lists them; the parse error suggests them.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-providers `parses_model_strings`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
