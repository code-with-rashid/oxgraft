---
id: GRAFT-0051
title: Untrusted workspaces cannot choose the profile
kind: adopt
status: trial
seam: approval
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex (rust-v0.172.0) ignores a project's config layer until the folder is trusted. GRAFT-0034 applied that to `model`; a self-review on 2026-11-25 found `profile` and the workspace's own `profiles/*.toml` were still honoured.

## Why it matters

A profile selects the approval policy: a cloned repo could set `profile = "ci"` or ship `profiles/default.toml` with `allow_all` and run any command without asking.

## Hypothesis

Opening an untrusted repository can never weaken approvals.

## Implementation

In untrusted workspaces `profile` from `.oxgraft/config.toml` is dropped with a warning and `profiles::load` skips `<workspace>/profiles/` (built-ins only); the user's `--profile` / `OXGRAFT_PROFILE` still win.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-cli `untrusted_workspaces_cannot_choose_the_profile`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.
