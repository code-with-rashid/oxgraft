# Oxgraft

**The best of every coding agent, grafted into one.**

Oxgraft is an open source coding-agent harness written in Rust. It keeps
studying every major open source harness (Codex, OpenCode, Claude Code's public
behaviour, Gemini CLI, goose, Aider, Cline, Pi, OpenHands, SWE-agent and more),
**grafts in their best ideas, avoids their limitations, and records the
evidence for each change.**

- **Any model.** Anthropic (native API, prompt caching, refusal fallbacks), OpenAI,
  OpenRouter, DeepSeek, Groq, Gemini, xAI, Mistral, Ollama, LM Studio, or any
  OpenAI-compatible server. Replies stream as they are written.
- **Small, sharp tools.** read, write, edit (several formats, batched edits), ls,
  glob, grep, a ranked repo `map`, bash, todo, web fetch, plus Agent Skills, MCP
  servers and a read-only helper agent.
- **Safe by default.** File tools stay inside the workspace; edits and commands
  ask first, or pick a mode (`--mode read-only | auto-edit | full-auto`);
  pipe-to-shell, `rm -rf`, raw disk writes and `sudo` always ask; agent config,
  CI and `.git` edits always ask; optional OS sandbox (bubblewrap / Seatbelt) that
  fails closed; a cloned repository can't run its hooks or MCP servers, or choose
  your model, profile or approval mode, until you trust it; web and MCP content
  is fenced as untrusted.
- **Easy to undo.** Every changing turn is checkpointed in a private git repo
  (`oxgraft restore N`), including changes made by shell commands; each run ends
  with a summary of what changed.
- **Stable in long runs.** Retries with backoff, loop and mistake guards,
  malformed tool calls re-asked privately, max-turn and token budgets, context
  compaction, Ctrl-C interrupts, crash-safe resumable and forkable sessions, and
  `run --until "<check>"` so "done" means the check passes.
- **Scriptable.** `oxgraft run --json` streams a versioned JSONL event protocol
  with meaningful exit codes.

> Status: early but working. See [STATUS.md](STATUS.md) and [ROADMAP.md](ROADMAP.md).

## Quick start

```sh
cargo install --git https://github.com/code-with-rashid/oxgraft oxgraft
export ANTHROPIC_API_KEY=...            # or: --model ollama/qwen3, openai/<model>, ...
oxgraft                                 # interactive
oxgraft run "add a --verbose flag and test it"
```

Full guide: [docs/usage.md](docs/usage.md).

## How it learns

```
             ┌──────────── learning/ ────────────┐
 upstream ──▶│ watch → review → graft proposal    │
 harnesses   └───────────────┬────────────────────┘
                             ▼
  rootstock (core + seams) ◀── scions (strategies, each with a donor + license)
                             │
                 profiles pick one strategy per seam
                             ▼
               evaluate vs default and baseline ──▶ promote, keep, or reject
```

- **Seams** are capability traits (edit format, context, approval, sandbox).
  **Strategies** are interchangeable answers to a seam; `oxgraft strategies`
  shows each one and the harness it came from.
- **Grafts** (`learning/grafts/`) record every idea adopted or avoided, with
  provenance and tests. [learning/iterations.md](learning/iterations.md) logs each
  learning iteration.
- **Specs** (`specs/`) define behaviour first; tests reference their acceptance criteria.

## Contributing

Humans and AI agents are both welcome. Start with [AGENTS.md](AGENTS.md) and
[CONTRIBUTING.md](CONTRIBUTING.md). `pnpm check` runs everything CI runs.

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
