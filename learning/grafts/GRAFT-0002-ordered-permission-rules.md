---
id: GRAFT-0002
title: Ordered allow/ask/deny permission rules per tool and resource
kind: adopt
status: trial
seam: approval
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ordered_rules
opened: 2026-11-24
decided: ""
---

## Observation

OpenCode governs tools with ordered `allow` / `ask` / `deny` rules matched by
action and resource pattern (see agentic-arena `docs/harnesses/opencode.md`).
agentic-arena's review also notes the limitation: these rules control tools
but are **not** an outer sandbox.

## Why it matters

One global "auto-approve" switch is too coarse: users want `cargo test` to
run freely but `rm` or `.env` reads to stop. Ordered rules express that with
little configuration.

## Hypothesis

Fewer approval prompts per task than a fixed ask-for-every-write policy, with
zero unprompted writes outside the rule set in the arena's boundary tasks.

## Implementation

`crates/oxgraft-strategies/src/approval/mod.rs`, last matching rule wins,
default `ask`. Kind `adopt` for the rules; the sandbox limitation is tracked
separately as a future `avoid` graft on the sandbox seam (candidate donor:
Codex's Landlock/Seatbelt sandboxing).

## Evaluation

Pending: needs tools (M2) and agentic-arena's boundary-response arena (M4).

## Decision

Pending.
