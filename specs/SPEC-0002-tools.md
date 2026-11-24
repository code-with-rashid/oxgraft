---
id: SPEC-0002
title: Built-in tools
status: implemented
grafts: [GRAFT-0003, GRAFT-0004, GRAFT-0005]
---

# SPEC-0002: Built-in tools

## Goal

A small, sharp tool set (lesson from Pi and mini-SWE-agent: fewer, clearer
tools beat many overlapping ones) that is safe by default.

## Tools

| Tool | Input | Notes |
|---|---|---|
| `read` | `path`, `offset?`, `limit?` | Numbered lines (Claude Code / OpenCode style); default 2000 lines |
| `write` | `path`, `content` | Creates parent dirs |
| `edit` | `path`, `edit` | `edit` uses the profile's `EditFormat`; its instructions are in the tool description |
| `ls` | `path?` | One level, dirs marked `/` |
| `glob` | `pattern`, `path?` | Respects `.gitignore` |
| `grep` | `pattern`, `path?`, `glob?` | Regex, respects `.gitignore`, `file:line: text` |
| `bash` | `command`, `timeout_secs?` | `sh -c` / `cmd /C`, workspace cwd, timeout kills the process |

## Acceptance criteria

- **AC1** Every tool validates its input and returns a tool error (not a
  crash) for missing or wrongly typed fields.
- **AC2** File tools refuse paths outside the workspace, including `..` and
  absolute paths elsewhere (GRAFT-0003, avoiding "permissions are not a
  sandbox" from OpenCode/Pi reviews).
- **AC3** Tool output is capped (default 30 000 chars) with a clear
  truncation note, keeping head and tail (SWE-agent's concise-observation lesson, GRAFT-0004).
- **AC4** `bash` enforces a timeout (default 120 s, max 600 s), returns exit
  code, stdout and stderr, and never hangs the loop.
- **AC5** `read` of a binary or missing file returns an explanatory error.
- **AC6** `edit` failures (not found / ambiguous) return the strategy's error
  text so the model can retry, and leave the file unchanged.
- **AC7** `edit` and `write` record the file's previous content so a session
  can be undone (`oxgraft undo`), inspired by Cline checkpoints and Aider's
  git-based recovery (GRAFT-0005).
