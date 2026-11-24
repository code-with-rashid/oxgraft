---
id: goose
repo: aaif-goose/goose
license: Apache-2.0
policy: adapt
last_reviewed: "v1.60.2"
status: reviewed
---

# goose

[aaif-goose/goose](https://github.com/aaif-goose/goose) · Apache-2.0 · policy `adapt`

Reviewed at `v1.60.2` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

## By seam

| Seam | What it does | Strength or limitation? | Source |
|---|---|---|---|
| edit_format | developer extension `write` and `edit` (before/after, exactly one match); no-match → "Did you mean" + preview; multi-match → line numbers. **Verified** | Strength: clear errors | [edit.rs](https://github.com/aaif-goose/goose/blob/v1.60.2/crates/goose/src/agents/platform_extensions/developer/edit.rs#L145-L190) |
| context | `.goosehints`; auto-compaction at min(0.8 × window, 225k); Code Mode loads tools on demand | Strength | [smart-context-management.md](https://github.com/aaif-goose/goose/blob/v1.60.2/documentation/docs/guides/sessions/smart-context-management.md) |
| approval | Autonomous (default) / Manual / Smart Approve; per-tool always/ask/never; smart read/write split judged by the LLM | Limitation: unsafe default | [goose-permissions.md](https://github.com/aaif-goose/goose/blob/v1.60.2/documentation/docs/guides/managing-tools/goose-permissions.md) |
| sandbox | No OS sandbox in tree (**Verified**); `--container` runs extensions in Docker; adversary reviewer fails open; pattern-based injection detection | Limitation for isolation | [adversary-mode.md](https://github.com/aaif-goose/goose/blob/v1.60.2/documentation/docs/guides/security/adversary-mode.md) |
| provider | 15+ providers; ACP agents (Claude Code, Codex) usable as providers | Strength | [architecture](https://github.com/aaif-goose/goose/blob/v1.60.2/documentation/docs/goose-architecture/goose-architecture.md) |
| tools | All tools via MCP extensions; tool errors returned to the model | Strength | [error-handling.md](https://github.com/aaif-goose/goose/blob/v1.60.2/documentation/docs/goose-architecture/error-handling.md) |
| session | SQLite sessions; resume, fork, `--resume --edit` in $EDITOR; `--max-turns` default 1000 | Strength. Limitation: high turn cap | [session-management.md](https://github.com/aaif-goose/goose/blob/v1.60.2/documentation/docs/guides/sessions/session-management.md) |
| extension | MCP, skills, recipes, plugins with hooks (JSON stdin); PreToolUse `on_failure` defaults to allow | Limitation: hooks fail open | [hooks.md](https://github.com/aaif-goose/goose/blob/v1.60.2/documentation/docs/guides/context-engineering/hooks.md) |
| delegation | sequential/parallel subagents, extension-scoped, recipes; 5-minute timeout; failures return no output; ACP server | Limitation: silent failures | [subagents.mdx](https://github.com/aaif-goose/goose/blob/v1.60.2/documentation/docs/guides/context-engineering/subagents.mdx) |

## Strengths worth grafting

- Adversary reviewer model checking each call against the task and a rules file (but fail closed).
- Session fork and history editing before resume.
- Compaction trigger with both a window share and an absolute cap.
- On-demand tool discovery for many MCP tools.
- Edit errors with match counts, line numbers, hints and previews.
- ACP server/client.

## Limitations to avoid

- Autonomous approval by default.
- Reviewers and PreToolUse hooks that fail open on error or timeout.
- Read/write classification guessed by an LLM rather than declared.
- Failed subagents returning nothing.
- Absolute paths passed through file tools unconfined (callers not checked).
- No built-in OS sandbox for shell.

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v1.60.2 | research agent for the Oxgraft build session | adversary reviewer, fork/edit sessions, MCP-first; Autonomous default, fail-open hooks |
