---
id: GRAFT-0109
title: Filter background output by regex
kind: adopt
status: trial
seam: tools
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's BashOutput tool takes an optional `filter` regex and returns only matching lines.

## Why it matters

Dev servers and watchers print far more than the model needs; reading it all costs context on every check.

## Hypothesis

Checking a long-running server for errors costs a few lines of context instead of its whole log.

## Implementation

`bash_output` accepts `filter` (a regex, compiled with a 1 MB size limit). Only matching new lines are returned; the rest still count as read. No matches gives `(none of N new lines matched)`; an invalid regex is an error.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `output_can_be_filtered`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
