# Capability matrix

Seams × harnesses. **Bold** = Oxgraft's current default. Cells for donors are
filled from `harnesses/<id>.md`; "?" means not yet reviewed. Update this file
in the same change that updates a harness profile or promotes a graft.

| Seam | Oxgraft today | Strategies available | Ideas to evaluate (donor → graft) |
|---|---|---|---|
| edit_format | **search_replace** (trial, GRAFT-0001) | whole_file, search_replace | Codex `apply_patch` (codex); fuzzy matching (aider) |
| context | **pinned_then_in_order** (baseline) | pinned_then_in_order | repo map (aider); auto-compaction (pi, opencode, claude-code); retrieval |
| approval | **ordered_rules** (trial, GRAFT-0002) | ordered_rules, allow_all | approval modes (codex); hooks (claude-code) |
| sandbox | **none** (baseline) | none | Landlock/Seatbelt (codex); containers (openhands) |

## Planned seams (each needs an ADR before it exists)

| Seam | Why | Likely donors |
|---|---|---|
| provider | model APIs behind one trait | goose, opencode, pi |
| tools | built-in tool set and descriptions | mini-swe-agent (minimal), swe-agent (tool design), pi |
| session | persistence, fork, resume, rewind | opencode, cline (checkpoints), pi |
| extension | MCP, skills, hooks, plugins | goose (MCP-first), claude-code (skills, hooks), deepseek-harness |
| delegation | subagents | opencode, claude-code |

## By harness

| Harness | Reviewed | Edit format | Context | Approval | Sandbox |
|---|---|---|---|---|---|
| codex | no | ? | ? | ? | ? |
| goose | no | ? | ? | ? | ? |
| opencode | partial (via agentic-arena) | ? | ? | ordered allow/ask/deny | none built in |
| claude-code | no | ? | ? | ? | ? |
| gemini-cli | no | ? | ? | ? | ? |
| pi | partial (via agentic-arena) | ? | compaction | none built in | none built in |
| deepseek-harness | partial (via agentic-arena) | ? | ? | presets + policy plugins | profile-selected |
| aider | partial (via agentic-arena) | multiple formats | repo map | user interaction | none |
| cline | partial (via agentic-arena) | ? | ? | auto-approve + interactive | none |
| openhands | partial (via agentic-arena) | ? | ? | security analyzer + confirmation | local or container |
| swe-agent | partial (via agentic-arena) | ? | ? | blocklists | SWE-ReX deployments |
| others in watch.toml | no | ? | ? | ? | ? |
