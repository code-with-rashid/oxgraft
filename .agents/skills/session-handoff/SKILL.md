---
name: session-handoff
description: Leave the Oxgraft repo ready for the next AI agent session. Use at the end of every working session, before committing, or when asked to wrap up or hand off.
---

# Session handoff

The next session may be a different harness with no memory of this one.

1. Run `pnpm check`. If you can't build locally, say so in `STATUS.md` and
   rely on CI; never claim it passed if you didn't run it.
2. Update `STATUS.md`:
   - `_Last updated_` date and which tool/agent you are;
   - **Works now**: add only what is verified;
   - **In flight**: anything half-done, with the exact next step;
   - **Next up**: the 1–3 most valuable next tasks;
   - **Known problems**: anything broken or unverified.
3. Tick finished items in `ROADMAP.md`.
4. If you learned something about the repo that every agent should know,
   add it to `AGENTS.md` (not to a tool-specific file).
5. Commit with a Conventional Commit message describing the change. No AI
   attribution trailers.
