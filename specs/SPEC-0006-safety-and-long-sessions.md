---
id: SPEC-0006
title: Sandbox, interrupts, budgets, compaction
status: implemented
grafts: [GRAFT-0020, GRAFT-0021, GRAFT-0023, GRAFT-0025, GRAFT-0042, GRAFT-0076, GRAFT-0094]
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
- **AC6** Under `ordered_rules`, shell commands that download and execute code,
  delete recursively with force, write to raw disks, format filesystems, or use
  sudo are never auto-allowed by a rule or by an earlier "always" answer; each
  one is asked about with its reason (`ApprovalRequested.risk`). A deny rule
  still wins.
- **AC7** When a sandboxed shell command fails with a write denial (read-only
  file system, permission denied) or, without network, a DNS/network error, the
  result ends with a note that the sandbox likely caused it and that the model
  should tell the user rather than retry. The sandbox is never widened
  automatically.
- **AC8** Chat `/compact` compacts the session immediately, the same way as
  automatic compaction; an empty session is left alone.
- **AC9** Shell commands run by the model get no environment variable whose
  name contains KEY, TOKEN, SECRET, PASSWORD, PASSWD or CREDENTIAL, unless it
  is listed in `OXGRAFT_SHELL_ENV_KEEP`.
