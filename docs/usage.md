# Using Oxgraft

## Install

From source (Rust 1.88+):

```sh
cargo install --git https://github.com/code-with-rashid/oxgraft oxgraft
```

Prebuilt binaries are attached to GitHub releases once a `v*` tag is pushed
(see `.github/workflows/release.yml`).

## Pick a model

Model strings are `provider/model`. Credentials come from environment variables.

| Provider | Example | Key |
|---|---|---|
| Anthropic | `anthropic/claude-opus-5-5` (default when the key is set) | `ANTHROPIC_API_KEY` |
| OpenAI | `openai/<model>` | `OPENAI_API_KEY` |
| OpenRouter | `openrouter/<vendor>/<model>` | `OPENROUTER_API_KEY` |
| DeepSeek, Groq, Gemini, xAI, Mistral | `deepseek/<model>`, `groq/<model>`, … | `<PROVIDER>_API_KEY` |
| Ollama / LM Studio (local) | `ollama/<model>`, `lmstudio/<model>` | none |
| Any OpenAI-compatible server | `compatible/<model>` + `OXGRAFT_COMPATIBLE_BASE_URL` | `OXGRAFT_COMPATIBLE_API_KEY` (optional) |

`oxgraft models` prints the full table, including base-URL override variables.

Short names work for Anthropic's current models: `--model opus`, `sonnet`,
`haiku` or `fable` (see `oxgraft models`).

### Local models (Ollama, LM Studio)

- Start Ollama with a bigger context, or it silently drops the start of long
  prompts (Oxgraft's instructions and tool list come first):
  `OLLAMA_CONTEXT_LENGTH=32768 ollama serve`.
- Use a model that supports tool calling; `oxgraft doctor --live` checks.
oxgraft config                            # settings in effect here, after every layer, and why
- For local providers Oxgraft assumes a 32k window and compacts the context
  before it fills up; set `context_window` if your server uses another size.

## Run

```sh
oxgraft doctor --live                     # check setup, then send two tiny requests (reply + tool call)
oxgraft                                   # interactive chat (prompts before edits and commands)
oxgraft run "fix the failing test"        # one task, then exit
oxgraft run "explain @src/main.rs"        # @path attaches a file from the workspace
oxgraft run --json --yes "…"              # headless: JSONL events on stdout, approve `ask` actions
oxgraft run -q "summarise CHANGELOG.md" > summary.txt   # only the final answer
oxgraft run --resume last "now add docs"  # continue the latest session
oxgraft run --fork last@4 "try it another way"  # new session from the first 4 messages of another
oxgraft run --until "cargo test" "fix the failing test"  # not done until the check passes
oxgraft sessions                          # list saved sessions
oxgraft export -o run.md                  # the latest session as a Markdown transcript
oxgraft prune --keep 20                   # delete older sessions and their undo backups
oxgraft stats                             # sessions and tokens per model in this workspace
oxgraft undo                              # restore files changed by the latest session
oxgraft checkpoints                       # workspace snapshots of the latest session, one per changing turn
oxgraft restore 2                         # put the workspace back to checkpoint 2 (undoable)
oxgraft diff                              # everything the latest session changed, as a patch
oxgraft --profile sandboxed run "…"       # shell commands confined to the workspace, offline
oxgraft --mode read-only run "explain the auth flow"   # no edits or commands at all
oxgraft run --plan "migrate to the new API"     # read-only; ends with a plan. Then: oxgraft run --resume last "go ahead"
oxgraft --mode auto-edit                  # edits without asking; commands still ask
oxgraft --mode full-auto run "…"          # edits and commands without asking (risky ones still ask)
oxgraft run --allow-tool 'bash:cargo *' --deny-tool web_fetch "…"   # per-run permissions
```

In chat: `"""` starts and ends a multi-line message; `!command` (run it yourself; the output goes with your next message), `/compact`, `/diff`, `/init`, `/mode X`, `/model M`, `/checkpoints`, `/restore N`, `/undo`, `/cost`, `/clear`, `/sessions`,
`/help`, `/exit`. Ctrl-C interrupts the current run (and any running command);
press it again to quit.

Exit codes: `0` done, `1` provider or internal failure, `2` usage/config error,
`3` stopped by a guard (max turns, token budget, refusal, interrupt).

### Headless events (`--json`)

One JSON object per line, each with a `type` (protocol version 1; new event
types and optional fields may be added, so ignore what you don't know):

| type | meaning |
|---|---|
| `session_started` | protocol version, profile, session id, model |
| `strategy_selected` | one per seam: which strategy the profile picked |
| `turn_started` | a request to the model is going out (`turn`) |
| `text_delta` | streamed assistant text; the full text follows in `message` |
| `message` | a complete assistant reply |
| `tool_call` / `tool_result` | a tool call and its result (`ok`, `output`) |
| `approval_requested` / `approval_resolved` | an `ask` decision (`risk` set for risky commands) |
| `retrying` / `warning` | provider retries; anything worth knowing |
| `files_changed` | files the run changed (from its checkpoints) |
| `session_finished` | `ok`, `reason`, `turns`, token counts; always last |

## Configure

`~/.config/oxgraft/config.toml` (Windows: `%APPDATA%\oxgraft\config.toml`), or
`.oxgraft/config.toml` in a workspace. Flags and `OXGRAFT_*` env vars override files.

```toml
model = "anthropic/claude-opus-5-5"
profile = "auto"               # auto (by model family), default, baseline, ci, tolerant, codex-like, sandboxed, fewer-prompts, or a path
effort = "high"                # Anthropic effort: low, medium, high, xhigh, max, none
thinking = true                # adaptive thinking on Anthropic models that support it
bell = true                    # ring the terminal bell when a task of 30 s or more finishes
subagent_model = "haiku"      # cheaper model for the read-only `task` subagent
max_turns = 50
max_tokens = 16000             # per response
max_session_tokens = 2000000   # stop a run above this many tokens
context_window = 200000        # compaction at 80%; 0 disables
instructions = "Use British English. Prefer small commits."   # added to the system prompt (also --append-system-prompt)
verify_command = "auto"       # or a command; after edits, failures go back to the model (trusted workspaces)
stream = true                  # show replies as they are written (OXGRAFT_STREAM=0 to turn off)
checkpoints = true             # snapshot the workspace after changing turns (private git repo in your data dir; OXGRAFT_DATA_DIR overrides)

# User config only: workspaces whose own hooks and MCP servers may run.
trusted_workspaces = ["/home/me/code/my-repo"]

[[hooks]]
event = "post_tool"            # pre_tool | post_tool | session_start | session_end | user_prompt | stop
tools = ["edit", "write"]      # empty = all tools
command = "cargo fmt --all"    # exit 2 = feedback (post_tool, stop) or block (pre_tool, user_prompt);
                               # user_prompt stdout is added to the task

[mcp_servers.github]
command = "github-mcp-server"
args = ["stdio"]
env = { GITHUB_TOKEN = "…" }

[mcp_servers.remote]               # Streamable HTTP server
url = "https://mcp.example.com/mcp"
headers = { Authorization = "Bearer ${EXAMPLE_TOKEN}" }   # ${VAR} reads the environment
```

A project `.mcp.json` (`{"mcpServers": {...}}`, the Claude Code format) is read
only for trusted workspaces. The same goes for a workspace's `model`,
`profile`, `verify_command`, hooks, rules and its own `profiles/*.toml`: a
cloned repository can't change which model you talk to or what runs without
asking until you add it to `trusted_workspaces`.

### Approval rules

Rules are added after the built-in ones (read-only tools allowed; edit, write
and bash ask; `*.env` denied). `*` matches any text. Any matching `deny` wins;
otherwise the last matching rule decides. Shell commands are checked part by
part (`a && b`, `a | b`, `a; b`), so `cargo *` never approves `cargo test && rm -rf ~`.

```toml
[[rules]]
tool = "bash"
resource = "cargo *"
decision = "allow"             # allow | ask | deny
match = ["cargo test", "cargo clippy --all-targets"]   # must match, checked at load
not_match = ["cargotest"]                             # must not match

[[rules]]
tool = "read"
resource = "*.pem"
decision = "deny"
justification = "certificates are secret; ask the user"   # shown to the model
```

If an example fails, Oxgraft refuses to start and names the rule, so a typo
can't silently disable a deny. Some shell commands are asked about every time,
whatever the rules or an earlier "always" answer: downloads piped into a
shell, forced recursive deletes, raw disk writes, `mkfs` and `sudo`.
Workspace rules apply only to trusted workspaces.

Commands the model runs never see environment variables that look like
secrets (names containing KEY, TOKEN, SECRET, PASSWORD or CREDENTIAL). To pass
one on purpose, list it: `OXGRAFT_SHELL_ENV_KEEP=GITHUB_TOKEN`.

## Project instructions and skills

Oxgraft reads `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` and
`.github/copilot-instructions.md` (with `@file` imports), and Agent Skills from
`.agents/skills/`, `.claude/skills/` and the same folders in your home
directory. Existing setups for other harnesses work unchanged.

## Custom commands

Put a prompt in `.agents/commands/<name>.md` (or `.claude/commands/`, the Claude
Code format; `~/` works too) and run it as `/name` in chat or
`oxgraft run /name <args>`. `$ARGUMENTS` in the file is replaced by `<args>`.

```markdown
---
description: Review a file for bugs
---
Review $ARGUMENTS for correctness bugs. List each with file:line.
```

## Profiles

A profile picks one strategy per seam. `oxgraft strategies` lists every
strategy and the harness it was learned from.

| Profile | Edit format | Sandbox | Use |
|---|---|---|---|
| `default` | search_replace | none | everyday use |
| `tolerant` | search_replace_tolerant | none | models that mangle whitespace |
| `codex-like` | apply_patch | none | Codex-family models |
| `sandboxed` | search_replace | workspace_write | unattended runs on Linux/macOS |
| `fewer-prompts` | search_replace | none | read-only shell commands (`ls`, `cat`, `grep`…) run without asking |
| `ci` | search_replace | none (approve all) | inside an outer container |
| `baseline` | whole_file | none | the floor every graft is measured against |
