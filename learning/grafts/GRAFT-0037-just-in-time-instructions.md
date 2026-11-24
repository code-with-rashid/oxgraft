---
id: GRAFT-0037
title: Just-in-time subdirectory instructions
kind: adopt
status: trial
seam: context
donor: gemini-cli
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Gemini CLI (v0.71.1 docs) loads GEMINI.md files just in time from the directories of files that tools touch, bounded by the trusted root.

## Why it matters

Monorepos keep package-specific rules (package manager, test command) in subdirectory instruction files that a root-only loader never sees.

## Hypothesis

Fewer convention mistakes in monorepo subpackages, at the cost of a few hundred tokens only when relevant.

## Implementation

After read/write/edit succeeds, AGENTS.md / CLAUDE.md / GEMINI.md files between the file and the workspace root (exclusive) are appended once per run to that tool result, keeping history append-only.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `nested_instructions_are_shown_once_when_touched`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
