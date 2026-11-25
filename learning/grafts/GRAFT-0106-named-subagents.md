---
id: GRAFT-0106
title: Named subagents with their own instructions and tools
kind: adopt
status: trial
seam: core
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode defines agents in markdown files with a description, tools and a prompt; Claude Code loads subagents from `.claude/agents/*.md` and the model picks one by name.

## Why it matters

One generic explorer prompt fits every delegation poorly; teams already keep reviewer or test-finder agents in their repos.

## Hypothesis

Delegations that match a named agent come back more focused, and existing `.claude/agents` files work unchanged.

## Implementation

The CLI loads `<name>.md` from `.agents/agents/` and `.claude/agents/` (home first, then the workspace only when trusted). Front matter `name`, `description`, `tools` (Claude-style names are lowercased); the body is the instructions (8,000 chars max). `task` lists the agents and takes `agent`; the subagent gets the instructions before its prompt and only the listed tools, intersected with the read-only subagent set, so a file can narrow but never widen what a subagent can do. Unknown agent names return an error listing the available ones.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `named_subagents_get_their_instructions_and_tools`; oxgraft-cli `agents_load_from_trusted_workspaces_only`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
