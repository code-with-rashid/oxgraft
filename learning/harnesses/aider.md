---
id: aider
repo: Aider-AI/aider
license: Apache-2.0
policy: adapt
last_reviewed: "v0.86.0"
status: reviewed
---

# Aider

[Aider-AI/aider](https://github.com/Aider-AI/aider) · Apache-2.0 · policy `adapt`

Reviewed at `v0.86.0` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

## By seam

| Seam | What it does | Strength or limitation? | Source |
|---|---|---|---|
| edit_format | whole, diff (SEARCH/REPLACE), diff-fenced, udiff, editor-diff/editor-whole; chosen per model. Failed blocks return "Did you mean" lines and note when REPLACE is already present. **Verified:** the edit-distance fuzzy fallback is unreachable (after a bare `return`). | Strength: per-model format, good error feedback. Limitation: dead fallback | [edit-formats.md](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/website/docs/more/edit-formats.md), [editblock_coder.py](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/coders/editblock_coder.py#L80-L188) |
| context | tree-sitter repo map ranked on a dependency graph within `--map-tokens` (default 1k); `/add`, `/read-only`; history summarisation | Strength | [repomap.md](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/website/docs/repomap.md) |
| approval | Interactive prompts; **Verified:** shell commands need explicit yes and `--yes-always` answers no to them; no rule language | Mixed | [io.py](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/io.py#L865-L866) |
| sandbox | None; `/run` and suggested commands run on the host | Limitation | [commands.md](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/website/docs/usage/commands.md) |
| provider | Many LLMs; main/weak/editor models; per-model settings | Strength | [llms.md](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/website/docs/llms.md) |
| tools | No model-callable tool loop; user-driven /run, /test, /lint, /web; auto-lint after edits with **Verified** `max_reflections = 3` | Strength: lint→fix loop. Limitation: not agentic | [lint-test.md](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/website/docs/usage/lint-test.md) |
| session | Every edit is a git commit; `/undo`; pre-commit hooks skipped by default; history restore off by default | Strength: git as undo. Limitation: skipped hooks | [git.md](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/website/docs/git.md) |
| extension | No MCP/plugins/hooks; `--read` conventions; `--watch-files` AI comments | Limitation | [options.md](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/website/docs/config/options.md) |
| delegation | Architect (plan) + editor (apply) models | Strength | [modes.md](https://github.com/Aider-AI/aider/blob/v0.86.0/aider/website/docs/usage/modes.md) |

## Strengths worth grafting

- Ranked repo map under a token budget.
- Lint after every edit with capped fix rounds (3).
- Architect/editor model split.
- Failed-edit feedback that names the nearest real lines and detects already-applied edits.
- Auto-approve never auto-runs shell commands.
- Weak model for cheap work (commit messages).

## Limitations to avoid

- Unreachable fallback code in the edit pipeline.
- Harness commits that skip pre-commit hooks.
- Shell commands straight to the host with no sandbox or policy layer.
- Unsupported scripting API; resumed sessions that don't restore the conversation by default.

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v0.86.0 | research agent for the Oxgraft build session | repo map, lint-fix loop, per-model edit formats; dead fuzzy fallback, no sandbox |
