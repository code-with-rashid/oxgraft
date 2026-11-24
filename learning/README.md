# The learning system

Oxgraft's purpose is to **keep learning from every major open source coding
harness**: adopt what they do best, avoid what they get wrong, and prove each
change with evidence. This folder is the project's memory of that work.

```
learning/
├── watch.toml              which harnesses we track, their license policy, last-reviewed release
├── harnesses/<id>.md       what we know about each one, by seam: strengths, limitations, ideas
├── capability-matrix.md    seams × harnesses, with Oxgraft's current default marked
└── grafts/GRAFT-NNNN-*.md  one record per idea we adopt, trial, or reject
```

## Vocabulary

| Term | Meaning |
|---|---|
| **Rootstock** | Oxgraft's stable core: `oxgraft-seams` traits plus `oxgraft-core`. |
| **Seam** | One capability axis where harnesses differ (edit format, context, approval, sandbox, …). |
| **Scion / strategy** | One harness's answer to a seam, implemented in `oxgraft-strategies`. |
| **Donor** | The harness an idea came from. |
| **Graft** | The record of adopting (or deliberately avoiding) one idea, with evidence. |
| **The graft took** | The strategy beat the current default and was promoted in `profiles/default.toml`. |

## The loop

1. **Watch.** `pnpm watch` (and the weekly `harness-watch` workflow) compares
   each harness's latest release with `last_reviewed` in `watch.toml`.
2. **Triage.** Read the upstream changes since `last_reviewed`. Update
   `harnesses/<id>.md`, sorting each change under a seam. Move `last_reviewed`.
3. **Propose.** For each idea worth having, or each limitation worth fixing,
   open a graft from `grafts/TEMPLATE.md`. Status `proposed`.
4. **Implement.** Add a strategy with full `Provenance` (donor, license,
   origin, graft id). Never change `oxgraft-core` to fit one strategy; if a
   seam must change, write an ADR first. Status `trial`.
5. **Evaluate.** Run it against the current default and the `baseline`
   profile (and against the donor itself where possible) on the same tasks.
   Record numbers in the graft. Status `evaluated`.
6. **Decide.** A maintainer promotes (`adopted`, change `profiles/default.toml`),
   keeps it opt-in (`available`), or rejects it (`rejected`, keep the record).
   Update `capability-matrix.md`.

Each step has a skill in `.agents/skills/` so any AI harness follows the same
procedure.

## Rules

- **License policy is binding.** `adapt` → code may be adapted, with
  attribution in `NOTICE`. `clean-room` and `reference` → behaviour and docs
  only; never paste upstream source.
- **No graft without provenance.** Tests fail if a borrowed strategy lacks
  its donor, license, or graft id.
- **No promotion without evidence.** A graft that has not been evaluated
  cannot become the default. "It's what Codex does" is not evidence.
- **Limitations count as much as strengths.** A graft can be of kind `avoid`:
  it records a donor's weakness and the test that shows Oxgraft doesn't share it.
- **Rejected grafts stay.** They stop the next agent re-proposing the same idea.
