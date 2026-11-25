---
id: GRAFT-0061
title: Context lines around grep matches
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

OpenCode's and Claude Code's grep tools wrap ripgrep and expose context options (-A/-B/-C).

## Why it matters

A bare matching line rarely shows enough to act on; the model follows up with a read of the whole file.

## Hypothesis

Fewer follow-up reads after searches.

## Implementation

`grep` takes `context` (0-5). Matches print as `path:N: text`, context lines as `path-N- text`, groups separated by `--`, overlapping windows merged.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `grep_shows_context_lines`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
