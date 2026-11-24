# Status

The handoff file. Every agent session reads this first and updates it last.
Keep it short: current truth, not history (history is in git and `learning/`).

_Last updated: 2026-11-24 by the scaffold session._

## Current milestone

**M1: Rootstock.** See `ROADMAP.md`.

## Works now

- Cargo workspace: `oxgraft-seams`, `oxgraft-strategies`, `oxgraft-core`,
  `oxgraft-protocol`, `oxgraft-cli` (binary `oxgraft`).
- Four seams (edit_format, context, approval, sandbox) with five strategies,
  each with provenance and unit tests.
- `oxgraft strategies` lists strategies and donors; `oxgraft profile <path>`
  validates a profile and prints the session preamble as JSONL.
- Learning system: 17 harnesses in `learning/watch.toml`, a profile stub per
  harness (8 seeded from agentic-arena's reviews), GRAFT-0001 and GRAFT-0002 in trial.
- Repo checks (`pnpm check:repo`), skills mirror, upstream watcher, docs site, CI.

## In flight

- Nothing.

## Next up

1. First real review of `codex` and `opencode` with the `watch-harnesses`
   skill (they cover the most seams); set their `last_reviewed`.
2. M2: the model loop (provider seam ADR, Anthropic provider, tool set).

## Known problems

- The scaffold was written on a machine without a Rust toolchain; the first
  CI run is the first compile. If CI is red, fixing it is the top task.
- No harness has been fully reviewed yet; capability-matrix cells are mostly "?".
- Package names (`oxgraft` on crates.io, npm, PyPI) are not published yet.
