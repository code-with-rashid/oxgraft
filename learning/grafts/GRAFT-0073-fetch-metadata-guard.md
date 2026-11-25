---
id: GRAFT-0073
title: web_fetch refuses link-local and cloud metadata addresses
kind: avoid
status: trial
seam: tools
donor: codex
donor_version: ""
license: Apache-2.0
origin: clean-room
strategy: ""
opened: 2026-11-25
decided: ""
---

## Observation

Codex keeps network access behind policy and sandbox controls; server-side request forgery to cloud metadata (169.254.169.254) is a classic way for injected instructions to steal credentials.

## Why it matters

In unattended runs (`--yes`, full-auto) a prompt-injected page or a redirect could make the agent fetch instance credentials into its context.

## Hypothesis

No fetch ever reaches a metadata service, while local dev servers stay reachable.

## Implementation

`check_destination` refuses metadata hostnames and hosts resolving to IPv4/IPv6 link-local, unspecified, AWS IPv6 (fd00:ec2::254) or Alibaba (100.100.100.200) metadata addresses; redirects are followed by hand (max 5) and every hop is checked. Loopback and private networks stay allowed. DNS rebinding between check and connect remains a known gap.

## Evaluation

Unit and end-to-end tests cover the behaviour (oxgraft-tools `metadata_and_link_local_destinations_are_blocked`, `redirects_to_metadata_are_refused`). A comparative
evaluation against the `baseline` profile on agentic-arena tasks is pending (ROADMAP M4).

## Decision

Pending maintainer review.

## Revision (2026-11-25, independent review)

The first version parsed the host by hand, differently from the HTTP client:
`http://169.254.169.254\@example.com/`, percent-encoded or tab-split hosts and
numeric IPv4 forms slipped through. Hosts are now parsed with the `url` crate
(the parser ureq uses), and a ureq resolver refuses blocked addresses at
connect time, which also closes the DNS-rebinding gap.
