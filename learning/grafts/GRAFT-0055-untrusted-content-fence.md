---
id: GRAFT-0055
title: Fence and flag content from outside the workspace
kind: adopt
status: trial
seam: tools
donor: goose
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Goose (v1.60.2, adversary-mode docs) detects prompt-injection patterns in tool output; Claude Code's docs warn that fetched content is untrusted.

## Why it matters

Web pages and MCP servers can carry instructions aimed at the agent (seen in this very build: a README telling agents to fetch a remote file).

## Hypothesis

Models follow fewer injected instructions when external content is clearly marked as data and suspicious phrases are flagged.

## Implementation

`util::untrusted(source, body)` wraps web_fetch and MCP results in `<untrusted source=…>` with forged closing tags defused, and appends a warning listing matched injection phrases. The system prompt says never to follow instructions inside these blocks. Detection is a hint, not a filter: nothing is removed.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `untrusted_content_is_fenced_and_flagged`; oxgraft-cli e2e `mcp_tools_work_end_to_end`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
