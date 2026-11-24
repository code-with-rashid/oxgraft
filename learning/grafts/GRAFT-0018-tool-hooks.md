---
id: GRAFT-0018
title: pre_tool / post_tool / session_end hooks
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

Claude Code runs user-configured hooks before and after tool calls; exit code 2 blocks a call or feeds stderr back to the model.

## Why it matters

Teams enforce policy (no `rm -rf`, run the formatter after edits) deterministically instead of hoping the model complies.

## Hypothesis

Formatter/linter hooks reduce follow-up fix turns; blocking hooks give zero policy violations.

## Implementation

`oxgraft-core::hooks`: TOML `[[hooks]]` with event, tool filter, command, timeout; env vars carry tool name, input and output; exit 2 = block (pre) or feedback (post).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `matching_and_exit_codes`, `hooks_block_and_give_feedback`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
