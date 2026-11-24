---
id: GRAFT-0017
title: Load Agent Skills with progressive disclosure
kind: adopt
status: trial
seam: extension
donor: claude-code
donor_version: ""
license: proprietary
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Claude Code, Codex, Gemini CLI, Cursor and Copilot read the open Agent Skills format (SKILL.md with name/description frontmatter), listing descriptions and loading bodies on demand.

## Why it matters

Users' existing skills (including the agent-skills catalogue) work in Oxgraft unchanged, without bloating every prompt.

## Hypothesis

Skills are invoked when relevant at a cost of one short line per skill in the system prompt.

## Implementation

Discovers `.agents/skills` and `.claude/skills` in the workspace and home; lists name and description in the system prompt; `skill` tool returns the SKILL.md body.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `discovers_and_loads_workspace_skills`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
