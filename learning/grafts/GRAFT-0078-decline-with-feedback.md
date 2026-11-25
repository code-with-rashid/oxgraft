---
id: GRAFT-0078
title: Decline an action and say what to do instead
kind: adopt
status: trial
seam: approval
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's approval prompt has "No, and tell Claude what to do differently".

## Why it matters

A bare 'no' leaves the model guessing; it often retries something similar.

## Hypothesis

Fewer repeated proposals after a refusal.

## Implementation

At an approval prompt, any answer longer than a few characters that isn't yes/no/always is guidance: `Answer::NoWith(text)`, and the tool result says the user declined and what they said. `Answer` is no longer `Copy`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `declining_with_feedback_reaches_the_model`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
