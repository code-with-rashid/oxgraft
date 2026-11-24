---
id: SPEC-0004
title: CLI, headless mode, config, sessions
status: implemented
grafts: [GRAFT-0005]
---

# SPEC-0004: CLI, headless mode, config, sessions

## Commands

| Command | Purpose |
|---|---|
| `oxgraft` / `oxgraft chat` | Interactive session (prompts for approvals) |
| `oxgraft run "<task>"` | One task, then exit; `--json` streams protocol events |
| `oxgraft run --resume <id> "<task>"` | Continue a saved session |
| `oxgraft sessions` | List saved sessions |
| `oxgraft undo [<id>]` | Restore files changed by a session |
| `oxgraft models` | Providers, env vars and example model strings |
| `oxgraft strategies`, `oxgraft profile <path>` | Introspection (from M1) |

Common flags: `--model provider/model`, `--profile <name|path>`, `--max-turns`,
`--yes` (approve all), `--cwd <dir>`.

## Config resolution (first wins)

flags → env (`OXGRAFT_MODEL`, `OXGRAFT_PROFILE`) → `.oxgraft/config.toml` in
the workspace → user config (`$XDG_CONFIG_HOME/oxgraft/config.toml` or
`~/.config/oxgraft/config.toml`, `%APPDATA%\oxgraft\config.toml`) → defaults.
Default model: `anthropic/claude-opus-5-5` if `ANTHROPIC_API_KEY` is set,
otherwise an error explaining how to choose one.

## Acceptance criteria

- **AC1** `run --json` prints only protocol events on stdout (one per line);
  human messages go to stderr.
- **AC2** Exit codes: 0 done, 1 error, 2 usage/config error, 3 stopped by a
  guard (max turns, refusal).
- **AC3** Sessions are saved under `.oxgraft/sessions/<id>.json` after every
  turn with an atomic write; `--resume` continues with the same history.
- **AC4** `undo` restores every file the session wrote to its pre-session
  content (deleting files the session created) and reports what it did.
- **AC5** Profiles resolve by name from `profiles/` in the workspace, then the
  built-in defaults, or by path.
- **AC6** End-to-end test: with the mock provider, `oxgraft run` reads a
  file, edits it, runs a command, and exits 0 with the file changed.
