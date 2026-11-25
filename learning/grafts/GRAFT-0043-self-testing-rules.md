---
id: GRAFT-0043
title: Approval rules carry examples checked at load
kind: adopt
status: trial
seam: approval
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex (rust-v0.172.0 execpolicy) lets each prefix rule list `match` and `not_match` example commands, validated when the policy loads.

## Why it matters

Wildcard rules are easy to get subtly wrong (`git status*` vs `git  status`, `*.pem` vs a nested path); a broken deny fails open silently.

## Hypothesis

Rule typos are caught at startup instead of during a session, and rule files document themselves.

## Implementation

`[[rules]]` accepts `match = [...]` and `not_match = [...]`; `config::check_examples` rejects the whole config with an error naming the rule and example. Rules with no examples behave as before.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli `rule_examples_are_checked_at_load`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
