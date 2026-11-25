---
id: GRAFT-0075
title: Quiet runs and last-message output for scripts
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

Codex `exec --output-last-message` and Claude Code's print mode (`-p`) give scripts just the final answer.

## Why it matters

Shell scripts and CI steps want the answer, not the tool chatter or a JSON stream to parse.

## Hypothesis

Oxgraft composes with shell pipelines (`oxgraft run -q "…" | …`).

## Implementation

`run --quiet/-q` prints only the final answer on stdout (warnings and stop reasons on stderr); `--output-last-message FILE` writes it to a file; `-q` conflicts with `--json`.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `quiet_runs_print_only_the_answer`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
