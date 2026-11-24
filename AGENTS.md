# AGENTS.md

Instructions for any AI coding agent working on Oxgraft (Claude Code, Codex,
Gemini CLI, Cursor, Copilot, OpenCode, Oxgraft itself, …). `CLAUDE.md`,
`GEMINI.md` and `.github/copilot-instructions.md` point here. This file is the
single source of truth; edit it, not the pointers.

## Mission

Oxgraft is a Rust coding-agent harness whose core job is to **keep learning
from every major open source harness**: graft in their best ideas, avoid their
limitations, and prove every change with evidence. Read `learning/README.md`
before any learning-related task.

## Start of every session

1. Read `STATUS.md`: current milestone, what's in flight, known problems.
2. Read `ROADMAP.md` for the milestone you are working on.
3. Pick **one** task. Prefer the first unchecked item of the current milestone,
   or the task the user gave you.
4. If the task matches a skill in `.agents/skills/`, follow that skill.

## End of every session (the handoff)

Follow `.agents/skills/session-handoff/SKILL.md`. In short: update `STATUS.md`
(what changed, what's next, what's broken), tick `ROADMAP.md`, and make sure
`pnpm check` passes. The next agent may be a different tool with no memory of
you: `STATUS.md` is all it gets.

## Repository map

```
crates/
  oxgraft-seams/       capability traits ("seams"); small, stable; ADR to change
  oxgraft-strategies/  one module per strategy, grouped by seam; carries Provenance
  oxgraft-core/        profiles, strategy assembly, session loop; NEVER depends on strategies
  oxgraft-protocol/    versioned JSONL event stream; append-only
  oxgraft-providers/   model backends: anthropic (raw HTTP), OpenAI-compatible presets, mock
  oxgraft-tools/       built-in tools, skills, MCP client, OS sandbox wrapper
  oxgraft-cli/         the `oxgraft` binary; config, approvals UI, wires everything into core
specs/                 SPEC-NNNN behaviour specs; tests cite their acceptance criteria
profiles/              strategy combinations (default, baseline, ci, tolerant, codex-like, sandboxed)
learning/              watch list, harness profiles, capability matrix, graft records
docs/adr/              architecture decisions; read before changing a seam
web/site/              docs site (Astro + Starlight); renders docs/ and learning/
scripts/               repo checks and the harness watcher (Node, no build step)
packaging/             npm and PyPI distribution wrappers
.agents/skills/        procedures for recurring work (mirrored to .claude/skills/)
```

## Commands

| Task | Command |
|---|---|
| Everything CI runs | `pnpm check` |
| Rust only | `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` |
| Run the CLI | `cargo run -p oxgraft -- strategies` · `cargo run -p oxgraft -- profile profiles/default.toml --json` |
| Repo invariants | `pnpm check:repo` (layering, skills mirror, learning records) |
| Upstream drift report | `pnpm watch` (set `GITHUB_TOKEN` to avoid rate limits) |
| Docs site | `pnpm site:dev` · `pnpm site:build` |

Setup: Rust stable via rustup, Node 22+, pnpm 9+. On Windows, develop in WSL
or install the MSVC build tools; CI builds Linux, macOS and Windows.

## Spec-driven workflow

1. New behaviour starts as a spec in `specs/` (or an edit to an existing one)
   with numbered acceptance criteria.
2. Write tests that cite the criteria (`// SPEC-0003 AC4`).
3. Implement until they pass; then log the iteration in `learning/iterations.md`
   and, if the idea came from another harness, add or update its graft record.
4. End-to-end behaviour is tested with the `mock:` provider (see
   `crates/oxgraft-cli/tests/e2e.rs`); provider HTTP code with a local fake server.

## Architecture rules (enforced by tests or `pnpm check:repo`)

- **`oxgraft-core` must not depend on `oxgraft-strategies`, `-providers` or `-tools`.**
  Core knows seams; the binary injects implementations. Strategies, providers and
  tools depend on `oxgraft-seams` only.
- **History is append-only.** Never edit or delete earlier messages in place
  (provider-specific blocks such as Anthropic thinking must round-trip unchanged);
  compaction replaces the whole history with a summary instead.
- **Fail closed on safety.** If a sandbox, trust check or approval cannot be
  enforced, refuse the action; never silently run it unconfined.
- **A new idea is a new strategy, not an edit to core.** If a seam can't
  express it, write an ADR (`docs/adr/`) proposing the seam change first.
- **Every strategy declares `Provenance`.** Borrowed ideas name donor, SPDX
  license, origin (`adapted` or `clean_room`) and graft id. Tests enforce it.
- **The event protocol is append-only.** Don't remove or rename events or
  fields; breaking changes bump `PROTOCOL_VERSION` with an ADR.
- **Defaults change only with evidence.** Edit `profiles/default.toml` only
  when a graft record shows an evaluation that beats the current default.

## License rules for learning from other harnesses (binding)

Check the donor's `policy` in `learning/watch.toml`:

- `adapt` (MIT, Apache-2.0): you may adapt code. Keep it small, add the
  upstream copyright and license to `NOTICE`, set `origin: Adapted`.
- `clean-room`: learn from behaviour and docs only. Do not open or paste
  source into this repo. Set `origin: CleanRoom`.
- `reference` (closed source, e.g. Claude Code): public docs, changelogs and
  observed behaviour only.

When unsure, treat it as `clean-room` and say so in the graft record.

## Code conventions

- Rust 2024 edition, stable toolchain, `unsafe` forbidden, clippy clean with `-D warnings`.
- Errors: `thiserror` in libraries; the CLI prints `oxgraft: <message>` and exits non-zero.
- Tests live next to code (`#[cfg(test)]`). Every strategy has unit tests for
  its success path and each error variant.
- Scripts are dependency-light ESM in `scripts/` and run with plain `node`.
- Commits: Conventional Commits (`feat(strategies): …`, `docs(learning): …`).
  No AI attribution lines or co-author trailers.

## Definition of done

- `pnpm check` passes locally (or in CI if you can't build locally).
- New strategies: registered, provenance complete, graft record updated,
  `learning/capability-matrix.md` updated.
- Harness reviews: `learning/harnesses/<id>.md` and `last_reviewed` updated together.
- `STATUS.md` updated.

## Don't

- Don't delete rejected graft records; they prevent re-proposals.
- Don't add a dependency to `oxgraft-seams` without an ADR.
- Don't commit secrets, API keys, `.env` files, or recorded sessions with credentials.
- Don't mark a graft `adopted` yourself; propose it and let a maintainer decide.
