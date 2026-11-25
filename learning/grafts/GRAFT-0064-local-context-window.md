---
id: GRAFT-0064
title: Small-context defaults and warnings for local models
kind: avoid
status: trial
seam: provider
donor: aider
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Aider's Ollama docs warn that Ollama's default context window is small and that it silently discards the start of prompts that exceed it; Aider sets the window itself.

## Why it matters

Coding agents put their instructions and tool schemas first, so silent head truncation breaks them in confusing ways, and a 200k compaction threshold never triggers on a 32k server.

## Hypothesis

First runs against Ollama fail loudly and fixably instead of producing confused agents.

## Implementation

For `ollama` and `lmstudio` the compaction window defaults to 32,768 tokens unless `context_window` is set; `doctor` warns Ollama users to start the server with `OLLAMA_CONTEXT_LENGTH` and to use a tool-calling model; usage docs explain both. Oxgraft can't set Ollama's window through the OpenAI-compatible API.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli e2e `doctor_warns_about_ollama_context`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
