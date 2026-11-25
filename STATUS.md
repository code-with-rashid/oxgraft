# Status

The handoff file. Every agent session reads this first and updates it last.
Keep it short: current truth, not history (history is in git and `learning/`).

_Last updated: 2026-11-25 by the autonomous build session (Claude Code)._

## Current milestone

**M3 (grafts) mostly done; M4 (evidence) not started.** See `ROADMAP.md`.

## Works now (verified by tests; CI on Linux, macOS, Windows)

- **Any model**: Anthropic native Messages API (raw HTTP, prompt caching, refusal
  fallbacks, thinking blocks preserved), 10 OpenAI-compatible presets, `mock:`.
  Retries with backoff and `retry-after`. `auto` profile picks the edit format by
  model family.
- **Tools**: read, write, edit (4 formats + old/new_string), ls, glob, grep, bash
  (POSIX sh, Git Bash or PowerShell on Windows), todo, web_fetch, skill, MCP
  (stdio), read-only `task` subagent.
- **Safety**: workspace path confinement; deny-wins approval rules with per-part
  shell command checks and justifications; `[[rules]]` config; protected paths;
  approval previews; read-before-edit and stale-file guards; OS sandbox profile
  (bubblewrap/Seatbelt, fails closed); fail-closed pre_tool hooks; trusted-workspace
  gate for repo hooks, MCP servers, rules, model and verify_command; `oxgraft undo`.
- **Stability**: loop guard, mistake breaker, max turns, token budget, per-turn call
  cap, size caps, max_tokens handling, compaction, Ctrl-C interrupts, crash-safe
  sessions with repair, fuzz-tested strategies, verify_command feedback loop.
- **CLI**: chat (/help /undo /cost /clear), run (--json, --resume), init, doctor,
  sessions, undo, models, strategies, profile.
- **Learning**: 16 of 17 watched harnesses reviewed at pinned tags (Roo Code is
  archived); 41 graft records; 52 logged iterations; specs SPEC-0001..0006.

## Not verified

- **No real model has been called.** No API key or local model server was available.
  Providers are tested against a local fake HTTP server and the mock. First human
  task: `oxgraft doctor`, then `oxgraft run "summarise this repo"` with a real key
  and with Ollama.
- Release workflow never run (no tag pushed). Packages not published.
- No comparative evaluation: every graft is `trial`. The `auto` profile is a trial default.
- macOS Seatbelt profile is exercised only by CI's test, not by a human.

## Next up

1. Real-model smoke tests (Anthropic, OpenAI-compatible, Ollama); fix what breaks.
2. agentic-arena adapter for `oxgraft run --json`, then evaluate edit formats (GRAFT-0001/0015/0024/0038).
3. Top backlog items from reviews (ROADMAP "Backlog from reviews").

## Known problems

- No streaming output yet; long responses appear when complete.
- `web_fetch` HTML-to-text is basic.

## Dev notes

- On the maintainer's Windows machine Rust lives in WSL Ubuntu:
  `wsl -d Ubuntu -- bash -lc "cd /mnt/c/Users/rashi/dev/opensource/oxgraft && CARGO_TARGET_DIR=~/oxgraft-target cargo test --workspace"`.
- Prefer the Edit tool over shell heredocs for Rust code containing backslashes.
