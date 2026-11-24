---
id: GRAFT-0027
title: web_fetch tool
kind: adopt
status: trial
seam: tools
donor: gemini-cli
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Gemini CLI (web_fetch), OpenCode (webfetch) and Claude Code (WebFetch) let the model read a URL the task refers to.

## Why it matters

Tasks often cite docs, issues or changelogs; without fetching, the model guesses.

## Hypothesis

Fewer hallucinated API usages on tasks that reference documentation URLs.

## Implementation

http(s) only, 30 s timeout, 3 MB cap, text/JSON/XML only, HTML converted to text; requires approval under the default rules.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `converts_html`, `fetches_over_http_and_rejects_other_schemes`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
