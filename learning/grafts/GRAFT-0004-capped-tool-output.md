---
id: GRAFT-0004
title: Cap tool output, keeping head and tail
kind: adopt
status: trial
seam: tools
donor: swe-agent
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

SWE-agent's agent-computer interface work showed that concise, bounded observations improve agent performance compared with raw, unbounded command output.

## Why it matters

A single `cat` of a log or a noisy test run can flood the context window and push out the task.

## Hypothesis

Fewer context-overflow failures and lower input tokens on tasks with noisy commands, at equal pass rate.

## Implementation

Every tool result is capped (default 30 000 chars) by `util::truncate`, keeping 60% head and 40% tail with an explicit truncation note; `read` pages with offset/limit; grep and glob cap result counts.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `truncation_keeps_head_and_tail`, `output_is_truncated`; SPEC-0002 AC3). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
