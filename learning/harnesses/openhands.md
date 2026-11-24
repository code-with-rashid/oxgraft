---
id: openhands
repo: OpenHands/OpenHands
license: MIT
policy: adapt
last_reviewed: "v1.45.0"
status: reviewed
---

# OpenHands

[OpenHands/OpenHands](https://github.com/OpenHands/OpenHands) · MIT · policy `adapt`

> Agent core lives in OpenHands/software-agent-sdk (MIT, Python).

Reviewed at `v1.45.0` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

> Agent core reviewed in OpenHands/software-agent-sdk at v1.64.0 (consumed by app v1.45.0).

Sources: [https://github.com/OpenHands/software-agent-sdk/blob/v1.64.0](https://github.com/OpenHands/software-agent-sdk/blob/v1.64.0) (README, docs, and source where marked Verified).

## Strengths worth grafting

- Risk-threshold approval (ConfirmRisky) plus deterministic policy rails that force HIGH for fetch-to-exec, raw-disk and catastrophic deletes; unparseable shell is UNKNOWN (Verified, security/defense_in_depth/policy_rails.py)
- StuckDetector with five patterns (repeat action+observation, action+error, monologue, alternating, context-window error loop) (Verified, conversation/stuck_detector.py)
- Subagent definitions with per-subagent iteration and budget caps and resumable task ids (Verified, subagent/schema.py)
- file_editor keeps 10 undo versions per file (Verified)
- Secret masking across observations (secret_registry.py)

## Limitations to avoid

- Risk label comes from the model itself (LLMSecurityAnalyzer returns action.security_risk)
- Condensation triggers on event count (240), not tokens
- Hooks cannot return ask (only allow/deny)
- Narrow litellm pin

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v1.45.0 | research agent for the Oxgraft build session | risk-scored approval with shell rails, 5-pattern stuck detector, budgeted subagents; model self-labels risk, event-count condenser |
