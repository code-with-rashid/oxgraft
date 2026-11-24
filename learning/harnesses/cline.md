---
id: cline
repo: cline/cline
license: Apache-2.0
policy: adapt
last_reviewed: "v4.1.27"
status: reviewed
---

# Cline

[cline/cline](https://github.com/cline/cline) · Apache-2.0 · policy `adapt`

Reviewed at `v4.1.27` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

Base: https://github.com/cline/cline/blob/v4.1.27 — now a monorepo: SDK in `sdk/packages/{llms,agents,core}`, apps in `apps/`.

## By seam

| Seam | What it does | Strength or limitation? | Source |
|---|---|---|---|
| edit_format | `editor` exact str_replace (fails on 0 or >1 matches, guards `$`), `insert_line`; model routing enables `apply_patch` for openai-native/codex/GPT. **Verified** | Strength | [editor.ts](https://github.com/cline/cline/blob/v4.1.27/sdk/packages/core/src/extensions/tools/executors/editor.ts), [model-tool-routing.ts](https://github.com/cline/cline/blob/v4.1.27/sdk/packages/core/src/extensions/tools/model-tool-routing.ts) |
| context | Summarises history near the window limit; basic and agentic compaction | Strength | [auto-compact.mdx](https://github.com/cline/cline/blob/v4.1.27/docs/features/auto-compact.mdx) |
| approval | Auto-approve toggles per category; the model sets `requires_approval` on commands itself; plan mode blocks file-editing commands by blacklist (source admits `python -c` gaps). **Verified** | Limitation: model judges its own risk | [auto-approve.mdx](https://github.com/cline/cline/blob/v4.1.27/docs/features/auto-approve.mdx), [command-guard.ts](https://github.com/cline/cline/blob/v4.1.27/sdk/packages/core/src/extensions/tools/command-guard.ts) |
| sandbox | Not documented; editor confines paths to cwd. **Verified** | Limitation | [editor.ts](https://github.com/cline/cline/blob/v4.1.27/sdk/packages/core/src/extensions/tools/executors/editor.ts) |
| provider | `@cline/llms` model catalog and handler factory | Strength | [packages README](https://github.com/cline/cline/blob/v4.1.27/sdk/packages/README.md) |
| tools | read_files, search_codebase, run_commands, fetch_web_content, apply_patch, editor, skills, ask_question, submit_and_exit | Strength | [model-tool-routing.ts](https://github.com/cline/cline/blob/v4.1.27/sdk/packages/core/src/extensions/tools/model-tool-routing.ts) |
| session | Checkpoint after every tool use as private refs `refs/cline/checkpoints/<session>/` via commit-tree and a private GIT_INDEX_FILE (captures untracked files); consecutive-mistake breaker. **Verified** | Strength | [checkpoint-hooks.ts](https://github.com/cline/cline/blob/v4.1.27/sdk/packages/core/src/hooks/checkpoint-hooks.ts), [mistake-tracker.ts](https://github.com/cline/cline/blob/v4.1.27/sdk/packages/core/src/runtime/safety/mistake-tracker.ts) |
| extension | Subprocess hook files, plugins, MCP, skills, `.clineignore` | Strength | [hook-file-hooks.ts](https://github.com/cline/cline/blob/v4.1.27/sdk/packages/core/src/hooks/hook-file-hooks.ts) |
| delegation | `spawn_agent` parallel subagents; approving the spawn approves everything inside | Limitation | [subagents.mdx](https://github.com/cline/cline/blob/v4.1.27/docs/features/subagents.mdx) |

## Strengths worth grafting

- Edit tool routed by model/provider.
- Per-turn git-ref checkpoints with a private index (untracked files, user index untouched).
- Consecutive-mistake breaker separate from loop detection.
- Tool errors that say what to do instead.

## Limitations to avoid

- "Safe" command classification taken from the model's own flag.
- Subagent tool calls bypass approval settings.
- Plan-mode guard misses `python -c` / `bash -c` writes.
- Docs (shadow repo) disagree with source (refs in the user's `.git`).

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v4.1.27 | research agent for the Oxgraft build session | git-ref checkpoints, model-routed edit tool, mistake breaker; model self-classifies risk |
