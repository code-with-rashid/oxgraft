---
id: GRAFT-0053
title: Repo map tool: ranked outline under a budget
kind: adopt
status: trial
seam: context
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Aider (v0.86.0) sends a repo map: tree-sitter definitions ranked by a dependency graph, cut to `--map-tokens`.

## Why it matters

In an unfamiliar repo the model spends many turns on ls/grep/read to find where things are.

## Hypothesis

Fewer exploration turns before the first relevant read on unfamiliar codebases.

## Implementation

Clean-room, simpler: `map` tool with per-language definition regexes (no tree-sitter), files ranked by how many other files use the names they define, cut to a character budget; on demand rather than in every prompt. Read-only, allowed by default, available to subagents.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `maps_definitions_ranked_by_use`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
