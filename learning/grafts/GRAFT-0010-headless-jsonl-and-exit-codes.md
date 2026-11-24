---
id: GRAFT-0010
title: Headless JSONL events and meaningful exit codes
kind: adopt
status: trial
seam: session
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex has a non-interactive exec mode with JSON output; Claude Code has print mode with stream-json; Pi offers JSONL and RPC modes.

## Why it matters

CI, scripts and agentic-arena need a machine-readable stream and exit codes that distinguish success, usage errors and guard stops.

## Hypothesis

Arena adapters need no output scraping.

## Implementation

`oxgraft run --json` prints only protocol events on stdout; exit codes 0/1/2/3 per SPEC-0004.

## Evaluation

Unit and end-to-end tests cover the behaviour (CLI e2e `run_reads_edits_runs_and_reports`, `exit_codes`; SPEC-0004 AC1-AC2). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
