---
id: GRAFT-0048
title: Show what a run changed and how to undo it
kind: adopt
status: trial
seam: session
donor: swe-agent
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

SWE-agent (v1.1.0) has the agent review its full diff before submitting; Cline and Aider show changed files per step.

## Why it matters

After an autonomous run the user has to discover what changed; changes made by shell commands are easy to miss.

## Hypothesis

Users notice unintended changes immediately and roll them back in one command.

## Implementation

The CLI compares the run's first and last checkpoints (GRAFT-0046) and prints `git diff --stat` plus `oxgraft restore N --session <id>` after `run` and after each chat turn. The model-side pre-finish review is not implemented yet.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `human_runs_end_with_a_change_summary`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
