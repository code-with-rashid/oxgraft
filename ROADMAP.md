# Roadmap

Milestones are ordered. Each item should be doable in one agent session.
Tick items in the same change that completes them. Target for M1–M4: 2026-12-07.

## M1: Rootstock (structure)

- [x] Cargo workspace with seams, strategies, core, protocol, CLI
- [x] Profiles and strategy assembly with clear errors
- [x] Learning system: watch list, harness profiles, grafts, capability matrix
- [x] AGENTS.md, skills, STATUS/ROADMAP handoff
- [x] CI (Rust on 3 OSes, repo checks, docs site) and weekly upstream watch
- [x] First green CI run
- [ ] Publish 0.0.1 of `oxgraft` to crates.io, npm and PyPI (maintainer)

## M2: First working agent

- [x] Spec-first: SPEC-0001..0004
- [x] Provider seam: Anthropic (native), OpenAI-compatible presets, mock
- [x] Tools: read, write, edit, ls, glob, grep, bash
- [x] Session loop in core emitting protocol events; retries; guards
- [x] `oxgraft run` headless with `--json`; interactive chat
- [x] Session persistence, resume, undo
- [ ] Smoke test against a real model (needs an API key)

## M3: Grafts

- [x] Edit formats: tolerant SEARCH/REPLACE (aider), apply_patch (codex)
- [x] Context compaction (pi/opencode/claude-code); git context; instruction discovery
- [x] OS sandbox (codex/claude-code); avoid graft "permissions are not a sandbox"
- [x] Skills, hooks, MCP, subagents, web_fetch, todo
- [x] Stability: interrupts, budgets, read-before-edit, session repair, Windows shell
- [x] Review 16 watched harnesses upstream at pinned versions
- [x] Deny-wins rules, compound-command checks, protected paths, mistake breaker, verify loop, JIT instructions
- [x] Streaming output (Anthropic SSE, OpenAI stream)

## Backlog from reviews (ranked; each needs a graft record first)

1. ~~Git-ref checkpoints per turn~~ (done: GRAFT-0046)
2. Sandbox network egress via a proxy with an allowlist, empty by default (claude-code, codex)
3. ~~Self-testing approval rules~~ (done: GRAFT-0043)
4. ~~Deterministic risk rails for shell~~ (done: GRAFT-0042)
5. ~~Auto-allow inert read-only shell commands~~ (trial: GRAFT-0047, profile `fewer-prompts`)
6. ~~Re-query on malformed tool calls~~ (done: GRAFT-0044)
7. ~~Review-on-submit~~ (change summary after each run: GRAFT-0048; a pre-finish checklist is still open)
8. ~~Goal mode~~ (`run --until <check>`: GRAFT-0054; a model-based verifier is still open)
9. ~~Fork a session~~ (`--fork ID@N`: GRAFT-0049; a browsable session tree is still open)
10. ~~Batched multi-edits matched against the original file~~ (done: GRAFT-0050)
11. ~~Repo map under a budget~~ (`map` tool: GRAFT-0053); LSP diagnostics after edits (opencode, claude-code) still open
12. Worktree-isolated writing subagents (claude-code); sandbox expansion on denial (gemini-cli)

## M4: Evidence

- [ ] agentic-arena adapter for oxgraft (headless JSONL)
- [ ] Evaluate GRAFT-0001, 0015, 0024 (edit formats) against baseline; record results
- [ ] Results page on the docs site generated from graft records
- [ ] README benchmark table: Oxgraft profiles vs donor harnesses on shared tasks

## Later

- TUI (ratatui), IDE extension, web console for session replays, image input,
  more providers (Bedrock, Vertex), Landlock-based sandbox without bubblewrap.
