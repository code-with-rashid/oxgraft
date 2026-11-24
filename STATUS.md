# Status

The handoff file. Every agent session reads this first and updates it last.
Keep it short: current truth, not history (history is in git and `learning/`).

_Last updated: 2026-11-25 by the autonomous build session (Claude Code)._

## Current milestone

**M3: grafts from real reviews**, with M2 (working agent) done. See `ROADMAP.md`.

## Works now (verified by tests; CI on Linux, macOS, Windows)

- **Any model**: Anthropic native Messages API (raw HTTP, prompt caching, refusal
  fallbacks, thinking blocks preserved), 10 OpenAI-compatible presets (OpenAI,
  OpenRouter, DeepSeek, Groq, Gemini, xAI, Mistral, Ollama, LM Studio, custom),
  and a scripted `mock:` provider. Retries with backoff and `retry-after`.
- **Tools**: read, write, edit (4 edit formats), ls, glob, grep, bash, todo,
  web_fetch, skill (Agent Skills), MCP servers over stdio, read-only `task` subagent.
- **Safety**: workspace path confinement, ordered approval rules, previews in the
  approval prompt, read-before-edit and stale-file guards, OS sandbox profile
  (bubblewrap/Seatbelt, fails closed), trusted-workspace gate for repo hooks and
  MCP servers, `oxgraft undo`.
- **Stability**: loop guard, max turns, token budget, max_tokens handling,
  context compaction, Ctrl-C interrupts, crash-safe sessions with repair on resume.
- **CLI**: `oxgraft` (chat with /help, /undo, /cost, /clear), `run` (`--json`,
  `--resume`), `sessions`, `undo`, `models`, `strategies`, `profile`; layered
  config; profiles default, baseline, ci, tolerant, codex-like, sandboxed.
- **Learning system**: 17 watched harnesses, 30 graft records, 34 logged
  iterations (`learning/iterations.md`), specs SPEC-0001..0006.
- Release workflow builds 5 targets on a `v*` tag (not yet exercised).

## Not verified

- **No real model has been called.** No API key or local model server was
  available during the build; providers are tested against a local fake HTTP
  server and the mock. First task for a human: run
  `oxgraft run "list the crates and summarise each"` with a real key.
- The release workflow has never run (no tag pushed yet).
- Comparative evaluations of grafts (agentic-arena) have not been run; every graft
  is `trial`, not `adopted`.
- Harness profiles in `learning/harnesses/` are still stubs/seeds; learnings so far
  came from agentic-arena's pinned reviews and documented behaviour, not from a
  fresh read of each upstream at a pinned version.

## Next up

1. Smoke-test with a real Anthropic key and with a local Ollama model; fix what breaks.
2. Review codex and opencode upstream with the `watch-harnesses` skill; set `last_reviewed`.
3. agentic-arena adapter for `oxgraft run --json` (ROADMAP M4), then evaluate GRAFT-0001/0015/0024.

## Known problems

- On Windows without Git for Windows, `bash` falls back to PowerShell; models may
  still write POSIX syntax there.
- `web_fetch`'s HTML-to-text conversion is basic (no readability extraction).
- No streaming output yet; long responses appear when complete.

## Dev notes

- On the maintainer's Windows machine, Rust lives in WSL Ubuntu:
  `wsl -d Ubuntu -- bash -lc "cd /mnt/c/Users/rashi/dev/opensource/oxgraft && CARGO_TARGET_DIR=~/oxgraft-target cargo test --workspace"`.
