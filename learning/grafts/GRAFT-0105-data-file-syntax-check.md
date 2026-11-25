---
id: GRAFT-0105
title: Report broken JSON and TOML right after an edit
kind: adopt
status: trial
seam: tools
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

OpenCode feeds LSP diagnostics back after edits; Aider lints edited files and asks the model to fix errors.

## Why it matters

A trailing comma in package.json or a bad quote in Cargo.toml breaks every later command; without immediate feedback the model finds out turns later.

## Hypothesis

Broken config files are fixed in the next turn instead of derailing the run.

## Implementation

`write` and `edit` parse `.json` and `.toml` files after changing them and append `warning: the file is no longer valid …` with the parser's message. No external tools needed; full LSP diagnostics remain on the backlog.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `broken_json_and_toml_are_reported`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
