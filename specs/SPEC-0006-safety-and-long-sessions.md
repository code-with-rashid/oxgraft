---
id: SPEC-0006
title: Sandbox, interrupts, budgets, compaction
status: implemented
grafts: [GRAFT-0020, GRAFT-0021, GRAFT-0023, GRAFT-0025]
---

# SPEC-0006: Safety and long sessions

## Acceptance criteria

- **AC1** With `sandbox = "workspace_write"`, shell commands can write only in the
  workspace and a private /tmp and have no network; if no OS sandbox works on
  the machine, the command is refused (fail closed).
- **AC2** The first Ctrl-C stops the run before the next model turn or tool call
  and kills a running shell command within ~200 ms; the second exits with 130.
- **AC3** Resuming a session whose last assistant message has unanswered tool
  calls first closes them with error results.
- **AC4** `max_session_tokens` stops a run at the start of the next turn once
  input + output tokens exceed it.
- **AC5** When a turn's context exceeds 80% of `context_window`, the model writes
  a handover summary and the session continues from task + summary; history is
  never edited in place.
