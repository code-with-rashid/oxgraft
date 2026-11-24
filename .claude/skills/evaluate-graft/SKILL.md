---
name: evaluate-graft
description: Measure a graft's strategy against Oxgraft's current default and baseline profiles and record the evidence. Use when asked to evaluate, benchmark, or decide on GRAFT-NNNN.
---

# Evaluate a graft

1. Confirm the graft is `trial` and `profiles/graft-NNNN.toml` exists.
2. Choose tasks that test the hypothesis. Prefer agentic-arena tasks so
   results are comparable over time; add focused tasks under `evals/` if none fit.
3. Run the same tasks, same model, same seed/settings, with three profiles:
   `baseline`, `default`, `graft-NNNN`. Where practical, also run the donor
   harness itself.
4. Record in the graft's **Evaluation** table: tasks, pass rate, tokens,
   wall time, and the exact commands. Include failures, not just averages.
5. Write a recommendation (adopt / keep available / reject) with reasons.
   Set `status: evaluated`. Do not set `adopted`; a maintainer decides.
6. `pnpm check:repo`, then the `session-handoff` skill.

Until the agent loop and arena adapter exist (ROADMAP M2/M4), say so in the
record and stop at step 1: do not invent numbers.
