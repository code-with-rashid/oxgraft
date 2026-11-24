---
status: accepted
date: 2026-11-24
---

# ADR 0001: Write the harness in Rust

## Context

Leading harnesses are written in TypeScript (Claude Code, OpenCode, Gemini
CLI, Cline, Pi), Rust (Codex CLI, goose), Go (Crush) and Python (Aider,
OpenHands, SWE-agent). Oxgraft learns from all of them regardless of language.

## Decision

The harness core and CLI are Rust (stable, 2024 edition). Supporting tooling
(repo checks, watcher, docs site) is Node/TypeScript; evaluation stays in
agentic-arena (Python) and talks to Oxgraft only through the event protocol.

## Consequences

- Single static binary, fast start-up, strong process and sandbox control.
- Codex and goose (Apache-2.0, Rust) are directly adaptable donors; others
  are design references, re-implemented in Rust.
- Slower iteration than TypeScript; mitigated by small crates and strict seams.
- Windows developers need WSL or the MSVC build tools; CI covers all three OSes.
