---
status: accepted
date: 2026-11-24
---

# ADR 0002: Seams and strategies

## Context

Harnesses differ on the same few questions: edit format, context management,
approval, sandboxing, tools, sessions, extensions, delegation. To keep
adopting ideas for years without rewriting the core, those differences must
be isolated.

## Decision

- Each question is a **seam**: a trait in `oxgraft-seams`.
- Each answer is a **strategy** in `oxgraft-strategies`, with `StrategyInfo`
  and mandatory `Provenance` (origin, donor harness, license, graft id).
- `oxgraft-core` depends on seams, never on strategies. The binary injects
  every strategy; a **profile** (`profiles/*.toml`) selects one per seam.
- Changing or adding a seam requires an ADR.

## Consequences

- A borrowed idea is an additive change that can be A/B tested by profile.
- Profiles like `baseline`, `default` and per-graft profiles make evaluation
  a configuration change, not a code change.
- Seams must be designed carefully up front; a leaky seam forces ADRs. Start
  with four (edit_format, context, approval, sandbox) and add the rest
  (provider, tools, session, extension, delegation) as M2–M3 need them.
