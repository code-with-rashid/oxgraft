---
id: GRAFT-0101
title: Structured final answers with a JSON schema
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

Codex `exec --output-schema` makes the final message conform to a JSON schema for use in scripts and CI.

## Why it matters

Scripts that consume an agent's answer need machine-readable output; prose answers break pipelines.

## Hypothesis

Headless runs can feed their result straight into other tools.

## Implementation

`--output-schema FILE` adds the schema to the system prompt and checks the final answer with a small validator (type, required, properties, items, enum; code fences tolerated); problems go back to the model, sharing the finish-check budget. The answer is printed (and written by `--output-last-message`) as compact JSON.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `checks_answers_against_a_schema`; oxgraft-cli e2e `output_schema_yields_valid_json`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
