---
status: accepted
date: 2026-11-25
---

# ADR 0007: Streaming through the provider seam

## Context

Every watched harness streams model output; without it a long answer appears
only when complete, and a slow model looks hung. The provider seam had one
blocking method, `complete`.

## Decision

- Add `Provider::stream(request, on_text)` with a default implementation that
  calls `complete` and reports the whole text once. Providers that can stream
  override it; the returned `Completion` must equal what `complete` returns,
  so core logic (tools, history, retries) is unchanged.
- Native providers rebuild the non-streaming response body from the SSE events
  and reuse their existing response parser, so both paths share one parser.
- Add a `text_delta` protocol event (append-only; protocol stays at version 1).
  The full `message` event is still emitted after each reply, so consumers that
  ignore deltas see no change.
- A stream that fails before completing is retried like any transient error;
  text already shown is followed by a `retrying` event, so renderers start a
  new line.
- Streaming is on by default (`stream = false` in config turns it off).

## Consequences

- Third-party providers keep compiling (default method).
- Usage from OpenAI-compatible streams depends on `stream_options.include_usage`;
  presets that may reject it don't send it and report zero usage when streaming.
