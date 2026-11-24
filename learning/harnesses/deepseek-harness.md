---
id: deepseek-harness
repo: deepseek-ai/deepseek-harness
license: MIT
policy: adapt
last_reviewed: "dsh-v0.2.3-alpha.2"
status: reviewed
---

# DeepSeek Harness

[deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness) · MIT · policy `adapt`

Reviewed at `dsh-v0.2.3-alpha.2` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

Sources: [https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.2.3-alpha.2](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.2.3-alpha.2) (README, docs, and source where marked Verified).

## Strengths worth grafting

- One append-only SessionEvent log from which context, fork, resume and telemetry derive (docs/architecture.md)
- Fork at a turn boundary by event count
- Sandbox seam wraps every spawn path (bash, PTY, LSP) together

## Limitations to avoid

- SAFETY.md: no security audit; sandbox and approval guarantee nothing
- Very large seam surface (capability-seams doc ~79 KB)

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at dsh-v0.2.3-alpha.2 | research agent for the Oxgraft build session | single append-only event log, fork by event count, sandbox seam over all spawns; unaudited, very large surface |
