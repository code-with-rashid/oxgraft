---
id: GRAFT-0041
title: Consecutive-mistake breaker
kind: adopt
status: trial
seam: session
donor: cline
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Cline (v4.1.27, verified mistake-tracker.ts) counts consecutive mistakes per session with a maximum; OpenHands' stuck detector catches action+error loops.

## Why it matters

The loop guard only catches identical calls; a model can also flail with many different failing calls.

## Hypothesis

Runs stuck in error streaks end early with a clear reason instead of exhausting max turns.

## Implementation

After 6 consecutive failed tool calls a reconsider note is added to the results; at 12 the run stops with `12 consecutive tool errors`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `protected_paths_always_ask_and_mistakes_trip_the_breaker`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
