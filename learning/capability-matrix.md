# Capability matrix

Seams × harnesses. **Bold** = Oxgraft's current default. "?" means not yet
reviewed at a pinned upstream version. Update this file in the same change that
updates a harness profile or adds or promotes a strategy.

## Oxgraft today

| Seam / area | Default | Also available | Ideas still to evaluate (donor) |
|---|---|---|---|
| edit_format | **search_replace** (GRAFT-0001) | whole_file, search_replace_tolerant (GRAFT-0015), apply_patch (GRAFT-0024) | fuzzy diff (cline) |
| context | **pinned_then_in_order** + compaction (GRAFT-0023) | — | repo map (aider), retrieval |
| approval | **ordered_rules** (GRAFT-0002) | allow_all | approval modes per session (codex) |
| sandbox | **none** | workspace_write (GRAFT-0025) | Landlock without bwrap (codex), containers (openhands) |
| provider | Anthropic native, 10 OpenAI-compatible presets (GRAFT-0009) | mock | Bedrock/Vertex, streaming |
| tools | read, write, edit, ls, glob, grep, bash, todo, web_fetch | skill, MCP, task | LSP diagnostics (crush/opencode) |
| session | save/resume/repair, undo (GRAFT-0005, 0020) | budgets (0021) | git checkpoints per turn (cline), fork (opencode) |
| extension | skills (0017), hooks (0018), MCP (0022), workspace trust (0019) | — | plugins (deepseek-harness), recipes (goose) |
| delegation | read-only `task` subagent (0026) | — | named subagents with policies (opencode) |

## By harness (what we have reviewed)

| Harness | Reviewed | Edit format | Context | Approval | Sandbox | Taken into Oxgraft |
|---|---|---|---|---|---|---|
| codex | documented behaviour | apply_patch | AGENTS.md | modes | Landlock/Seatbelt | 0006, 0008, 0010, 0024, 0025, 0030 |
| goose | via agentic-arena | ? | ? | tool permissions | outer isolation advised | 0019, 0022 |
| opencode | via agentic-arena | ? | compaction | ordered allow/ask/deny | none built in | 0002, 0003, 0009, 0014, 0026 |
| claude-code | public docs/behaviour | str_replace | CLAUDE.md, @imports, compaction | permission rules, hooks | bubblewrap/Seatbelt | 0012, 0013, 0016, 0017, 0018, 0028 |
| gemini-cli | public docs | ? | GEMINI.md | ? | ? | 0006, 0027 |
| pi | via agentic-arena | ? | compaction | none built in | none built in | 0003, 0023 |
| deepseek-harness | via agentic-arena | ? | ? | presets + policy plugins | profile-selected | — |
| aider | via agentic-arena | multiple formats | repo map | user interaction | none | 0001, 0011, 0015 |
| cline | via agentic-arena | diff | ? | auto-approve + interactive | none | 0005, 0020, 0029 |
| openhands | via agentic-arena | ? | ? | security analyzer | local or container | 0021 |
| swe-agent | via agentic-arena | ? | bounded observations | blocklists | SWE-ReX | 0004 |
| mini-swe-agent | documented behaviour | bash only | minimal | none | outer | 0007 |
| others in watch.toml | no | ? | ? | ? | ? | — |
