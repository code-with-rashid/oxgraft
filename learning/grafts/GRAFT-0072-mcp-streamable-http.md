---
id: GRAFT-0072
title: MCP over Streamable HTTP
kind: adopt
status: trial
seam: extension
donor: goose
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Goose, Claude Code and Gemini CLI connect to remote MCP servers over Streamable HTTP (MCP 2025-06-18), not only stdio.

## Why it matters

Many MCP servers (GitHub's, hosted SaaS servers) are remote; stdio-only means they can't be used at all.

## Hypothesis

Users can plug in hosted MCP servers with a URL and a header.

## Implementation

`ServerConfig` takes `url` and `headers` (with `${VAR}` expansion) instead of `command`; each JSON-RPC message is POSTed with `Accept: application/json, text/event-stream`, replies may be JSON or SSE, `Mcp-Session-Id` is kept and resent. Claude Code `.mcp.json` HTTP entries work as-is. Same trust rules as stdio servers.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `http_servers_work_with_json_and_sse_replies`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
