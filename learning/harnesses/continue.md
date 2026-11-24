---
id: continue
repo: continuedev/continue
license: Apache-2.0
policy: adapt
last_reviewed: "v2.0.0-vscode"
status: reviewed
---

# Continue

[continuedev/continue](https://github.com/continuedev/continue) · Apache-2.0 · policy `adapt`

Reviewed at `v2.0.0-vscode` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

> README: no longer maintained, read-only; 2.0.0 was the final release.

Sources: [https://github.com/continuedev/continue/blob/v2.0.0-vscode](https://github.com/continuedev/continue/blob/v2.0.0-vscode) (README, docs, and source where marked Verified).

## Strengths worth grafting

- Permission rules with argument globs, e.g. Write(**/*.ts), Bash(npm install*) (docs: cli/tool-permissions.mdx)
- In headless mode, `ask` tools are removed from the agent instead of failing or being auto-allowed
- `exclude` hides a tool from the model entirely, distinct from deny

## Limitations to avoid

- Edits applied via a second `apply` model: extra cost and non-determinism
- Project archived

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v2.0.0-vscode | research agent for the Oxgraft build session | archived upstream; arg-glob permissions, headless removes ask tools |
