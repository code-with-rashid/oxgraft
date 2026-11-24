# Contributing

Thanks for helping Oxgraft learn. Humans and AI agents follow the same rules,
written in [AGENTS.md](AGENTS.md).

## Ways to help

- **Review a harness.** Pick a stub in `learning/harnesses/` and follow
  `.agents/skills/watch-harnesses/SKILL.md`.
- **Propose a graft.** Seen a harness do something better (or worse)? Open a
  "Graft proposal" issue or a PR with a record from `learning/grafts/TEMPLATE.md`.
- **Implement or evaluate a graft** marked `proposed` or `trial`.
- **Roadmap items** in [ROADMAP.md](ROADMAP.md).

## Setup

Rust stable (rustup), Node 22+, pnpm 9+. Then:

```sh
pnpm install
pnpm check
```

## Pull requests

- One idea per PR; Conventional Commit titles.
- `pnpm check` green.
- Learning from another harness? Respect its license policy in
  `learning/watch.toml`, and fill in provenance.
