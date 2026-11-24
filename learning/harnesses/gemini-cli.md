---
id: gemini-cli
repo: google-gemini/gemini-cli
license: Apache-2.0
policy: adapt
last_reviewed: "v0.71.1"
status: reviewed
---

# Gemini CLI

[google-gemini/gemini-cli](https://github.com/google-gemini/gemini-cli) · Apache-2.0 · policy `adapt`

Reviewed at `v0.71.1` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

> Note at v0.71.1: the README says the project is for enterprise Gemini Code Assist only (no free tier) and is "not tuned for the Gemini 4 series"; it should not be used for model benchmarks. [README](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/README.md)

## By seam

| Seam | What it does | Strength or limitation? | Source |
|---|---|---|---|
| edit_format | `replace` with `old_string`/`new_string`, a required `instruction` describing the change, `allow_multiple`; `write_file`. **Verified:** exact/flexible/regex/fuzzy strategies and an LLM repair step (`FixLLMEditWithInstruction`) for failed edits. | Strength: the instruction enables repair. Limitation: repair costs a model call | [file-system.md](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/tools/file-system.md), [edit.ts](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/packages/core/src/tools/edit.ts) |
| context | GEMINI.md concatenated global → workspace/parents → just-in-time from directories tools touch (up to a trusted root); /compress; PreCompress hook. | Strength: just-in-time subdirectory instructions | [gemini-md.md](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/cli/gemini-md.md) |
| approval | TOML policy engine: tool name, `commandPrefix`, RE2 `argsPattern`; allow/deny/ask_user; Default < Workspace < User < Admin tiers; rules scoped to approval modes. Workspace tier documented as non-functional (issue #18186). | Strength: admin tier, linear-time regex. Limitation: project policies broken | [policy-engine.md](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/reference/policy-engine.md) |
| sandbox | Off unless `-s`; Seatbelt (6 profiles), Docker/Podman, Windows native (integrity levels), gVisor, LXC; "sandbox expansion" offers a one-run widening when a command is denied. | Strength: backends + expansion. Limitation: off by default; Windows labels persist | [sandbox.md](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/cli/sandbox.md) |
| provider | Google only (login/Code Assist, API key, Vertex); automatic model fallback; experimental local Gemma router. | Limitation: single vendor | [model-routing.md](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/cli/model-routing.md) |
| tools | file-system, shell, web-fetch, web-search, memory, todos, tracker, ask-user, MCP resources, activate-skill. | Strength: broad set | [tools.md](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/reference/tools.md) |
| session | Auto-saved sessions, `--resume`, /rewind, /restore; opt-in checkpoints (shadow git + conversation + pending tool call). | Strength: files and conversation restored together. Limitation: opt-in | [checkpointing.md](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/cli/checkpointing.md) |
| extension | Extensions, skills, custom commands, MCP; 11 hook events incl. BeforeModel and BeforeToolSelection, strict JSON over stdout; untrusted folders run in safe mode. | Strength: model-stage hooks | [hooks](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/hooks/index.md), [trusted-folders.md](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/cli/trusted-folders.md) |
| delegation | Built-in investigator/help/generalist/browser agents; markdown custom agents with own tools, MCP, policies; subagents cannot call subagents; A2A remote agents; read-only Plan Mode default. | Strength: recursion blocked, per-agent tools | [subagents.md](https://github.com/google-gemini/gemini-cli/blob/v0.71.1/docs/core/subagents.md) |

## Strengths worth grafting

- Sandbox expansion: a denied sandboxed command can be retried once with specific extra paths or network, after approval.
- Just-in-time loading of instruction files from directories that tools touch, bounded by the trusted root.
- Checkpoints that capture files, conversation and the pending tool call together.
- Policy tiers with an always-winning admin tier; RE2 argument matching.
- Subagents may not spawn subagents.

## Limitations to avoid

- Workspace `.gemini/policies` rules have no effect (documented as broken).
- By default, shell commands run unsandboxed.
- `/restore` has nothing to restore by default (checkpointing off).
- Windows sandbox leaves files with a Low integrity level after exit.
- Cannot use non-Google model providers.
- Utility calls silently fall back across model tiers.

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v0.71.1 | research agent for the Oxgraft build session | sandbox expansion, JIT GEMINI.md, checkpoints with pending call; sandbox off by default, Google-only |
