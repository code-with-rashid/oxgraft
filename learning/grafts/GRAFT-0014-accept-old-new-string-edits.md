---
id: GRAFT-0014
title: Accept old_string/new_string edit inputs
kind: avoid
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

Models are trained against harnesses whose edit tools take `old_string`/`new_string` (Claude Code, OpenCode). Given a different schema they sometimes still send those fields.

## Why it matters

Rejecting a well-formed edit because of field names wastes a turn and can derail weaker models.

## Hypothesis

Lower edit-tool error rate for models from other ecosystems, at no cost to others.

## Implementation

The `edit` tool accepts `old_string`/`new_string`/`replace_all` when `edit` is absent, with the same uniqueness rules.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `edit_accepts_old_new_string_inputs`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
