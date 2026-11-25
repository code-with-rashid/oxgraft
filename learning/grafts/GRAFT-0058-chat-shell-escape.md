---
id: GRAFT-0058
title: Run a shell command yourself from chat with !
kind: adopt
status: trial
seam: tools
donor: gemini-cli
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Gemini CLI (`!cmd`, shell mode) and Aider (`/run`) let the user run a command directly and share its output with the model.

## Why it matters

Users often want to show the model a failing test or log; asking the model to run it costs a turn and an approval.

## Hypothesis

Faster loops when the user already knows which command to show.

## Implementation

In chat, `!command` runs in the bash tool's shell (no approval: it is the user's own action), prints the output, and prefixes its last 8k characters to the user's next message.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `chat_bang_runs_a_command_and_passes_its_output_on`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
