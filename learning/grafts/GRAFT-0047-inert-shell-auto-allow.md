---
id: GRAFT-0047
title: Auto-allow inert read-only shell commands
kind: adopt
status: trial
seam: approval
donor: crush
donor_version: ""
license: FSL-1.1-MIT
origin: clean-room
strategy: "inert_reads"
opened: 2026-11-25
decided: ""
---

## Observation

Crush (v0.102.0 README; FSL-1.1-MIT, no source consulted) auto-runs read-only bash only when the whole command parses as inert: no redirection, substitution, pipeline, assignment or mutating argument.

## Why it matters

Asking before `ls` or `git status` trains users to approve without reading; fewer, more meaningful prompts make approvals safer.

## Hypothesis

Most prompts in exploration-heavy sessions disappear with no increase in unsafe actions.

## Implementation

Strategy `inert_reads` wraps `ordered_rules`: a bash `ask` becomes `allow` when `is_inert` holds (single known reader, no shell metacharacters or globs, no mutating options, no absolute/home/parent paths) and no argument is a path the rules deny reading. Selected by profile `fewer-prompts`; the default is unchanged pending evaluation.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-strategies `recognises_inert_commands`, `inert_commands_are_allowed_but_rules_still_win`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
