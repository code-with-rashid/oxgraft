# Iteration log

One row per learning iteration: a harness (or several) studied, what Oxgraft
took or avoided, the spec and tests that pin it down, and the commit. Counted
honestly: an iteration is only logged when its tests pass.

| # | Date | Donor(s) | Learning | Kind | Graft | Spec | Verified by |
|---|---|---|---|---|---|---|---|
| 1 | 2026-11-24 | aider | SEARCH/REPLACE edit format | adopt | GRAFT-0001 | — | strategy unit tests |
| 2 | 2026-11-24 | opencode | Ordered allow/ask/deny permission rules | adopt | GRAFT-0002 | — | strategy unit tests |
| 3 | 2026-11-25 | opencode, goose, aider | Provider-neutral model; Anthropic native + 10 OpenAI-compatible presets + mock | adopt | GRAFT-0009 | SPEC-0001 | provider mapping tests |
| 4 | 2026-11-25 | opencode, pi | File tools confined to the workspace (permissions ≠ sandbox) | avoid | GRAFT-0003 | SPEC-0002 | `refuses_paths_outside_workspace` |
| 5 | 2026-11-25 | swe-agent | Bounded observations: head+tail truncation, capped search results | adopt | GRAFT-0004 | SPEC-0002 | truncation tests |
| 6 | 2026-11-25 | claude-code, opencode | Numbered, paged `read` | adopt | GRAFT-0012 | SPEC-0002 | `read_numbers_lines_and_pages` |
| 7 | 2026-11-25 | codex | Non-interactive shell env, hard timeouts, no pipe hangs | adopt | GRAFT-0008 | SPEC-0002 | bash timeout tests |
| 8 | 2026-11-25 | cline, aider | Per-session undo without git | adopt | GRAFT-0005 | SPEC-0004 | undo unit + e2e tests |
| 9 | 2026-11-25 | codex, claude-code, gemini-cli | AGENTS.md / CLAUDE.md / GEMINI.md discovery with `@` imports | adopt | GRAFT-0006 | SPEC-0003 | prompt tests |
| 10 | 2026-11-25 | mini-swe-agent | Loop guard on identical repeated calls | avoid | GRAFT-0007 | SPEC-0003 | `blocks_repeated_identical_calls` |
| 11 | 2026-11-25 | aider | Never run tool calls truncated by max_tokens | avoid | GRAFT-0011 | SPEC-0003 | `handles_max_tokens` |
| 12 | 2026-11-25 | codex, claude-code, pi | Headless JSONL events + exit codes | adopt | GRAFT-0010 | SPEC-0004 | CLI e2e tests |
| 13 | 2026-11-25 | (stability) | Provider HTTP paths tested against a local fake server: headers, 429 + retry-after, 401, x-should-retry, refused connections, bad JSON | — | — | SPEC-0001 | `http_fake.rs` |
| 14 | 2026-11-25 | claude-code | Anthropic automatic prompt caching on the append-only loop | adopt | GRAFT-0009 | SPEC-0001 | `anthropic_round_trip_over_http` |
| 15 | 2026-11-25 | claude-code, codex | Git branch + status in the system prompt | adopt | GRAFT-0013 | SPEC-0003 | agent tests |
| 16 | 2026-11-25 | opencode, claude-code | Edit tool accepts `old_string`/`new_string` | avoid | GRAFT-0014 | SPEC-0002 | `edit_accepts_old_new_string_inputs` |
| 17 | 2026-11-25 | aider, cline | Whitespace-tolerant SEARCH/REPLACE strategy + `tolerant` profile | adopt | GRAFT-0015 | SPEC-0002 | tolerant strategy tests |
| 18 | 2026-11-25 | claude-code, codex | `todo` plan tool | adopt | GRAFT-0016 | SPEC-0002 | `todo_renders_plan` |
| 19 | 2026-11-25 | claude-code, codex, gemini-cli | Agent Skills discovery + `skill` tool | adopt | GRAFT-0017 | SPEC-0002 | `discovers_and_loads_workspace_skills` |
| 20 | 2026-11-25 | claude-code | Tool hooks (pre/post/session_end, exit 2 = block/feedback) | adopt | GRAFT-0018 | SPEC-0003 | `hooks_block_and_give_feedback` |
| 21 | 2026-11-25 | goose | Repository hooks run only in trusted workspaces | avoid | GRAFT-0019 | SPEC-0004 | `workspace_config_and_flags_layer` |
| 22 | 2026-11-25 | cline, claude-code, codex | Ctrl-C interrupts the run and running commands; second Ctrl-C exits | adopt | GRAFT-0020 | SPEC-0003 | `cancellation_stops_before_the_next_turn` |
| 23 | 2026-11-25 | cline | Resume repairs dangling tool calls from interrupted runs | adopt | GRAFT-0020 | SPEC-0004 | `resume_repairs_dangling_tool_calls` |
| 24 | 2026-11-25 | openhands | Per-run token budget | avoid | GRAFT-0021 | SPEC-0003 | `token_budget_stops_runaway_sessions` |
| 25 | 2026-11-25 | goose, claude-code, codex | MCP client over stdio with timeouts; tools as `mcp__server__tool` | adopt | GRAFT-0022 | SPEC-0002 | `mcp_tools_work_end_to_end` |
| 26 | 2026-11-25 | claude-code, goose | `.mcp.json` / workspace MCP servers only in trusted workspaces | avoid | GRAFT-0019 | SPEC-0004 | `mcp_tools_work_end_to_end` |
