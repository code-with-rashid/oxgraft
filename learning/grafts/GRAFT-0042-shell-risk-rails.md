---
id: GRAFT-0042
title: Deterministic risk rails for shell commands
kind: adopt
status: trial
seam: approval
donor: openhands
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenHands (software-agent-sdk v1.64.0, security/defense_in_depth/policy_rails.py) forces HIGH risk for fetch-to-exec, raw-disk writes and catastrophic deletes regardless of what the model labels the action.

## Why it matters

Allow rules such as `bash *` or an 'always' answer are broad; the commands that do irreversible damage are a small, recognisable set and should never ride on a blanket approval.

## Hypothesis

No pipe-to-shell, forced recursive delete, raw disk write, mkfs or sudo runs without a yes given for that exact command.

## Implementation

`oxgraft_strategies::approval::shell_risk` checks each simple command (and the whole line for pipes); `ordered_rules` turns a matching allow into ask and reports the reason through the new `needs_fresh_yes` seam method (ADR 0006), so core ignores 'always' for it. Deny rules still win; allow_all is untouched.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-strategies `rails_flag_dangerous_commands`, `risk_rails_beat_allow_rules`; oxgraft-core `risky_commands_ignore_always_answers`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
