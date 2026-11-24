---
id: swe-agent
repo: SWE-agent/SWE-agent
license: MIT
policy: adapt
last_reviewed: "v1.1.0"
status: reviewed
---

# SWE-agent

[SWE-agent/SWE-agent](https://github.com/SWE-agent/SWE-agent) · MIT · policy `adapt`

Reviewed at `v1.1.0` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

Sources: [https://github.com/SWE-agent/SWE-agent/blob/v1.1.0](https://github.com/SWE-agent/SWE-agent/blob/v1.1.0) (README, docs, and source where marked Verified).

## Strengths worth grafting

- Format/blocklist/bash-syntax errors re-query the model up to 3 times without polluting history (Verified, agents.py L1020-1110)
- Review on submit: first submit shows the diff and a checklist, second submit confirms (config/default.yaml)
- Autosubmit on fatal error extracts the patch (Verified)
- Lint-gated windowed edits that auto-revert when they add errors (Verified)
- Timeout escalation: 3 consecutive timeouts end the run (Verified)
- Replayable trajectories

## Limitations to avoid

- Command blocking by prefix: `env vim` / `bash -c vim` bypass, `make` over-blocks
- Instance template hard-codes Python/SWE-bench wording
- No interactive approval or local sandbox

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v1.1.0 | research agent for the Oxgraft build session | re-query outside history, review-on-submit, lint-gated edits; prefix blocklists, Docker assumed |
