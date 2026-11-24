---
id: codex
repo: openai/codex
license: Apache-2.0
policy: adapt
last_reviewed: "rust-v0.172.0"
status: reviewed
---

# OpenAI Codex CLI

[openai/codex](https://github.com/openai/codex) · Apache-2.0 · policy `adapt`

Reviewed at `rust-v0.172.0` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

> Docs note: at this tag `docs/*.md` are stubs linking to a docs site that now redirects to learn.chatgpt.com, so doc claims below are not pinned to the tag.

## By seam

| Seam | What it does | Strength or limitation? | Source |
|---|---|---|---|
| edit_format | `apply_patch` envelope (Add/Delete/Update File, optional Move to, `@@` context) with a streaming parser; context search goes exact → ignore trailing whitespace → ignore all surrounding whitespace. **Verified** | Strength: multi-file, tolerant. Limitation: last fallback ignores leading indentation (risky for Python/YAML) | [parser.rs](https://github.com/openai/codex/blob/rust-v0.172.0/codex-rs/apply-patch/src/parser.rs), [seek_sequence.rs](https://github.com/openai/codex/blob/rust-v0.172.0/codex-rs/apply-patch/src/seek_sequence.rs) |
| context | AGENTS.md on the first turn, `project_doc_max_bytes`, fallback filenames; compaction including remote compaction (files exist, **Verified**) | Strength | [config-advanced](https://learn.chatgpt.com/docs/config-file/config-advanced) |
| approval | `approval_policy` on-request / never (`untrusted` retired); optional `auto_review` reviewer agent; execpolicy Starlark `prefix_rule` with allow/prompt/forbidden, `justification`, and `match`/`not_match` examples validated at load (**Verified**, README) | Strength: self-testing rules | [execpolicy README](https://github.com/openai/codex/blob/rust-v0.172.0/codex-rs/execpolicy/README.md), [sandboxing](https://learn.chatgpt.com/docs/sandboxing) |
| sandbox | read-only / workspace-write / danger-full-access; Seatbelt (macOS), bubblewrap + seccomp (Linux), native Windows sandbox; optional transparent TCP egress via proxy | Strength: all three OSes, egress control | [linux-sandbox README](https://github.com/openai/codex/blob/rust-v0.172.0/codex-rs/linux-sandbox/README.md) |
| provider | `[model_providers.<id>]` with base_url, env_key, wire_api, bearer-token helper; `--oss` for Ollama/LM Studio; Bedrock | Strength | [config-advanced](https://learn.chatgpt.com/docs/config-file/config-advanced) |
| tools | shell, unified exec, apply_patch, web search, MCP, a "Code Mode" script host (module names, **Verified**) | Full list not documented in repo | [core/src](https://github.com/openai/codex/tree/rust-v0.172.0/codex-rs/core/src) |
| session | `history.jsonl` under CODEX_HOME, size cap, `codex exec resume` restores sandbox permissions and cwd | Strength | [release notes](https://github.com/openai/codex/releases/tag/rust-v0.172.0) |
| extension | hooks (project hooks only in trusted projects; admin-managed hooks), skills, MCP, plugins (**Verified**, docs/config.md) | Strength: trust gating | [config.md](https://github.com/openai/codex/blob/rust-v0.172.0/docs/config.md) |
| delegation | spawn/send_input/wait/close/resume multi-agent tools; `max_depth` ignored by V2 backend (**Verified**) | Mixed | [config_toml.rs](https://github.com/openai/codex/blob/rust-v0.172.0/codex-rs/config/src/config_toml.rs) |

## Strengths worth grafting

- Policy rules with `match`/`not_match` examples validated at load, and `forbidden` with a `justification` telling the model what to do instead.
- `host_executable` so `/tmp/evil/git` doesn't match a rule for `git`.
- Project-scoped config cannot set `model_provider`, `openai_base_url` or `notify`: a cloned repo can't redirect API traffic.
- Strict-first, then progressively looser patch context matching.
- Optional reviewer agent answering approval prompts at the sandbox boundary.

## Limitations to avoid

- Patch context differing only in leading indentation still applies (last `trim()` fallback).
- `max_depth` is silently ignored under the V2 multi-agent backend.
- Without bwrap/user namespaces the Linux sandbox only warns at startup; docs don't say whether commands then run unsandboxed.
- No "ask for everything except a known-safe list" mode since `untrusted` was retired.
- Docs cannot be pinned to a release.

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at rust-v0.172.0 | research agent for the Oxgraft build session | self-testing exec rules, trust-gated hooks, project config can't redirect providers; indentation-blind last fallback |
