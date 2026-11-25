---
id: SPEC-0003
title: Agent loop, approvals, stability guards
status: implemented
grafts: [GRAFT-0006, GRAFT-0007, GRAFT-0044, GRAFT-0054]
---

# SPEC-0003: Agent loop

## Goal

A single, predictable loop: ask the model, run the tools it requests (subject
to approval and sandbox), feed results back, stop when it is done or a guard
trips. Every step is an event on the protocol stream.

## Behaviour

1. Build the system prompt: base instructions + environment (cwd, OS, date) +
   project instructions discovered from `AGENTS.md` / `CLAUDE.md` / `GEMINI.md`
   in the workspace (GRAFT-0006, from Codex/Claude Code/Gemini CLI).
2. Call the provider with the full append-only history and tool specs.
3. Append the assistant message verbatim. Emit `message` / `tool_call` events.
4. For each tool call: approval policy decides; `ask` goes to the `Approver`
   (interactive prompt, or deny in headless mode unless `--yes`). Denied calls
   return an error result telling the model the user declined.
5. All results of a turn go back in one user message.
6. Stop on `end_turn` with no tool calls, on `max_turns`, on a refusal, or on
   a non-retryable provider error.

## Stability guards

- **AC1** `max_turns` (default 50) ends the session with `ok: false` and reason.
- **AC2** Loop detection: the same tool with the same input three times in a
  row gets a warning result instead of running a fourth time (GRAFT-0007,
  addressing the repetition failure mode seen across harnesses).
- **AC3** Unknown tool names return an error result listing available tools.
- **AC4** A tool panic is caught and returned as an error result; the session continues.
- **AC5** `max_tokens` stop with no tool calls asks the model to continue
  (bounded to 3 times) instead of silently ending.
- **AC6** Every session emits `session_started`, one `strategy_selected` per
  seam, and ends with exactly one `session_finished`.
- **AC7** Token usage is accumulated and reported in `session_finished`.
- **AC8** When a reply contains a tool call that cannot run at all (unknown
  tool, input that is not a JSON object, missing required field), the model is
  asked again with the errors, up to 2 times per turn, and the malformed
  attempts are not added to the history. If the corrected reply carries
  provider-signed blocks (thinking), the attempts are kept so history stays
  truthful. After 2 re-asks the reply is processed normally (calls fail).
- **AC9** With `stream = true` each reply's text is emitted as `text_delta`
  events before the full `message` event; with `stream = false` only `message`.
- **AC10** `@path` imports in instruction files resolve only to files inside the
  workspace (after following symlinks); absolute paths, `..` and links out are
  left as plain text, so a repository can't pull private files into the prompt.
- **AC11** With a finish check (`oxgraft run --until "<command>"`), a reply
  without tool calls ends the run only if the command passes; otherwise its
  output goes back to the model and the run continues, up to 3 times, after
  which the run stops with `finish check … still failing`. The check comes only
  from the command line, never from workspace config.
- **AC12** `@path` words in a task attach that file's content (text files inside
  the workspace only; at most 10 files, 200 KB each, 20k characters shown) after
  the task text; attached files count as read for the read-before-edit guard.
