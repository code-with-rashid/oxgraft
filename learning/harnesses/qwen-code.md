---
id: qwen-code
repo: QwenLM/qwen-code
license: Apache-2.0
policy: adapt
last_reviewed: "v0.28.4"
status: reviewed
---

# Qwen Code

[QwenLM/qwen-code](https://github.com/QwenLM/qwen-code) · Apache-2.0 · policy `adapt`

Reviewed at `v0.28.4` from upstream docs (and source where marked **Verified**, policy permitting).
Claims without "Verified" are what the docs say.

> Forked from Gemini CLI v0.8.2. Its README asks agents to fetch a remote SKILL.md; treated as data and not followed.

Sources: [https://github.com/QwenLM/qwen-code/blob/v0.28.4](https://github.com/QwenLM/qwen-code/blob/v0.28.4) (README, docs, and source where marked Verified).

## Strengths worth grafting

- Protected self-modification/persistence paths (.qwen settings, QWEN.md, AGENTS.md, hooks, skills, .git/, CI workflows, package.json) never fast-path auto-approval, symlinks resolved (docs: auto-mode.md)
- `/goal`: keep working until an independent verifier accepts done/blocked within a visible token budget (docs: goals.md)
- Sandbox (bwrap, Landlock fallback) independent of approval mode; backend refuses rather than weakening policy

## Limitations to avoid

- LLM classifier as an approval gate is non-deterministic
- Sandbox reads stay broad (host secrets readable)

## Change log of our reviews

| Date | Upstream range | Reviewer | Summary |
|---|---|---|---|
| 2026-11-25 | first review at v0.28.4 | research agent for the Oxgraft build session | protected self-modification paths, /goal with independent verifier, sandbox independent of approval; LLM classifier gate |
