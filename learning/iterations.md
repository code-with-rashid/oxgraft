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
| 27 | 2026-11-25 | pi, opencode, claude-code | Append-only context compaction via handover summary | adopt | GRAFT-0023 | SPEC-0003 | `compacts_context_and_continues` |
| 28 | 2026-11-25 | codex | `apply_patch` edit format + `codex-like` profile | adopt | GRAFT-0024 | SPEC-0002 | apply_patch strategy tests |
| 29 | 2026-11-25 | codex, claude-code (avoiding opencode/pi gap) | OS sandbox for bash: bubblewrap / Seatbelt, workspace-write, offline, fails closed; `sandboxed` profile | adopt | GRAFT-0025 | SPEC-0002 | `confines_writes_to_the_workspace_when_available` |
| 30 | 2026-11-25 | opencode, claude-code | Read-only `task` subagent with a fresh context; hidden from `last` | adopt | GRAFT-0026 | SPEC-0005 | `subagent_explores_read_only_and_reports` |
| 31 | 2026-11-25 | gemini-cli, opencode, claude-code | `web_fetch` tool (approval required, text only) | adopt | GRAFT-0027 | SPEC-0002 | `fetches_over_http_and_rejects_other_schemes` |
| 32 | 2026-11-25 | claude-code, opencode | Read-before-edit and stale-file guards | adopt | GRAFT-0028 | SPEC-0003 | `edits_require_a_fresh_read` |
| 33 | 2026-11-25 | cline, claude-code | Approval prompt previews edits and writes | adopt | GRAFT-0029 | SPEC-0004 | approval tests (manual preview) |
| 34 | 2026-11-25 | claude-code, codex | Windows: Git Bash `sh` first, PowerShell fallback, never cmd.exe | avoid | GRAFT-0030 | SPEC-0002 | bash tests on Windows CI |
| 35 | 2026-11-25 | claude-code, codex | Chat slash commands: /help, /cost, /clear, unknown-command hints | adopt | — | SPEC-0004 | manual smoke test |
| 36 | 2026-11-25 | (stability) | Size caps: read 50 MB, edit 10 MB, grep skips files over 5 MB (no OOM on logs/dumps) | avoid | — | SPEC-0002 | `huge_files_are_refused_not_loaded` |
| 37 | 2026-11-25 | (stability) | Fuzz test: 12 000 malformed edits across all edit formats, plus approval/context/sandbox seams, never panic | — | — | SPEC-0002 | `edit_formats_never_panic` |
| 38 | 2026-11-25 | (stability) | At most 32 tool calls run per turn; the rest get an error result (history stays valid) | avoid | — | SPEC-0003 | `caps_tool_calls_per_turn` |
| 39 | 2026-11-25 | claude-code | `oxgraft doctor`: config, model/credentials, profile, live sandbox, MCP, shell, git, instructions, skills | adopt | GRAFT-0031 | SPEC-0004 | `doctor_reports_problems_and_readiness` |
| 40 | 2026-11-25 | codex, claude-code | `oxgraft init` writes a verified AGENTS.md through the normal loop | adopt | GRAFT-0032 | SPEC-0004 | `init_writes_agents_md_through_the_loop` |
| 41 | 2026-11-25 | codex, claude-code, opencode, gemini-cli, aider, goose | First upstream reviews at pinned tags; profiles and `last_reviewed` set for six harnesses | review | — | — | `check-learning` |
| 42 | 2026-11-25 | goose | pre_tool hooks fail closed (found the same flaw in Oxgraft) | avoid | GRAFT-0033 | SPEC-0005 | `failing_pre_tool_hooks_fail_closed` |
| 43 | 2026-11-25 | codex | Untrusted workspaces cannot choose the model/provider | adopt | GRAFT-0034 | SPEC-0004 | `workspace_config_and_profiles` |
| 44 | 2026-11-25 | aider, goose | Failed edits return the nearest real lines with numbers | adopt | GRAFT-0035 | SPEC-0002 | `failed_edits_show_the_nearest_real_lines` |
| 45 | 2026-11-25 | claude-code (avoiding opencode last-match) | Deny always wins; compound bash commands checked per part; `$(` asks | adopt | GRAFT-0036 | SPEC-0003 | `deny_always_wins_and_compound_commands_are_checked_per_part` |
| 46 | 2026-11-25 | codex | Configurable `[[rules]]` with justifications shown to the model | adopt | GRAFT-0036 | SPEC-0004 | `configured_rules_apply_with_reasons` |
| 47 | 2026-11-25 | gemini-cli | Just-in-time subdirectory AGENTS.md/CLAUDE.md/GEMINI.md, once per run, in the tool result | adopt | GRAFT-0037 | SPEC-0003 | `nested_instructions_are_shown_once_when_touched` |
