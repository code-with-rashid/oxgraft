---
id: kilocode
repo: Kilo-Org/kilocode
license: MIT
policy: adapt
last_reviewed: "v7.8.32"
status: reviewed
---

# Kilo Code

[Kilo-Org/kilocode](https://github.com/Kilo-Org/kilocode) · MIT · policy `adapt`

Reviewed at `v7.8.32` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

> CLI is an OpenCode fork; README says main takes no new features (work moved to a v2 branch).

Sources: [https://github.com/Kilo-Org/kilocode/blob/v7.8.32](https://github.com/Kilo-Org/kilocode/blob/v7.8.32) (README, docs, and source where marked Verified).

## Strengths worth grafting

- Per-step snapshots in a separate git repo outside the project, project .git untouched (docs: checkpoints.md)
- Optional OS sandbox that fails closed when it cannot be enforced (docs: sandboxing.md)
- Compaction updates the previous summary rather than starting over

## Limitations to avoid

- Last-match-wins permission rules: a trailing catch-all silently overrides earlier allows
- `kilo run --auto` approves everything not explicitly denied
- Sandbox has no Windows support

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v7.8.32 | research agent for the Oxgraft build session | OpenCode fork; fail-closed sandbox, snapshots outside project .git; last-match rules, --auto |
