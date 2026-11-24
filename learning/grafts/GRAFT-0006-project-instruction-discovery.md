---
id: GRAFT-0006
title: Discover AGENTS.md, CLAUDE.md and GEMINI.md with @imports
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

Codex reads AGENTS.md; Claude Code reads CLAUDE.md and supports `@path` imports; Gemini CLI reads GEMINI.md with the same import style.

## Why it matters

Repositories already carry instructions for these tools. Reading all of them (once each) makes Oxgraft useful in existing repos with no new config.

## Hypothesis

Agents follow repo conventions (package manager, test command) without being told in the task.

## Implementation

`oxgraft-core::prompt::project_instructions` reads AGENTS.md, CLAUDE.md, GEMINI.md and .github/copilot-instructions.md, expands one level of `@file` imports, de-duplicates, and caps at 40 000 chars.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `discovers_instructions_and_resolves_imports_once`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
