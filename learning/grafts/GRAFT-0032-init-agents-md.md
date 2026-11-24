---
id: GRAFT-0032
title: `oxgraft init` writes AGENTS.md
kind: adopt
status: trial
seam: context
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex and Claude Code both offer /init, which has the agent study the repository and write its instruction file.

## Why it matters

Good project instructions are the cheapest quality lever; generating a verified first draft lowers the barrier to having one.

## Hypothesis

Repos initialised this way need fewer exploratory turns in later sessions.

## Implementation

`oxgraft init` runs a preset task through the normal loop (approvals, undo, sandbox all apply) asking for a verified, concise AGENTS.md, improving an existing one rather than replacing it.

## Evaluation

Unit and end-to-end tests cover the behaviour (CLI e2e `init_writes_agents_md_through_the_loop`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
