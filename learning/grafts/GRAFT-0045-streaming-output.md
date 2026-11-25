---
id: GRAFT-0045
title: Stream model output through the provider seam
kind: adopt
status: trial
seam: provider
donor: opencode
donor_version: ""
license: MIT
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Every reviewed harness (OpenCode, Codex, Claude Code, Gemini CLI, Aider, …) streams model output; Oxgraft printed replies only when complete. No code was adapted: the design follows the public Anthropic and OpenAI streaming formats.

## Why it matters

Without streaming, long replies look like a hang, and users can't interrupt early when the model heads the wrong way.

## Hypothesis

Interactive use feels responsive with no change to agent behaviour, since the completion is identical.

## Implementation

`Provider::stream` (default: complete, then report once; ADR 0007). Anthropic and OpenAI-compatible providers rebuild the blocking response body from SSE events and reuse their response parsers; OpenAI-compatible falls back to a plain request on a 4xx. Core emits `text_delta` events; the terminal renderer prints them live. `stream = false` / `OXGRAFT_STREAM=0` turns it off.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-providers `stream_events_rebuild_the_response`, `stream_chunks_rebuild_the_response`, `anthropic_streams_over_http`, `openai_compatible_streams_over_http`, `parses_server_sent_events`; oxgraft-core `replies_stream_as_text_deltas_then_full_messages`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
