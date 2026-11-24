---
status: accepted
date: 2026-11-24
---

# ADR 0003: A recorded learning loop

## Context

Oxgraft's differentiator is continuous learning from other harnesses. Without
a record, ideas get re-proposed, provenance is lost, and "Codex does it" gets
mistaken for evidence.

## Decision

- `learning/watch.toml` is the list of tracked harnesses, each with a license
  `policy` (adapt, clean-room, reference) and `last_reviewed` upstream version.
- `learning/harnesses/<id>.md` holds what we know, organised by seam.
- `learning/grafts/GRAFT-NNNN-*.md` records each idea (kind adopt or avoid)
  through proposed → trial → evaluated → adopted / available / rejected.
- A weekly workflow reports upstream drift; skills in `.agents/skills/` define
  how any AI agent performs each step.
- `scripts/check-learning.mjs` validates the records in CI.

## Consequences

- Any agent (human or AI) can continue the work from the files alone.
- License policy is checked mechanically where possible.
- Some overhead per idea; acceptable because rejected ideas are cheap to keep
  and expensive to rediscover.
