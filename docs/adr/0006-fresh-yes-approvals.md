---
status: accepted
date: 2026-11-25
---

# ADR 0006: Actions that need a fresh yes every time

## Context

An "always" answer in chat approves a tool for the rest of the session. That is
convenient for `bash` until the model runs `curl … | sh` or `rm -rf` under the
same blanket approval. OpenHands forces such commands to high risk with
deterministic rails, whatever the model or the user said earlier.

## Decision

Add a default method to the approval seam:

```rust
fn needs_fresh_yes(&self, action: &Action) -> Option<String> { None }
```

When `decide` returns `Ask` and this returns a reason, core asks again even if
the user answered "always" for the tool, and reports the reason in
`ApprovalRequested.risk` (a new optional field; protocol stays at version 1).
A policy that returns `Allow` outright (such as `allow_all` inside a sandbox) is
not overridden.

## Consequences

- Existing policies compile unchanged (default method).
- `ordered_rules` uses the shell risk rails (GRAFT-0042) for this.
