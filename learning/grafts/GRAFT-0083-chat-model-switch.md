---
id: GRAFT-0083
title: Switch model inside chat
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

Aider (`/model`) and Claude Code (`/model`) switch models mid-session, keeping the conversation.

## Why it matters

Users want a cheap model for exploration and a strong one for the hard part, or to retry a step on another provider, without losing context.

## Hypothesis

Lower cost per task with the same outcomes when users mix models.

## Implementation

Chat `/model M` (model string or alias) builds the provider with the session's effort/thinking options and points the agent at it; `/model` shows the current one. The edit format chosen at startup stays. Switching providers mid-history drops provider-specific blocks the new provider can't use (they are kept in the session).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `chat_model_switches_provider`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
