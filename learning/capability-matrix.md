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

## By harness

Every watched harness except Roo Code (archived) was reviewed at a pinned tag on 2026-11-25; the per-seam detail, strengths and limitations with sources are in `learning/harnesses/<id>.md`, and the ideas not yet taken are ranked in ROADMAP.md ("Backlog from reviews"). Grafts per donor:

| Harness | Reviewed at | Grafts taken into Oxgraft |
|---|---|---|
| codex | rust-v0.172.0 | 0006, 0008, 0010, 0024, 0025, 0030, 0032, 0034 |
| claude-code | v2.1.331 (docs) | 0012, 0013, 0016, 0017, 0018, 0028, 0031, 0036 |
| opencode | v1.19.6 | 0002, 0003, 0009, 0014, 0026, 0038 |
| gemini-cli | v0.71.1 | 0027, 0037 |
| aider | v0.86.0 | 0001, 0011, 0015, 0035, 0039 |
| goose | v1.60.2 | 0019, 0022, 0033 |
| pi | v1.6.0 | 0023 |
| cline | v4.1.27 | 0005, 0020, 0029, 0041 |
| openhands | v1.45.0 (sdk v1.64.0) | 0021 |
| swe-agent | v1.1.0 | 0004 |
| mini-swe-agent | v2.4.6 | 0007 |
| qwen-code | v0.28.4 | 0040 |
| kilocode | v7.8.32 | — |
| continue | v2.0.0-vscode (archived) | — |
| deepseek-harness | dsh-v0.2.3-alpha.2 | — |
| crush | v0.102.0 (FSL, clean-room) | — |
| roo-code | not reviewed (archived) | — |
