# web/

Web projects for Oxgraft, managed as a pnpm workspace from the repo root.

| Package | What | Commands |
|---|---|---|
| `site/` (`@oxgraft/site`) | Docs and learning site (Astro + Starlight). Renders `docs/`, `learning/` and the top-level guides; those files stay the source of truth. | `pnpm site:dev`, `pnpm site:build` |

Planned: `console/`, a web viewer that replays `oxgraft --json` session
event streams (see `crates/oxgraft-protocol`).
