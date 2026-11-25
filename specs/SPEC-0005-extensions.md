---
id: SPEC-0005
title: Extensions - skills, hooks, MCP
status: implemented
grafts: [GRAFT-0017, GRAFT-0018, GRAFT-0019, GRAFT-0022, GRAFT-0043]
---

# SPEC-0005: Extensions

## Goal

Reuse the ecosystems users already have (Agent Skills, MCP servers, hook
scripts) without letting a cloned repository run code unasked.

## Acceptance criteria

- **AC1** Skills in `.agents/skills` and `.claude/skills` (workspace, then home)
  are listed by name and description in the system prompt; the `skill` tool
  returns a skill's body; the first definition of a name wins.
- **AC2** Hooks run for `pre_tool`, `post_tool` and `session_end`, filtered by
  tool name, with a timeout. Exit 2 blocks (pre) or returns feedback (post);
  other failures are warnings only.
- **AC3** MCP servers from the user config start over stdio; their tools appear
  as `mcp__<server>__<tool>`; tool errors (`isError`) become tool errors; every
  request times out; a server that fails to start is a warning, not a fatal error.
- **AC4** Hooks and MCP servers defined by the workspace (`.oxgraft/config.toml`,
  `.mcp.json`) load only when the workspace is listed in the user config's
  `trusted_workspaces`; otherwise a warning explains how to enable them.
- **AC6** A `[[rules]]` entry may list `match` and `not_match` example
  resources. At load, every `match` example must match the rule's patterns and
  no `not_match` example may; otherwise the config is rejected with an error
  naming the rule and the example.
