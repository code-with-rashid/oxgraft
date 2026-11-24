---
id: pi
repo: earendil-works/pi
license: MIT
policy: adapt
last_reviewed: "v1.6.0"
status: reviewed
---

# Pi

[earendil-works/pi](https://github.com/earendil-works/pi) · MIT · policy `adapt`

Reviewed at `v1.6.0` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

Base: https://github.com/earendil-works/pi/blob/v1.6.0 — core harness in `packages/coding-agent`.

## By seam

| Seam | What it does | Strength or limitation? | Source |
|---|---|---|---|
| edit_format | `edit` with `edits[]` of `{oldText,newText}`, each matched against the original, unique and non-overlapping; repairs `edits` sent as a JSON string; exact then fuzzy (NFKC, trailing whitespace, smart quotes); keeps CRLF/BOM. **Verified** | Strength | [edit.ts](https://github.com/earendil-works/pi/blob/v1.6.0/packages/coding-agent/src/core/tools/edit.ts), [edit-diff.ts](https://github.com/earendil-works/pi/blob/v1.6.0/packages/coding-agent/src/core/tools/edit-diff.ts) |
| context | AGENTS.md / CLAUDE.md / AGENTS.override.md; lazy skill bodies; compaction at window − 16k reserve, keeps last 20k tokens, cumulative read/modified file lists across compactions | Strength | [compaction.md](https://github.com/earendil-works/pi/blob/v1.6.0/packages/coding-agent/docs/compaction.md) |
| approval | None built in; left to extensions | Limitation | [security.md](https://github.com/earendil-works/pi/blob/v1.6.0/packages/coding-agent/docs/security.md) |
| sandbox | None built in; Docker, credential-substituting proxy, micro-VM extension suggested | Limitation | [containerization.md](https://github.com/earendil-works/pi/blob/v1.6.0/packages/coding-agent/docs/containerization.md) |
| provider | 25+ providers, `/login`, custom providers | Strength | [providers.md](https://github.com/earendil-works/pi/blob/v1.6.0/packages/coding-agent/docs/providers.md) |
| tools | read, write, edit, bash, powershell, ls, find, grep; pluggable bash backend; per-file mutation queue by realpath. **Verified** | Strength | [file-mutation-queue.ts](https://github.com/earendil-works/pi/blob/v1.6.0/packages/coding-agent/src/core/tools/file-mutation-queue.ts) |
| session | JSONL tree (`id`/`parentId`): branch, fork, clone; append-only compaction. **Verified** | Strength | [session-manager.ts](https://github.com/earendil-works/pi/blob/v1.6.0/packages/coding-agent/src/core/session-manager.ts) |
| extension | In-process TS extensions (~20 events), npm/git packages, project trust for `.pi/*` | Strength; no capability boundary | [extensions.md](https://github.com/earendil-works/pi/blob/v1.6.0/packages/coding-agent/docs/extensions.md) |
| delegation | Deliberately none | Limitation | [README](https://github.com/earendil-works/pi/blob/v1.6.0/README.md) |

## Strengths worth grafting

- Batched multi-edits matched against the original, with argument repair.
- Compaction that carries cumulative read/modified file lists.
- Session tree with branching from any entry.
- Per-file mutation queue for parallel tool calls.
- Pluggable shell backend (VM / remote execution).

## Limitations to avoid

- Destructive bash runs without any prompt when no extension is installed.
- Context files load even when project trust is declined; project `sessionDir` read before trust.
- In-process extensions can read credentials and session history.

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v1.6.0 | research agent for the Oxgraft build session | batched edits with repair, file lists across compaction, session tree; no approval or sandbox |
