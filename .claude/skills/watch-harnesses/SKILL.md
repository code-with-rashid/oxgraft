---
name: watch-harnesses
description: Review what changed in the open source coding harnesses Oxgraft tracks and record it. Use when asked to check upstream harnesses, review a harness, triage the weekly harness-watch issue, or update learning/harnesses profiles.
---

# Watch harnesses

Turns upstream changes into Oxgraft knowledge. Read `learning/README.md` first.

## Steps

1. Run `pnpm watch` (set `GITHUB_TOKEN` if available). It lists each harness
   in `learning/watch.toml` whose latest release differs from `last_reviewed`.
   If the user named one harness, do only that one.
2. Pick at most **two** harnesses per session; depth beats coverage.
3. For each, read the upstream release notes / changelog / commits between
   `last_reviewed` and the latest release. First review (empty
   `last_reviewed`): read the README, docs and architecture overview instead.
4. Respect the harness `policy`: for `clean-room` and `reference`, read docs,
   changelogs and issues only, never source.
5. Update `learning/harnesses/<id>.md`:
   - fill the **By seam** table (edit_format, context, approval, sandbox, and
     notes on planned seams: provider, tools, session, extension, delegation);
   - add concrete items under **Strengths worth grafting** and
     **Limitations to avoid**, each with a source link;
   - append a row to **Change log of our reviews**;
   - set `status` to `reviewed` (or `partial`) and `last_reviewed` to the tag.
6. Set the same `last_reviewed` in `learning/watch.toml`.
7. Update the harness row in `learning/capability-matrix.md`.
8. For each idea that is clearly worth trying, follow the `propose-graft`
   skill. Don't implement in the same session unless asked.
9. Run `pnpm check:repo`, then the `session-handoff` skill.

## Quality bar

- Every claim links to an upstream source at a specific version.
- Separate what the harness *claims* from what you *verified*.
- Write limitations as testable statements ("permission rules don't confine
  child processes"), so they can become `avoid` grafts.
