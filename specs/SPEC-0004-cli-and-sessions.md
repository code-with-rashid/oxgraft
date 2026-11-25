---
id: SPEC-0004
title: CLI, headless mode, config, sessions
status: implemented
grafts: [GRAFT-0005, GRAFT-0046, GRAFT-0048, GRAFT-0049, GRAFT-0052, GRAFT-0063]
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
- **AC7** With `checkpoints = true` (default) and git installed, the workspace is
  snapshotted into a private repo in the user data dir (never inside the
  workspace; git runs with only Oxgraft's own config, no hooks or fsmonitor) at session start and after every
  turn that used a tool other than read-only ones, including changes made by
  shell commands. `oxgraft checkpoints` lists them; `oxgraft restore N` returns
  the workspace to checkpoint N after recording a pre-restore checkpoint. The
  user's own `.git`, index and history are never read or written; `.gitignore`d
  files and heavy directories (node_modules, target, …) are left alone. A
  failing checkpoint repo is a warning, never a session error; checkpoints are
  off when the workspace is the home directory or a filesystem root.
- **AC8** In human mode, a `run` (or chat turn) that changed the workspace ends
  with a `git diff --stat` of the run's changes from its checkpoints and the
  `oxgraft restore` command that undoes all of them; a run that changed nothing
  prints no summary.
- **AC9** `run` and `chat` accept `--fork ID` or `--fork ID@N`: a new session
  starts from a copy of the first N messages of session ID (all if N is
  omitted). N must follow a finished reply (an assistant message without tool
  calls); otherwise the error lists valid fork points. The original session is
  never modified; the fork records `forked_from`.
- **AC10** `--mode` / `approval_mode` / `OXGRAFT_APPROVAL_MODE` selects
  `read-only` (edit, write, bash and MCP tools denied with a reason), `suggest`
  (default rules), `auto-edit` (edit and write allowed) or `full-auto` (edit,
  write and bash allowed). Mode rules sit between the defaults and user rules.
  Protected paths and risk rails still ask in every mode; an unknown mode is a
  usage error; an untrusted workspace can't set the mode.
- **AC11** A Markdown file `<name>.md` in `.agents/commands/` or
  `.claude/commands/` (workspace, then home; names of letters, digits, `-`,
  `_`) is a custom command: `/name args` in chat or `oxgraft run /name args`
  runs its body with `$ARGUMENTS` replaced (or the arguments appended). An
  unknown `/name` in `run` is a usage error listing the available commands.
- **AC12** A run that changed the workspace emits `files_changed` with the
  changed paths (workspace-relative, from the run's checkpoints) just before
  `session_finished`; a run that changed nothing emits none.
- **AC13** `oxgraft export [ID] [-o FILE]` writes the session as Markdown:
  title, model and token counts, each user and assistant message, tool calls,
  and tool results shortened to 1,500 characters; thinking blocks are omitted.
- **AC14** `instructions` from the user config, the workspace config and
  `--append-system-prompt` are all appended to the system prompt, in that order.
- **AC15** `--plan` runs in read-only mode with an instruction to return a
  numbered plan instead of changing anything; it cannot be combined with
  `--mode`.
