---
id: GRAFT-0031
title: `oxgraft doctor` setup check
kind: adopt
status: trial
seam: session
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code's /doctor command checks the installation and configuration and reports problems.

## Why it matters

Most failed first runs are setup problems (no key, wrong model string, missing sandbox, broken MCP server); a single command that names the fix saves users and agents time.

## Hypothesis

Fewer support issues about setup; agents can self-diagnose before a run.

## Implementation

`oxgraft doctor` checks config layers and warnings, model string and credentials, profile, live sandbox, every MCP server, shell, git, instruction files and skills; exits 0 when ready, 2 otherwise.

## Evaluation

Unit and end-to-end tests cover the behaviour (CLI e2e `doctor_reports_problems_and_readiness`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
