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
