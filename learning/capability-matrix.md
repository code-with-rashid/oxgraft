# Capability matrix

Seams × harnesses. **Bold** = Oxgraft's current default. "?" means not yet
reviewed at a pinned upstream version. Update this file in the same change that
updates a harness profile or adds or promotes a strategy.

## Oxgraft today

| Seam / area | Default | Also available | Ideas still to evaluate (donor) |
|---|---|---|---|
| edit_format | **search_replace** (GRAFT-0001) | whole_file, search_replace_tolerant (GRAFT-0015), apply_patch (GRAFT-0024) | fuzzy diff (cline) |
| context | **pinned_then_in_order** + compaction (GRAFT-0023) | `map` tool (0053) | ranked repo map with tree-sitter (aider), retrieval |
| approval | **ordered_rules** (GRAFT-0002) + risk rails (0042) | inert_reads (0047), allow_all; modes read-only/auto-edit/full-auto (0052) | per-tool modes in chat |
| sandbox | **none** | workspace_write (GRAFT-0025) | Landlock without bwrap (codex), containers (openhands) |
| provider | Anthropic native, 10 OpenAI-compatible presets (GRAFT-0009), streaming (GRAFT-0045) | mock | Bedrock/Vertex |
| tools | read, write, edit, ls, glob, grep, map, bash, todo, web_fetch | skill, MCP, task | LSP diagnostics (crush/opencode) |
| session | save/resume/repair, undo (GRAFT-0005, 0020), per-turn checkpoints (0046), fork (0049) | budgets (0021) | session tree UI (pi) |
| extension | skills (0017), hooks (0018), MCP (0022), workspace trust (0019) | — | plugins (deepseek-harness), recipes (goose) |
| delegation | read-only `task` subagent (0026) | — | named subagents with policies (opencode) |

## By harness

Every watched harness except Roo Code (archived) was reviewed at a pinned tag on 2026-11-25; the per-seam detail, strengths and limitations with sources are in `learning/harnesses/<id>.md`, and the ideas not yet taken are ranked in ROADMAP.md ("Backlog from reviews"). Grafts per donor:

| Harness | Reviewed at | Grafts taken into Oxgraft |
|---|---|---|
| codex | rust-v0.172.0 | 0006, 0008, 0010, 0024, 0025, 0030, 0032, 0034, 0043, 0051, 0052, 0073, 0075, 0089, 0094 |
| claude-code | v2.1.332 (docs) | 0012, 0013, 0016, 0017, 0018, 0028, 0031, 0036, 0056, 0057, 0062, 0067, 0069, 0070, 0071, 0076, 0077, 0078, 0080, 0081, 0085, 0092, 0093 |
| opencode | v1.19.6 | 0002, 0003, 0009, 0014, 0026, 0038, 0045, 0059, 0060, 0061, 0066, 0074, 0086, 0087 |
| gemini-cli | v0.71.1 | 0027, 0037, 0058 |
| aider | v0.86.0 | 0001, 0011, 0015, 0035, 0039, 0053, 0064, 0065, 0079, 0082, 0083, 0088 |
| goose | v1.61.0 | 0019, 0022, 0033, 0055, 0072, 0090, 0091 |
| pi | v1.6.0 | 0023, 0049, 0050 |
| cline | v4.1.27 | 0005, 0020, 0029, 0041, 0046, 0084 |
| openhands | v1.45.0 (sdk v1.64.0) | 0021, 0042 |
| swe-agent | v1.1.0 | 0004, 0044, 0048, 0063 |
| mini-swe-agent | v2.4.6 | 0007 |
| qwen-code | v0.28.4 | 0040, 0054 |
| kilocode | v7.8.32 | — |
| continue | v2.0.0-vscode (archived) | — |
| deepseek-harness | dsh-v0.2.3-alpha.2 | — |
| crush | v0.102.0 (FSL, clean-room) | 0047 |
| roo-code | not reviewed (archived) | — |
