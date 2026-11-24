---
id: GRAFT-0036
title: Deny always wins; compound shell commands checked per part; configurable rules with reasons
kind: adopt
status: trial
seam: approval
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: "ordered_rules"
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code (v2.1.331 docs) evaluates deny before ask before allow regardless of specificity, and splits compound bash commands so a deny or ask on any part applies. Codex execpolicy (rust-v0.172.0) attaches a justification to forbidden rules. OpenCode (v1.19.6) uses last-match-wins, where a narrow allow can override a broad deny.

## Why it matters

With last-match-wins and whole-string matching, `bash: cargo *` approves `cargo test && rm -rf ~`, and users had no way to add rules at all.

## Hypothesis

No compound command is approved because only its first part matches an allow rule; denied calls tell the model what to do instead.

## Implementation

`ordered_rules`: any matching deny wins; else last match; bash commands split on && || ; | and newlines with the most restrictive result; `$(`/backticks at least ask; rules carry an optional justification surfaced to the model via a new `ApprovalPolicy::explain`. `[[rules]]` in user config (and trusted workspaces) append to the defaults. Defaults now also allow ls, todo and skill.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-strategies `deny_always_wins_and_compound_commands_are_checked_per_part`; CLI e2e `configured_rules_apply_with_reasons`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
