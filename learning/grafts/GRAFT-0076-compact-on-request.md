---
id: GRAFT-0076
title: Compact the context on request
kind: adopt
status: trial
seam: context
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's `/compact` (and OpenCode's summarise) let the user compact a long conversation before switching topic, instead of waiting for the automatic threshold.

## Why it matters

Long chat sessions get slower and costlier per turn well before the 80% threshold; users know when an old topic is done.

## Hypothesis

Long chats stay cheaper and more focused.

## Implementation

`Agent::compact_session` runs the same handover-summary compaction as GRAFT-0023 on demand; chat `/compact` calls it. History is replaced by task + summary, never edited in place.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `sessions_compact_on_request`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
