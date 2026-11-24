---
id: GRAFT-0022
title: MCP tools over stdio
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

goose composes capabilities MCP-first; Claude Code, Codex, Gemini CLI and OpenCode all load MCP servers, and Claude Code reads a project `.mcp.json`.

## Why it matters

MCP is the shared extension ecosystem; supporting it gives Oxgraft thousands of existing tools.

## Hypothesis

Any stdio MCP server's tools work with no adapter code; a broken server never blocks a session.

## Implementation

`oxgraft-tools::mcp`: JSON-RPC 2.0 over stdio with per-request timeouts, pagination, server-request rejection, tools exposed as `mcp__<server>__<tool>`. Config `[mcp_servers.<name>]` in user config; `.mcp.json` and workspace config only when the workspace is trusted. Hidden `oxgraft __mcp-fixture` server for tests.

## Evaluation

Unit and end-to-end tests cover the behaviour (CLI e2e `mcp_tools_work_end_to_end`; oxgraft-tools mcp tests). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
