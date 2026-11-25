---
id: SPEC-0002
title: Built-in tools
status: implemented
grafts: [GRAFT-0003, GRAFT-0004, GRAFT-0005, GRAFT-0047, GRAFT-0050, GRAFT-0053, GRAFT-0055, GRAFT-0060]
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
- **AC8** Under the `inert_reads` approval strategy, a shell command runs
  without asking only if the whole line is one known read-only command (`ls`,
  `cat`, `grep`, `find` without -exec/-delete, …; never `git`, which runs repo-configured commands)
  with no redirection, substitution, pipeline, chaining, glob, assignment,
  mutating option, or path outside the workspace, and no argument the rules
  deny reading. Deny rules still win; everything else is decided by `ordered_rules`.
- **AC9** `edit` also accepts `edits: [{old_string, new_string}, ...]`: every
  `old_string` must occur exactly once in the original file and no two may
  overlap; all replacements apply together, or (on any error, naming the edit
  number) none do.
- **AC10** `map` returns source files (Rust, Python, JS/TS, Go, JVM/.NET/Swift,
  Ruby, PHP) with their top-level definitions, files whose defined names are
  used in the most other files first, within a character budget (default 6000,
  max 40000), honouring `.gitignore`, skipping Rust test modules, and saying
  how many files were left out. It is read-only and allowed by default.
- **AC11** Output from `web_fetch` and MCP tools is wrapped in
  `<untrusted source="...">…</untrusted>` (a closing tag inside the content is
  defused), and text with common prompt-injection phrases gets a warning line.
  The system prompt tells the model never to follow instructions inside these
  blocks.
- **AC12** `web_fetch` turns HTML into Markdown-like text: headings as `#`,
  list items as `- `, links as `[text](url)` (page anchors and `javascript:`
  dropped), `<pre>` fenced with whitespace kept, table cells separated by `|`,
  named and numeric entities decoded; script, style, head, nav, footer, forms
  and comments removed. Arbitrary input never panics.
- **AC13** When `bash` output is longer than the output limit, the full output
  is written to `.oxgraft/outputs/` (the 20 newest are kept) and the truncated
  result names that file.
