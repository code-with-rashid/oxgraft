---
id: claude-code
repo: anthropics/claude-code
license: proprietary
policy: reference
last_reviewed: "v2.1.331"
status: reviewed
---

# Claude Code

[anthropics/claude-code](https://github.com/anthropics/claude-code) · proprietary · policy `reference`

> The public repo holds issues, plugins and the changelog, not the source.

Reviewed at `v2.1.331` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

> Closed source: everything below is from public documentation and the changelog (docs read 2026-11-25; the docs are unversioned). Nothing could be verified against code.

## By seam

| Seam | What it does | Strength or limitation? | Source |
|---|---|---|---|
| edit_format | Exact `old_string`→`new_string`, unique unless `replace_all`; no fuzzy matching | Strength: never edits the wrong place. Limitation: whitespace mismatches fail | [tools-reference](https://code.claude.com/docs/en/tools-reference) |
| context | CLAUDE.md, git status snapshot, auto-compact and `/compact`; unknown model ids assumed to have 1M context | Mixed | [changelog](https://github.com/anthropics/claude-code/blob/v2.1.331/CHANGELOG.md) |
| approval | deny → ask → allow regardless of specificity; modes incl. acceptEdits, dontAsk, bypassPermissions, plan, classifier-based auto; bash rules split compound commands | Strength | [permissions](https://code.claude.com/docs/en/permissions) |
| sandbox | Seatbelt (macOS), bubblewrap (Linux/WSL2); egress via local proxy with empty-by-default `allowedDomains`; protected paths (`.claude`, hooks, `.mcp.json`, shell rc) unwritable | Strength; no sandbox on native Windows | [sandboxing](https://code.claude.com/docs/en/sandboxing) |
| provider | Anthropic API, Bedrock, Google Cloud, Microsoft Foundry, gateways | Claude models only (as documented) | [changelog](https://github.com/anthropics/claude-code/blob/v2.1.331/CHANGELOG.md) |
| tools | Read, Edit, Write, Bash, PowerShell, LSP diagnostics after edits, Monitor, ToolSearch (lazy MCP tools) | Strength | [tools-reference](https://code.claude.com/docs/en/tools-reference) |
| session | checkpoint before every prompt (last 100), `/rewind` code and/or conversation, resume, fork | Mixed | [checkpointing](https://code.claude.com/docs/en/checkpointing) |
| extension | hooks (exit 2 blocks; deny/ask rules still apply), plugins, skills, MCP | Strength: documented precedence | [permissions](https://code.claude.com/docs/en/permissions) |
| delegation | markdown subagents with tools/disallowedTools/model/`isolation: worktree`; background, up to 20 concurrent, nesting up to 3 | Strength | [sub-agents](https://code.claude.com/docs/en/sub-agents) |

## Strengths worth grafting

- Deny always beats allow, independent of specificity.
- Compound shell commands are split before rule matching; unparseable commands are never auto-allowed.
- Sandbox network egress through a proxy with an empty-by-default allowlist.
- `isolation: worktree` for subagents that need to write.
- LSP diagnostics after each edit.
- Partial "summarize from / up to here" rewinds.

## Limitations to avoid

- Native Windows runs Bash unsandboxed.
- Read/Edit/Write/WebFetch ignore the sandbox's `denyRead` and `allowedDomains`; sandboxed commands can read `~/.ssh` and `~/.aws/credentials` by default.
- `/rewind` doesn't restore files changed only by Bash, by background subagents, or via links.
- Argument-restricting Bash rules can be bypassed with variable indirection (`URL=... && curl $URL`).
- Unknown model ids assumed to have 1M context, so compaction can start too late behind gateways.
- `excludedCommands` run fully unsandboxed, so broad patterns defeat the sandbox.

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v2.1.331 | research agent for the Oxgraft build session | deny-first rules, egress allowlist proxy, worktree subagents; no Windows sandbox, file tools bypass sandbox reads |
