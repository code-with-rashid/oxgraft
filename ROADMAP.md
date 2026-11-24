# Roadmap

Milestones are ordered. Each item should be doable in one agent session.
Tick items in the same change that completes them. Target for M1–M4: 2026-12-07.

## M1: Rootstock (structure)

- [x] Cargo workspace with seams, strategies, core, protocol, CLI
- [x] Profiles and strategy assembly with clear errors
- [x] Learning system: watch list, harness profiles, grafts, capability matrix
- [x] AGENTS.md, skills, STATUS/ROADMAP handoff
- [x] CI (Rust on 3 OSes, repo checks, docs site) and weekly upstream watch
- [ ] First green CI run
- [ ] Publish 0.0.1 of `oxgraft` to crates.io, npm and PyPI (maintainer)

## M2: First working agent

- [ ] ADR: provider seam (async, streaming, tool calls)
- [ ] Anthropic provider (Messages API, streaming, tool use)
- [ ] Tools: read, write/edit (via EditFormat), grep, glob, bash (via ApprovalPolicy + Sandbox)
- [ ] Session loop in core emitting protocol events
- [ ] `oxgraft run "<task>"` headless with `--json`; minimal interactive mode
- [ ] Session persistence to JSONL (resume)

## M3: First grafts from real reviews

- [ ] Review codex, opencode, aider, claude-code, pi (`watch-harnesses` skill)
- [ ] Graft: Codex-style `apply_patch` edit format
- [ ] Graft: repo map context (aider) or compaction (pi/opencode)
- [ ] Graft: Linux Landlock sandbox (codex), `avoid` graft for "permissions are not a sandbox"
- [ ] Skills loading (SKILL.md) and AGENTS.md discovery

## M4: Evidence

- [ ] agentic-arena adapter for oxgraft (headless JSONL)
- [ ] Evaluate GRAFT-0001 and GRAFT-0002 against baseline; record results
- [ ] Results page on the docs site generated from graft records
- [ ] README benchmark table: Oxgraft profiles vs donor harnesses on shared tasks

## Later

- TUI (ratatui), MCP client (rmcp), subagents, hooks, IDE extension,
  more providers, web console for session replays.
