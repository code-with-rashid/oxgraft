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
| 48 | 2026-11-25 | opencode | `auto` profile: edit format chosen by model-family table (trial default) | adopt | GRAFT-0038 | SPEC-0004 | `auto_picks_profile_by_model_family` |
| 49 | 2026-11-25 | aider | `verify_command` after editing turns; failures fed back, capped at 3 rounds; trusted-only in workspaces | adopt | GRAFT-0039 | SPEC-0005 | `verify_command_feeds_failures_back_after_edits` |
| 50 | 2026-11-25 | openhands, swe-agent, mini-swe-agent, pi, cline, kilocode, qwen-code, continue, deepseek-harness, crush | Second review batch: 16 of 17 watched harnesses reviewed at pinned tags (roo-code archived); Crush license corrected to FSL-1.1-MIT, clean-room | review | — | — | `check-learning`, `pnpm watch` |
| 51 | 2026-11-25 | qwen-code | Protected self-modification paths always ask, whatever the rules | adopt | GRAFT-0040 | SPEC-0006 | `protected_paths_always_ask_and_mistakes_trip_the_breaker` |
| 52 | 2026-11-25 | cline, openhands | Consecutive-mistake breaker: nudge at 6, stop at 12 | adopt | GRAFT-0041 | SPEC-0006 | `protected_paths_always_ask_and_mistakes_trip_the_breaker` |
| 53 | 2026-11-25 | openhands | Shell risk rails: fetch-to-exec, forced recursive delete, raw disk, mkfs, sudo always asked, with reason; `needs_fresh_yes` seam method | adopt | GRAFT-0042 | SPEC-0006 | `risky_commands_ignore_always_answers` |
| 54 | 2026-11-25 | codex | `match`/`not_match` examples on approval rules, validated at load (config error on failure); rules documented in usage.md | adopt | GRAFT-0043 | SPEC-0005 | `rule_examples_are_checked_at_load` |
| 55 | 2026-11-25 | swe-agent | Malformed tool calls (unknown tool, bad JSON, missing field) re-asked up to 2× outside the history; thinking-safe | adopt | GRAFT-0044 | SPEC-0003 | `malformed_tool_calls_are_requeried_out_of_history` |
| 56 | 2026-11-25 | opencode, codex, claude-code | Streaming output: `Provider::stream` seam method (ADR 0007), SSE for Anthropic and OpenAI-compatible, `text_delta` events, live terminal rendering | adopt | GRAFT-0045 | SPEC-0001, SPEC-0003 | `anthropic_streams_over_http` |
| 57 | 2026-11-25 | cline, kilocode | Per-turn checkpoints in a private git repo (captures shell changes); `oxgraft checkpoints` / `restore N`, undoable | adopt | GRAFT-0046 | SPEC-0004 | `checkpoints_capture_shell_changes_and_restore` |
| 58 | 2026-11-25 | crush (clean-room) | `inert_reads` approval strategy + `fewer-prompts` profile: single read-only shell commands run without asking | adopt | GRAFT-0047 | SPEC-0002 | `inert_commands_are_allowed_but_rules_still_win` |
| 59 | 2026-11-25 | swe-agent, cline | Change summary after each run/chat turn (diff stat from checkpoints + one-command undo) | adopt | GRAFT-0048 | SPEC-0004 | `human_runs_end_with_a_change_summary` |
| 60 | 2026-11-25 | pi, goose | `--fork ID@N` for run and chat: new session from a copy of another's history at a finished reply | adopt | GRAFT-0049 | SPEC-0004 | `sessions_can_be_forked` |
| 61 | 2026-11-25 | — | Hooks and verify_command run in the bash tool's shell (Git Bash or PowerShell on Windows, not cmd) | fix | — | SPEC-0005 | `hooks::tests` |
| 62 | 2026-11-25 | pi | `edit` takes `edits: [{old_string,new_string}]`, matched against the original, all-or-nothing | adopt | GRAFT-0050 | SPEC-0002 | `batched_edits_apply_together_or_not_at_all` |
| 63 | 2026-11-25 | codex | Security fix from self-review: untrusted workspaces can no longer pick the profile (config `profile` or own `profiles/*.toml`), which could have switched approvals to allow_all | adopt | GRAFT-0051 | SPEC-0005 | `untrusted_workspaces_cannot_choose_the_profile` |
| 64 | 2026-11-25 | — | Security fix from self-review: `@import` in AGENTS.md/CLAUDE.md could read absolute paths or symlinks outside the workspace into the prompt; now confined | fix | — | SPEC-0003 | `imports_stay_inside_the_workspace` |
| 65 | 2026-11-25 | codex, claude-code | `--mode` read-only / suggest / auto-edit / full-auto; rails and protected paths still ask; workspace can't set it | adopt | GRAFT-0052 | SPEC-0004 | `approval_modes_change_what_runs_without_asking` |
| 66 | 2026-11-25 | aider | `map` tool: definitions per file ranked by cross-file use, under a size budget | adopt | GRAFT-0053 | SPEC-0002 | `maps_definitions_ranked_by_use` |
| 67 | 2026-11-25 | qwen-code | `run --until <check>`: done only when the check passes; failures fed back up to 3 rounds | adopt | GRAFT-0054 | SPEC-0003 | `finish_check_sends_failures_back_until_it_passes` |
| 68 | 2026-11-25 | goose, claude-code | web_fetch and MCP output fenced as `<untrusted>` with injection-phrase warnings; system prompt rule | adopt | GRAFT-0055 | SPEC-0002 | `untrusted_content_is_fenced_and_flagged` |
| 69 | 2026-11-25 | claude-code | `doctor` reports trust, approval mode, checkpoints (side-effect free), streaming | adopt | GRAFT-0031 | SPEC-0004 | `doctor_reports_problems_and_readiness` |
| 70 | 2026-11-25 | — | web_fetch HTML to Markdown-ish text (headings, lists, links, pre, tables, entities), fuzz-tested; fixed a possible UTF-8 slicing panic | fix | — | SPEC-0002 | `converts_html_to_markdownish_text` |
| 71 | 2026-11-25 | cline | Chat `/checkpoints` and `/restore N`; checkpoint listing/restore shared with the CLI commands | adopt | GRAFT-0046 | SPEC-0004 | `chat_lists_and_restores_checkpoints` |
| 72 | 2026-11-25 | gemini-cli | Sandboxed command failures caused by the sandbox (write denied, no network) get a do-not-retry note | adopt | — | SPEC-0006 | `sandbox_failures_get_a_hint` |
| 73 | 2026-11-25 | — | Security fix from independent review: checkpoint repo moved out of the workspace (a cloned repo could plant one with hooks/fsmonitor and run code at session start); git runs with only Oxgraft config, hooks off; verbatim Windows paths simplified for git | fix | GRAFT-0046 | SPEC-0004 | `checkpoint_repo_is_never_inside_the_workspace` |
| 74 | 2026-11-25 | — | Fixes from review: Anthropic requests wrap unparsed tool input as an object (a cut-off tool call no longer breaks the session); OpenAI streams accept `"error": null` and tell index-less calls apart by id | fix | — | SPEC-0001 | `non_object_tool_input_is_sent_wrapped` |
| 75 | 2026-11-25 | — | Fixes from review: risk rails scan every word (xargs/if/braces/single &, quoting tricks, `bash <(curl)`, eval/source, Windows rd/del/ri); inert_reads drops git, uniq, date and checks attached option paths | fix | GRAFT-0042, GRAFT-0047 | SPEC-0006, SPEC-0002 | `rails_flag_dangerous_commands` |
| 76 | 2026-11-25 | — | Fixes from review: batched edits count overlapping repeats; `<untrusted>` fence defused case-insensitively; streams use a 180 s idle timeout instead of a 600 s total | fix | GRAFT-0050, GRAFT-0055, GRAFT-0045 | SPEC-0002, SPEC-0001 | `batched_edits_apply_together_or_not_at_all` |
| 77 | 2026-11-25 | claude-code | `doctor --live`: real round trip (streamed reply, latency, tokens) and a forced tool call to check the model can use tools | adopt | GRAFT-0031 | SPEC-0004 | `doctor_live_checks_reply_and_tool_calling` |
| 78 | 2026-11-25 | claude-code, gemini-cli | `@path` mentions in a task attach workspace files (and count as read) | adopt | GRAFT-0056 | SPEC-0003 | `mentioned_files_are_attached_and_editable` |
| 79 | 2026-11-25 | claude-code, gemini-cli | Custom commands: `.agents/commands/<name>.md` (or `.claude/commands`) as `/name` in chat and `run`, with `$ARGUMENTS` | adopt | GRAFT-0057 | SPEC-0004 | `custom_commands_expand_in_run` |
| 80 | 2026-11-25 | gemini-cli, aider | Chat `!command`: run a command yourself; output goes to the model with the next message | adopt | GRAFT-0058 | SPEC-0004 | `chat_bang_runs_a_command_and_passes_its_output_on` |
| 81 | 2026-11-25 | opencode, claude-code | One reminder per run when the model finishes with todo items not done | adopt | GRAFT-0059 | SPEC-0003 | `unfinished_todo_items_get_one_reminder` |
| 82 | 2026-11-25 | opencode | Truncated bash output saved in full to .oxgraft/outputs/ with the path in the result | adopt | GRAFT-0060 | SPEC-0002 | `long_output_is_saved_in_full` |
| 83 | 2026-11-25 | opencode, claude-code | `grep` `context` option (like -C), merged groups | adopt | GRAFT-0061 | SPEC-0002 | `grep_shows_context_lines` |
| 84 | 2026-11-25 | claude-code | `user_prompt` hooks (add context or block) and `stop` hooks (send the model back, max 3) | adopt | GRAFT-0062 | SPEC-0005 | `user_prompt_and_stop_hooks` |
| 85 | 2026-11-25 | swe-agent | `files_changed` event before `session_finished` (from checkpoints); JSON event reference in usage docs | adopt | GRAFT-0063 | SPEC-0004 | `json_runs_report_changed_files` |
| 86 | 2026-11-25 | aider | Local providers: 32k compaction window by default; doctor warns about Ollama context truncation and tool support | avoid | GRAFT-0064 | SPEC-0001 | `doctor_warns_about_ollama_context` |
| 87 | 2026-11-25 | aider, claude-code | Model aliases opus / sonnet / haiku / fable | adopt | GRAFT-0065 | SPEC-0001 | `parses_model_strings` |
| 88 | 2026-11-25 | opencode | `oxgraft export [ID] [-o file]`: session as a Markdown transcript | adopt | GRAFT-0066 | SPEC-0004 | `sessions_export_as_markdown` |
