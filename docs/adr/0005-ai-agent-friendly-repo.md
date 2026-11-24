---
status: accepted
date: 2026-11-24
---

# ADR 0005: Built for iteration by any AI harness

## Context

Most work on Oxgraft will be done by AI coding agents, often different tools
in successive sessions, with no shared memory.

## Decision

- `AGENTS.md` is the single source of instructions. `CLAUDE.md`, `GEMINI.md`
  and `.github/copilot-instructions.md` only point to it.
- `STATUS.md` is the handoff: read first, updated last, every session.
- `ROADMAP.md` lists milestone items sized for one session each.
- Recurring procedures are Agent Skills in `.agents/skills/` (read by Codex,
  Cursor, Gemini CLI, Copilot, Antigravity), mirrored to `.claude/skills/`
  for Claude Code by `pnpm skills:sync` and checked in CI.
- One command, `pnpm check`, runs everything CI runs.
- A devcontainer gives cloud agents a ready toolchain.

## Consequences

- Any harness can pick up the next task from files alone.
- Instructions must be kept current; `session-handoff` makes that part of done.
