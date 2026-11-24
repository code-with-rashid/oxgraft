---
name: propose-graft
description: Write a graft record proposing that Oxgraft adopt a strength from another coding harness, or avoid one of its limitations. Use after a harness review, or when someone suggests "do it like <harness>".
---

# Propose a graft

1. Search `learning/grafts/` for an existing record on the same idea,
   including rejected ones. If one exists, update it instead; if it was
   rejected, only reopen it with new evidence.
2. Copy `learning/grafts/TEMPLATE.md` to
   `learning/grafts/GRAFT-NNNN-<short-slug>.md`, with NNNN one above the
   highest existing number.
3. Fill the frontmatter. `origin` must agree with the donor's `policy` in
   `learning/watch.toml` (`adapt` → `adapted` or `clean-room`; otherwise `clean-room`).
4. Write **Observation** with source links at `donor_version`.
5. Write a **Hypothesis** that can be measured: which metric, which tasks,
   what result would make you reject it.
6. Name the seam. If no seam fits, say so and draft an ADR in `docs/adr/`
   instead of forcing it.
7. Add the idea to the "Ideas to evaluate" column of
   `learning/capability-matrix.md`.
8. Status stays `proposed`. Run `pnpm check:repo`.
