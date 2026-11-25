---
id: GRAFT-0063
title: Report the files a headless run changed
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

SWE-agent submits the run's diff as its result; evaluation harnesses (including agentic-arena) need to know what an agent changed.

## Why it matters

Headless consumers had to diff the workspace themselves and missed changes made by shell commands.

## Hypothesis

Benchmarks and CI wrappers can score runs from the event stream alone.

## Implementation

Protocol event `files_changed { files }` (append-only addition) emitted before `session_finished`, from `git diff --name-only -z` between the run's first and last checkpoints (GRAFT-0046); omitted when nothing changed or checkpoints are off.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `json_runs_report_changed_files`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
