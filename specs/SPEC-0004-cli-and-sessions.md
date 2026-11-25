---
id: SPEC-0004
title: CLI, headless mode, config, sessions
status: implemented
grafts: [GRAFT-0005, GRAFT-0046, GRAFT-0048, GRAFT-0049, GRAFT-0052, GRAFT-0063, GRAFT-0075, GRAFT-0077, GRAFT-0082, GRAFT-0083, GRAFT-0088, GRAFT-0089]
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
- **AC16** `oxgraft prune [--keep N] [--dry-run]` deletes all but the newest N
  sessions (default 50) and their undo backups; other files are untouched.
- **AC17** `run --quiet` prints only the final answer on stdout (warnings and the
  stop reason on stderr); `--output-last-message FILE` also writes it to FILE;
  `--quiet` cannot be combined with `--json`.
- **AC18** In chat, `/mode X` switches the approval mode for the following
  messages; `/mode` shows the current mode; an unknown mode is reported and
  changes nothing.
- **AC19** In chat, a line that is just `"""` starts a multi-line message that
  ends at the next `"""`, and a line ending in a backslash continues on the next
  line.
- **AC20** In chat, `/model M` switches the model for the following messages,
  keeping the conversation; an invalid model is reported and changes nothing.
- **AC21** `oxgraft diff [ID]` (and `/diff` in chat) prints the patch from the
  session's first checkpoint to the current workspace.
- **AC22** `oxgraft config` prints the resolved settings, their sources, the
  trust state and all config warnings, using the same layering as a run.
