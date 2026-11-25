---
id: GRAFT-0081
title: A cheaper model for the read-only subagent
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

Claude Code's built-in Explore subagent runs on a smaller, faster model; OpenCode lets each agent pick its model.

## Why it matters

The `task` subagent only searches and summarises; running it on the main (largest) model costs more and is slower than needed.

## Hypothesis

Exploration gets cheaper and faster with no loss in final quality.

## Implementation

`subagent_model` (any model string or alias) builds a second provider; `Agent.subagent` holds it and `backend(depth)` picks it for depth > 0. Like `model`, an untrusted workspace can't set it.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `subagent_uses_its_own_model`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.

## Revision (2026-11-25, independent review)

See iterations 117-118 in learning/iterations.md: fixed after an independent review of iterations 103-116.
