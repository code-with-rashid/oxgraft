# Status

The handoff file. Every agent session reads this first and updates it last.
Keep it short: current truth, not history (history is in git and `learning/`).

_Last updated: 2026-11-25 by the second autonomous session (Claude Code), iterations 53–76._

## Current milestone

**M3 (grafts) done; M4 (evidence) not started.** See `ROADMAP.md`.

## Works now (verified by tests; CI on Linux, macOS, Windows)

- **Any model**: Anthropic native Messages API (raw HTTP, prompt caching, refusal
  fallbacks, thinking blocks preserved), 10 OpenAI-compatible presets, `mock:`.
  **Streaming** (SSE) for both, with `text_delta` events; retries with backoff
  and `retry-after`. `auto` profile picks the edit format by model family.
- **Tools**: read, write, edit (4 formats, old/new_string, batched `edits`), ls,
  glob, grep, `map` (ranked repo outline), bash (POSIX sh, Git Bash or PowerShell
  on Windows), todo, web_fetch, skill, MCP (stdio), read-only `task` subagent.
- **Safety**: workspace path confinement (symlinks included); deny-wins rules with
  per-part shell checks, justifications and self-testing `match`/`not_match`
  examples; **risk rails** (pipe-to-shell, `rm -rf`, raw disk, mkfs, sudo always
  ask, even after "always"); protected paths; approval modes
  (`--mode read-only|suggest|auto-edit|full-auto`); `inert_reads` strategy
  (`fewer-prompts` profile); OS sandbox profile (fails closed); fail-closed
  pre_tool hooks; trusted-workspace gate for hooks, MCP, rules, model, profile,
  approval mode, verify_command and the repo's own `profiles/`; `@imports`
  confined to the workspace; web/MCP output fenced as `<untrusted>`.
- **Recovery**: per-turn **checkpoints** in a private repo under the user data dir (captures
  shell changes; never touches the user's `.git`), `oxgraft checkpoints`,
  `oxgraft restore N` (undoable), change summary after each run, `oxgraft undo`.
- **Stability**: loop guard, mistake breaker, malformed tool calls re-asked out of
  history, max turns, token budget, per-turn call cap, size caps, max_tokens
  handling, compaction, Ctrl-C, crash-safe sessions with repair, fuzz-tested
  strategies, verify_command loop, `run --until <check>`.
- **CLI**: chat, run (`--json`, `--resume`, `--fork ID@N`, `--until`), init,
  doctor, sessions, checkpoints, restore, undo, models, strategies, profile.
- **Learning**: 16 of 17 watched harnesses reviewed at pinned tags (Roo Code is
  archived); 55 graft records; 76 logged iterations; specs SPEC-0001..0006;
  ADRs 0001–0007.

## Not verified

- **An independent review of iterations 53–72 found 9 problems; all are fixed**
  (iterations 73–76), including two security holes: a checkpoint repo planted
  by a cloned repository could run code, and the risk rails missed commands
  that weren't first in their part (`xargs rm -rf`, `if …; then rm -rf ~`).
- CI is green on Linux, macOS and Windows for iterations 53–76 (run 45551308549).
  Its first run caught a path-spelling bug in the checkpoint "never inside the
  workspace" check (macOS `/private/var`, Windows short names), now fixed.
- **No real model has been called.** Providers are tested against a local fake
  HTTP server (including SSE streams) and the mock. First human task:
  `oxgraft doctor`, then `oxgraft run "summarise this repo"` with a real key and
  with Ollama. Streaming against real servers is the riskiest untested path;
  `OXGRAFT_STREAM=0` turns it off.
- OpenAI-compatible streaming sends `stream_options.include_usage` only for
  openai, openrouter and deepseek; others may report zero tokens when streaming.
- Release workflow never run (no tag pushed). Packages not published.
- No comparative evaluation: every graft is `trial`. The `auto` profile is a trial default.
- macOS Seatbelt profile is exercised only by CI's test, not by a human.

## Next up

1. Real-model smoke tests (Anthropic, OpenAI-compatible, Ollama), streaming on and off.
2. agentic-arena adapter for `oxgraft run --json`, then evaluate edit formats
   (GRAFT-0001/0015/0024/0038) and `fewer-prompts` vs default.
3. Remaining backlog (ROADMAP): egress proxy allowlist, LSP diagnostics,
   worktree-isolated writing subagents, sandbox expansion on denial.

## Known problems

- Checkpoints restore removes files that became tracked after `.gitignore`
  changed; the pre-restore checkpoint keeps them recoverable.

## Dev notes

- On the maintainer's Windows machine Rust lives in WSL Ubuntu:
  `wsl -d Ubuntu -- bash -lc "cd /mnt/c/Users/rashi/dev/opensource/oxgraft && CARGO_TARGET_DIR=~/oxgraft-target cargo test --workspace"`.
- Prefer the Edit tool over shell heredocs for Rust code containing backslashes.
- Python on this machine defaults to cp1252: run scripts with `PYTHONUTF8=1` and
  open files with `encoding="utf-8"`, or non-ASCII characters get corrupted.
