---
id: SPEC-0005
title: Extensions - skills, hooks, MCP
status: implemented
grafts: [GRAFT-0017, GRAFT-0018, GRAFT-0019, GRAFT-0022, GRAFT-0043, GRAFT-0051, GRAFT-0062, GRAFT-0072, GRAFT-0079]
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
- **AC5** A `[[rules]]` entry may list `match` and `not_match` example
  resources. At load, every `match` example must match the rule's patterns and
  no `not_match` example may; otherwise the config is rejected with an error
  naming the rule and the example.
- **AC6** Hooks and `verify_command` run in the same shell as the `bash` tool
  (POSIX `sh`; on Windows Git Bash `sh` if found, else PowerShell), so a command
  that works for the model works in a hook.
- **AC7** An untrusted workspace cannot choose the profile: `profile` in its
  `.oxgraft/config.toml` is ignored with a warning, and its `profiles/*.toml`
  files are not used for name lookup (built-in profiles are), since a profile
  selects the approval policy. `--profile` and `OXGRAFT_PROFILE` still apply.
- **AC8** `user_prompt` hooks run before a task reaches the model (task in
  `OXGRAFT_PROMPT`): stdout is appended to the task as context, exit 2 stops the
  run with the hook's message. `stop` hooks run when the model wants to finish
  (reply in `OXGRAFT_FINAL_MESSAGE`): exit 2 sends stderr back to the model and
  the run continues, at most 3 times per run.
- **AC9** An MCP server may be given by `url` (Streamable HTTP) instead of
  `command`: messages are POSTed as JSON, replies may be JSON or SSE, the
  `Mcp-Session-Id` from the server is sent back on later requests, `headers`
  are added (with `${VAR}` from the environment). Setting both or neither of
  `command` and `url` is an error for that server.
- **AC10** `verify_command = "auto"` resolves, in trusted workspaces only, to
  the project's own quick check by marker file (Cargo.toml, go.mod, a
  package.json typecheck/check/lint script, Python project files); otherwise no
  verify command runs and a warning says why.
