---
status: accepted
date: 2026-11-24
---

# ADR 0004: A versioned, append-only event protocol

## Context

agentic-arena, CI, a future TUI, IDE extension and web console all need to
observe sessions. Every serious harness ends up with a machine interface
(Codex `exec --json`, OpenCode's server, Pi's JSONL/RPC modes).

## Decision

`oxgraft-protocol` defines `Event`, serialised one per line as JSON tagged by
`type`. Every session begins with `session_started` and one `strategy_selected`
per seam, so each recorded run names its exact configuration. The schema is
append-only; breaking changes bump `PROTOCOL_VERSION`.

## Consequences

- The arena can benchmark Oxgraft like any other harness, by profile.
- UIs are clients of the stream, not of internals.
- Field names are effectively public API from the first release.
