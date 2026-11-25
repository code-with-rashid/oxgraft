---
id: GRAFT-0091
title: Strip terminal control sequences from model output
kind: avoid
status: trial
seam: session
donor: goose
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

goose v1.61.0 release notes: "Strip terminal controls from assistant markdown".

## Why it matters

Model text (or a web page it quotes) can contain ANSI/OSC sequences that clear the screen, rewrite earlier lines, set the window title or fake hyperlinks and prompts.

## Hypothesis

What the terminal shows is exactly what the model wrote, minus control codes.

## Implementation

`render::plain` drops CSI, OSC (BEL or ST terminated) and two-character escapes plus other control characters (keeping newlines and tabs) for streamed text, full messages and one-line tool summaries. Approval previews already neutralise control characters (iteration 118).

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli `terminal_controls_are_stripped`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
