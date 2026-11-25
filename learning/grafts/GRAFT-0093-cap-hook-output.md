---
id: GRAFT-0093
title: Cap hook output sent to the model
kind: avoid
status: trial
seam: extension
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code v2.1.332 release notes: a hook that blocks or fails with a very large log caused a freeze and context overflow; text over about 50,000 characters is now shortened.

## Why it matters

Oxgraft passed a hook's whole stderr to the model as feedback; a noisy linter or test hook could flood the context.

## Hypothesis

No hook can overflow the context in one turn.

## Implementation

Hook stderr is cut to its last 20,000 characters with a note of how much was omitted (user_prompt context was already capped; verify output keeps its last 8,000).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `long_hook_output_is_shortened`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
