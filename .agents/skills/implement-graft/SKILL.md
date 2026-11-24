---
name: implement-graft
description: Implement a proposed graft as a new Oxgraft strategy behind a profile, without changing the core. Use when asked to implement GRAFT-NNNN or to add a strategy learned from another harness.
---

# Implement a graft

1. Read the graft record and the donor profile in `learning/harnesses/`.
   Check the donor `policy` in `learning/watch.toml` before looking at any code.
2. Add a module in `crates/oxgraft-strategies/src/<seam>/`. Set `StrategyInfo`:
   unique `id`, `summary`, and full `Provenance` (`origin`, `harness`,
   `license`, `graft`, `notes`).
3. If you adapted code (policy `adapt` only): keep it minimal, note the
   upstream file and commit in a comment, and add the upstream copyright and
   license to `NOTICE`.
4. Register it in the seam's function in `crates/oxgraft-strategies/src/lib.rs`.
5. Write unit tests: the success path, every error variant, and the specific
   behaviour the hypothesis depends on.
6. Do **not** modify `oxgraft-core` or `oxgraft-seams` to fit it. If the seam
   is insufficient, stop and write an ADR.
7. Add a profile `profiles/graft-NNNN.toml` that differs from `default.toml`
   only in this strategy.
8. Set the graft `status: trial` and `strategy: <id>`; update
   `learning/capability-matrix.md`.
9. `pnpm check`, then the `session-handoff` skill.
