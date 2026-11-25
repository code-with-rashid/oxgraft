---
id: GRAFT-0079
title: Detect the project's check command
kind: adopt
status: trial
seam: session
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Aider lints changed files automatically with a per-language default linter and fixes what it reports.

## Why it matters

`verify_command` (GRAFT-0039) only helps users who configure it; a per-project default makes the edit-check-fix loop work out of the box.

## Hypothesis

More runs end with code that builds, without per-project setup.

## Implementation

`verify_command = "auto"` picks `cargo check --quiet --all-targets`, `go vet ./...`, the package.json `typecheck`/`check`/`lint` script (pnpm/yarn/bun/npm from the lockfile) or a Python compile. These run repository code, so `auto` applies only in trusted workspaces (warning otherwise).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli `verify_command_auto_detects_the_project_check`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.

## Revision (2026-11-25, independent review)

See iterations 117-118 in learning/iterations.md: fixed after an independent review of iterations 103-116.
