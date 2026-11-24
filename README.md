# Oxgraft

**The best of every coding agent, grafted into one.**

Oxgraft is an open source coding-agent harness written in Rust. Its core idea:
keep studying every major open source harness (Codex, OpenCode, Claude Code's
public behaviour, Gemini CLI, goose, Aider, Cline, Pi, OpenHands, SWE-agent
and more), **graft in their best ideas, avoid their limitations, and prove
each change with evidence.**

> Status: early scaffold (M1). Not usable as an agent yet. See [STATUS.md](STATUS.md) and [ROADMAP.md](ROADMAP.md).

## How it works

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

- **Seams** are capability traits: edit format, context, approval, sandbox
  (more to come: provider, tools, sessions, extensions, delegation).
- **Strategies** are interchangeable answers to a seam. `oxgraft strategies`
  shows each one and the harness it was learned from.
- **Grafts** are records in `learning/grafts/` of every idea adopted, trialled
  or rejected, with its evidence.

## Try it

```sh
cargo run -p oxgraft -- strategies
cargo run -p oxgraft -- profile profiles/default.toml --json
```

## Contributing

Humans and AI agents are both welcome. Start with [AGENTS.md](AGENTS.md) and
[CONTRIBUTING.md](CONTRIBUTING.md). `pnpm check` runs everything CI runs.

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE) for attributions to
the harnesses whose code we adapt.
