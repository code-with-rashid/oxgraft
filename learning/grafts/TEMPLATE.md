---
id: GRAFT-NNNN
title: Short imperative title
kind: adopt            # adopt (bring in a strength) | avoid (fix a donor's limitation)
status: proposed       # proposed | trial | evaluated | adopted | available | rejected
seam: edit_format      # edit_format | context | approval | sandbox | (new seam → ADR first)
donor: aider           # id from learning/watch.toml
donor_version: ""      # release tag or commit the observation is based on
license: Apache-2.0
origin: clean-room     # adapted | clean-room   (must match watch.toml policy)
strategy: ""           # strategy id once implemented
opened: YYYY-MM-DD
decided: ""
---

## Observation

What the donor does, with links to its docs, code, or changelog at `donor_version`.

## Why it matters

The problem it solves (or, for `avoid`, the failure it causes), and for whom.

## Hypothesis

What should improve, measured how. Example: "fewer failed edits on files over
500 lines, at no more than 10% extra tokens".

## Implementation

Strategy id, files, and anything the seam could not express (if so: ADR link).

## Evaluation

| Profile | Tasks | Pass rate | Tokens | Wall time | Notes |
|---|---|---|---|---|---|
| baseline | | | | | |
| default (before) | | | | | |
| with this graft | | | | | |

Commands used, so anyone can rerun it.

## Decision

Adopted / available / rejected, by whom, and why.
