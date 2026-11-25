---
id: GRAFT-0062
title: user_prompt and stop hook events
kind: adopt
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

Claude Code has UserPromptSubmit hooks (add context or block a prompt) and Stop hooks (exit 2 keeps the agent working); Gemini CLI has similar model-stage hooks.

## Why it matters

Teams want to inject context (ticket text, conventions) automatically and enforce 'not done until X' with their own scripts.

## Hypothesis

Project policy can be enforced at the start and end of a task without changing Oxgraft.

## Implementation

Hook events `user_prompt` (env OXGRAFT_PROMPT; stdout appended to the task as context; exit 2 stops the run with the hook's message) and `stop` (env OXGRAFT_FINAL_MESSAGE; exit 2 sends stderr back and continues, at most 3 times). Same trust gate as other hooks.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `user_prompt_and_stop_hooks`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
