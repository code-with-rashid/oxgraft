---
id: GRAFT-0029
title: Preview edits and writes in the approval prompt
kind: adopt
status: trial
seam: approval
donor: cline
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Cline shows a diff for approval before applying changes; Claude Code shows the proposed edit in its permission prompt.

## Why it matters

Users cannot meaningfully approve a change they cannot see.

## Hypothesis

Users reject harmful edits more often and approve faster.

## Implementation

The terminal approver prints up to 40 lines of the edit or written content before asking.

## Evaluation

Unit and end-to-end tests cover the behaviour (manual; covered indirectly by approval tests). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
