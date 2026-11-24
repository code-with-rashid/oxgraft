---
id: GRAFT-0039
title: Automatic verify command after edits
kind: adopt
status: trial
seam: tools
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Aider (v0.86.0) lints after every edit and feeds errors back for a capped number of fix rounds (verified `max_reflections = 3`).

## Why it matters

Models often stop after an edit that does not compile; an automatic check catches it while the context is fresh.

## Hypothesis

Fewer sessions that end with a broken build, at the cost of one command per editing turn.

## Implementation

`verify_command` (user config, or trusted workspace) runs after any turn that changed files; on failure its output tail is appended to that turn's tool results; pauses after 3 consecutive failures. Untrusted workspaces cannot set it.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `verify_command_feeds_failures_back_after_edits`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
