---
id: opencode
repo: anomalyco/opencode
license: MIT
policy: adapt
last_reviewed: "v1.19.6"
status: reviewed
---

# OpenCode

[anomalyco/opencode](https://github.com/anomalyco/opencode) · MIT · policy `adapt`

Reviewed at `v1.19.6` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

## By seam

| Seam | What it does | Strength or limitation? | Source |
|---|---|---|---|
| edit_format | Exact-string `edit`, `write`, plus `apply_patch`. **Verified:** `edit` falls back through 9 matchers (Simple, LineTrimmed, BlockAnchor, WhitespaceNormalized, IndentationFlexible, EscapeNormalized, TrimmedBoundary, ContextAware, MultiOccurrence); the registry swaps in `apply_patch` for model ids containing `gpt-` (excluding `oss`, `gpt-4`). | Strength: model-aware edit format and a tolerant fallback chain. Limitation: model choice by id substring. | [edit.ts](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/opencode/src/tool/edit.ts), [registry.ts](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/opencode/src/tool/registry.ts) |
| context | AGENTS.md walking up from cwd plus a global one; falls back to CLAUDE.md (first match per category wins); auto-compaction on with a reserved buffer; tool-output pruning opt-in. | Strength: compatibility, configurable compaction. Limitation: AGENTS.md shadows CLAUDE.md. | [rules.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/rules.mdx), [config.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/config.mdx) |
| approval | allow/ask/deny per tool with wildcard input patterns, last match wins; once/always/reject; `--auto`; `doom_loop` asks on 3 identical calls; `.env` reads denied by default. | Strength: input-pattern rules, doom-loop prompt. Limitation: most tools (bash, edit) default to allow. | [permissions.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/permissions.mdx) |
| sandbox | None documented; **Verified:** no sandbox code in tree. `external_directory` is a path guard, not isolation. | Limitation | [permissions.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/permissions.mdx) |
| provider | AI SDK + Models.dev, 75+ providers, local models, baseURL override. | Strength: breadth | [providers.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/providers.mdx) |
| tools | bash, edit, write, read, grep, glob, apply_patch, skill, todowrite, webfetch, websearch, question; experimental `lsp`; LSP diagnostics fed back to the agent. | Strength: diagnostics as feedback | [tools.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/tools.mdx), [lsp.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/lsp.mdx) |
| session | Internal-git snapshots for undo/redo, on by default (docs warn about cost on large repos); /compact, /new, /share. | Strength: undo by default. Limitation: snapshot cost | [tui.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/tui.mdx) |
| extension | JS/TS plugins (local or npm) with many events; custom tools; MCP; skills. | Strength: big event surface. Limitation: plugins run in-process | [plugins.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/plugins.mdx) |
| delegation | Primary agents Build and Plan; subagents General, Explore (read-only), Scout; per-agent model, permissions, `steps` cap, hidden flag. | Strength: named agents with own policy. Limitation: no documented recursion limit | [agents.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/agents.mdx) |

## Strengths worth grafting

- Edit tool chosen per model family (apply_patch for GPT-5-class, str_replace otherwise). [registry.ts](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/opencode/src/tool/registry.ts)
- Ordered fallback matcher chain for failed exact edits. [edit.ts](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/opencode/src/tool/edit.ts)
- `doom_loop`: three identical calls turn into an approval prompt rather than a hard stop. [permissions.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/permissions.mdx)
- Per-agent `steps` cap that asks the agent to summarise and list remaining work when reached. [agents.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/agents.mdx)
- "Always" approvals that propose a safe command prefix (e.g. `git status*`) for the session. [permissions.mdx](https://github.com/anomalyco/opencode/blob/v1.19.6/packages/web/src/content/docs/permissions.mdx)

## Limitations to avoid

- With no config, `rm -rf` inside the workspace runs without a prompt (bash defaults to allow).
- Shell commands have no OS isolation and can write outside the workspace.
- A new model id silently gets the wrong edit tool because selection is by id substring.
- CLAUDE.md is ignored, not merged, when AGENTS.md exists in the same directory.
- Snapshot undo is costly on large or submodule-heavy repos (docs; not measured).

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v1.19.6 | research agent for the Oxgraft build session | 9-matcher edit fallback, model-aware apply_patch, doom_loop prompt; no sandbox, permissive defaults |
