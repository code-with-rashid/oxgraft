# Specs

Oxgraft is built spec-first. Every feature starts as a spec here; tests are
written from its acceptance criteria; code is written to pass them.

- File name: `SPEC-NNNN-<slug>.md`. Status: `draft` → `accepted` → `implemented`.
- Each acceptance criterion has an id (`AC1`, `AC2`, …). Tests that cover it
  mention the id in a comment (`// SPEC-0003 AC4`), so `grep` finds coverage.
- A spec that comes from another harness links its graft record; a graft that
  needs new behaviour links its spec.
- Changing accepted behaviour means editing the spec in the same change.

| Spec | Title | Status |
|---|---|---|
| [SPEC-0001](SPEC-0001-model-and-providers.md) | Conversation model and providers (any AI model) | implemented |
| [SPEC-0002](SPEC-0002-tools.md) | Built-in tools | implemented |
| [SPEC-0003](SPEC-0003-agent-loop.md) | Agent loop, approvals, stability guards | implemented |
| [SPEC-0004](SPEC-0004-cli-and-sessions.md) | CLI, headless mode, config, sessions | implemented |
| [SPEC-0005](SPEC-0005-extensions.md) | Extensions: skills, hooks, MCP | implemented |
| [SPEC-0006](SPEC-0006-safety-and-long-sessions.md) | Sandbox, interrupts, budgets, compaction | implemented |
