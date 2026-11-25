---
id: SPEC-0001
title: Conversation model and providers
status: implemented
grafts: [GRAFT-0045]
---

# SPEC-0001: Conversation model and providers

## Goal

Oxgraft works with any AI model that supports tool calling, through one
provider-neutral conversation model and a small `Provider` trait.

## Design

- `oxgraft-seams::model` holds the neutral types: `Message`, `Block`
  (`Text`, `ToolUse`, `ToolResult`, `Opaque`), `ToolSpec`, `Completion`,
  `StopReason`, `Usage`, `ProviderError`.
- `Block::Opaque { provider, raw }` carries provider-specific blocks that must
  be sent back unchanged (e.g. Anthropic `thinking` blocks). A provider
  re-emits only its own opaque blocks and silently drops others.
- History is **append-only**: the loop never edits or removes earlier blocks.
  (Required by Anthropic's preserved-thinking check; also simplest to reason about.)
- `oxgraft-providers` implements:
  - `anthropic`: native Messages API (`anthropic-version: 2026-10-21`), raw
    HTTP; default model `claude-opus-5-5`; `thinking` omitted (adaptive by default).
  - `openai`-compatible Chat Completions, with presets for OpenAI,
    OpenRouter, DeepSeek, Groq, Gemini (OpenAI endpoint), xAI, Ollama,
    LM Studio, and any custom base URL.
  - `mock`: scripted responses from a JSON file, for deterministic tests.
- Model strings are `provider/model` (e.g. `anthropic/claude-opus-5-5`,
  `ollama/qwen3`), or `mock:<path>`.

## Acceptance criteria

- **AC1** Model strings parse into provider + model; unknown providers give an
  error listing the known ones.
- **AC2** Anthropic requests serialise system, tools and messages in API shape;
  tool results from one turn go back in a single user message.
- **AC3** Anthropic responses parse text, tool_use and stop reasons
  (`end_turn`, `tool_use`, `max_tokens`, `refusal`); thinking and unknown
  block types round-trip unchanged as `Opaque`.
- **AC4** OpenAI-compatible requests map tools to `function` tools, tool calls
  to `tool_calls` with string `arguments`, tool results to `role: tool`;
  responses map back, including malformed `arguments` (kept as a string error
  input rather than a crash).
- **AC5** Retries: 408/409/429/5xx and transport errors retry with exponential
  backoff (honouring `retry-after`), up to a limit; 400/401/403/404 do not.
  Errors say which provider, status and message.
- **AC6** Missing API keys produce an actionable error naming the env var.
- **AC7** The mock provider replays turns in order and errors clearly when the
  script runs out.
- **AC8** With streaming on (default), Anthropic and OpenAI-compatible providers
  request server-sent events and report text as it arrives; the completion they
  return equals the non-streaming one (same parser). A stream that ends early or
  sends an error event is a classified error (overloaded = transient). An
  OpenAI-compatible server that rejects the streaming request with a 4xx gets a
  plain request instead.
- **AC9** The model may be given as a short alias (`opus`, `sonnet`, `haiku`,
  `fable`, any case), resolving to the current Anthropic model id.
