---
id: GRAFT-0095
title: Private session files
kind: adopt
status: trial
seam: session
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex and Claude Code keep transcripts under the user's home with owner-only permissions.

## Why it matters

Session files contain prompts, file contents and command output, sometimes secrets; on shared machines a world-readable file leaks them.

## Hypothesis

Other local users can't read Oxgraft sessions.

## Implementation

`Session::save` sets the file to 0600 and the sessions folder to 0700 on Unix (ignored where the filesystem refuses chmod). Windows relies on the user profile's ACLs.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `session_files_are_private` (Unix)). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
