---
id: GRAFT-0040
title: Protected paths always need an explicit yes
kind: adopt
status: trial
seam: approval
donor: qwen-code
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Qwen Code (v0.28.4 docs) routes writes to self-modification and persistence paths (its settings, QWEN.md, AGENTS.md, hooks, skills, .git/, CI workflows) away from any auto-approval fast path.

## Why it matters

An agent that can silently rewrite its own instructions, hooks, CI or git internals can persist a prompt injection or escalate its own permissions.

## Hypothesis

No write to a protected path happens without an explicit user yes, under any profile or rule.

## Implementation

edit/write on .oxgraft/, .git/, .github/workflows/, .agents/, .claude/, .gemini/, .mcp.json and AGENTS.md/CLAUDE.md/GEMINI.md (any directory) are forced to `ask`, ignoring allow rules, allow_all and earlier `always` answers.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `protected_paths_always_ask_and_mistakes_trip_the_breaker`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
