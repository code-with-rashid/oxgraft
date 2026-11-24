---
id: mini-swe-agent
repo: SWE-agent/mini-swe-agent
license: MIT
policy: adapt
last_reviewed: "v2.4.6"
status: reviewed
---

# mini-SWE-agent

[SWE-agent/mini-swe-agent](https://github.com/SWE-agent/mini-swe-agent) · MIT · policy `adapt`

Reviewed at `v2.4.6` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

Sources: [https://github.com/SWE-agent/mini-swe-agent/blob/v2.4.6](https://github.com/SWE-agent/mini-swe-agent/blob/v2.4.6) (README, docs, and source where marked Verified).

## Strengths worth grafting

- Head 5k + tail 5k truncation with elided-char count (Verified, config/mini.yaml)
- Approval modes human/confirm/yolo switchable mid-run, confirm-on-exit (Verified, interactive.py)
- Specific recovery prompt on length stop without tool call
- Process-group kill on timeout; swappable environments incl. bubblewrap (Verified)
- Crash-safe trajectory save in finally

## Limitations to avoid

- No edit tool (sed/heredocs only)
- No compaction: long tasks exceed the window
- Finish signalled by a magic first-line string that command output could spoof
- cd/exports don't persist between actions

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v2.4.6 | research agent for the Oxgraft build session | bash-only minimal loop, head+tail truncation, mid-session approval modes; no compaction, spoofable finish string |
