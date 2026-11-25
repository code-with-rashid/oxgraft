---
id: GRAFT-0092
title: Refuse pipes and devices in file tools
kind: avoid
status: trial
seam: tools
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code v2.1.332 release notes: Read, @-mentions and compaction hung on a named pipe nothing writes to (e.g. a .env mounted by a secrets manager).

## Why it matters

Oxgraft's read, edit and write opened such files directly and would block the session forever.

## Hypothesis

No tool call can hang on a special file.

## Implementation

`special_file` checks metadata before reading or writing: anything that is neither a regular file nor a directory gets a clear error. @-mentions and grep already skipped non-regular files.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `special_files_are_refused` (Unix)). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
