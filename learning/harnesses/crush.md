---
id: crush
repo: charmbracelet/crush
license: FSL-1.1-MIT
policy: clean-room
last_reviewed: "v0.102.0"
status: reviewed
---

# Crush

[charmbracelet/crush](https://github.com/charmbracelet/crush) · NOASSERTION · policy `clean-room`

> GitHub reports no standard SPDX license; treat as source-available until checked.

Reviewed at `v0.102.0` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

> License is FSL-1.1-MIT (source-available; Competing Use excluded; each version becomes MIT after two years). Oxgraft is a competing agent: clean-room only, README and docs read, no source.

Sources: [https://github.com/charmbracelet/crush/blob/v0.102.0](https://github.com/charmbracelet/crush/blob/v0.102.0) (README, docs, and source where marked Verified).

## Strengths worth grafting

- Read-only bash auto-runs only when the whole command parses as inert (no redirection, substitution, pipeline, assignment or mutating argument) (README)
- Claude Code-compatible hooks with first-deny-wins (docs/hooks)
- MCP channels that push CI/webhook events into a session

## Limitations to avoid

- Hooks do not fire inside subagents (policy bypass via task)
- Config is executable at load time ($(...) in crush.json, crushrc)
- `--yolo` on a shared serve workspace is first-client-wins

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v0.102.0 | research agent for the Oxgraft build session | FSL-1.1-MIT (clean-room only); inert-command auto-allow, first-deny-wins hooks; hooks skip subagents, executable config |
