---
id: GRAFT-0060
title: Save truncated shell output in full
kind: adopt
status: trial
seam: tools
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode writes tool output that exceeds its limit to a file and gives the model the path; Claude Code similarly persists large outputs.

## Why it matters

Head-and-tail truncation hides the middle of long logs (the first compiler error is often there); re-running the command to see it wastes time.

## Hypothesis

Models find errors in long outputs without re-running commands.

## Implementation

When bash output exceeds `max_output_chars`, the full text goes to `.oxgraft/outputs/bash-<ns>.log` (20 newest kept) and the truncated result names the file for `read`/`grep`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `long_output_is_saved_in_full`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
