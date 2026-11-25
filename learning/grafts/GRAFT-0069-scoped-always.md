---
id: GRAFT-0069
title: Scope 'always' answers for shell commands to the command
kind: adopt
status: trial
seam: approval
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's approval prompt offers "Yes, and don't ask again for `npm test` commands", scoped to a command prefix rather than the whole shell tool.

## Why it matters

An 'always' for `bash` approved every later shell command in the session, so approving a test run also approved `rm`.

## Hypothesis

Users keep the convenience of 'always' without handing over the whole shell.

## Implementation

For bash, 'always' records a key per command part: the first word, or the first two for tools with subcommands (git, cargo, npm, docker, make, …). A later command runs without asking only if every part's key was approved; lines with `$(` or backticks never match. Other tools keep tool-wide 'always'. The prompt says what 'always' will cover.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-core `always_for_bash_is_scoped_to_the_command`, `risky_commands_ignore_always_answers`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.

## Revision (2026-11-25, independent review)

The first version keyed `echo hi > .git/hooks/pre-commit` as `echo`, and
`git -C d status` as plain `git`. Now a command line gives no "always" key if
it contains redirection, substitution, subexpressions, quotes, escapes or `@`;
wrappers and interpreters (`python`, `env`, `xargs`, `find`, `sed`, `awk`, …)
give none; subcommand tools with a flag or `+toolchain` before the subcommand
give none; and every part must have a key. For web_fetch the host comes from
the `url` crate, so `https://evil.com\@docs.rs/` is evil.com.
