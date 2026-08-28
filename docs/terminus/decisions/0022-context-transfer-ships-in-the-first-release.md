# ADR-0022: Context transfer ships in the first release

Status: accepted
Date: 2026-08-27
Requirements: TER-REQ-003, TER-REQ-013, TER-REQ-014
Evidence: `cargo run --release -p terminus-orbits --example link_throughput` (sections C and D); `--example feeder_terminals` (section H)

Replaces ADR-0021, which deferred this feature.

## Decision

The **first release builds streaming context transfer.** Moving a session's
working memory from one anchor to another — make-before-break, 10.7 GB in
0.86 s on a 100 Gbps link — is first-release work, not a later block.

A session whose anchor *fails* still resumes by replaying its transcript from
the vault (ADR-0004). That path is unchanged and remains the answer to a dead
machine. What it is no longer asked to be is the answer to a *planned* move.

## Why

ADR-0021 deferred the feature on one premise: at the adopted anchor policy a
session never moves, so streaming working memory has no steady-state customer.
The premise held only at a 20,000 km re-anchor margin, and that margin did not
survive.

The necklace hop correction found routing counting hops against what a
satellite can *see* rather than what it has a laser terminal aimed at. A hop
moves one place, not two, so hops cost double, held paths lengthen, and the
whole margin sweep moves. At 20,000 km a session no longer sits still — it
moves four times a day — and the setting that does produce zero migrations,
25,000 km, leaves 10 ms of the 300 ms first-token budget to think in. ADR-0020
therefore adopts 5,000 km, and at 5,000 km:

| | |
|---|---|
| anchor changes per session | **12.70 a day**, one every 113 minutes |
| at the million-terminal ceiling | **better than 1.2 million migrations a day** |
| working memory per migration | 10.7 GB, 0.86 s on a 100 Gbps link |
| busiest feeder link carries | **17.5 Gbps** of working memory, against 2.1 Mbps of conversation |

Every one of those migrations happens inside a live conversation. That is the
steady-state customer ADR-0021 could not find.

Transcript replay cannot serve it. Replay answers a failed anchor because there
is nothing left to stream from — the working memory died with the machine. A
planned move has a live source, and paying a full prefill instead of copying
from it costs the user a stall in the middle of a sentence, twelve times a day.
The loss bound in TER-REQ-014 covers a failure; it was never meant to cover
routine policy.

The bandwidth this commits to is not new spending. It is already in the sizing:
17.5 Gbps on the busiest feeder link is what the adopted margin costs whether
or not the transfer is graceful. Building the streaming path decides whether
that traffic is a make-before-break overlap or a stall followed by a prefill.

## Consequences

- The first release ships anchor selection, the vault, transcript replay, **and**
  KV-cache streaming with make-before-break overlap and the bandwidth
  reservation it needs.
- The compliance matrix lists make-before-break machinery under Series 2 for
  TER-REQ-013. **That needs to move to Series 1**, and this ADR is the reason.
- Feeder links are sized by the migration burst either way — 10.7 GB in 0.86 s,
  and a failed telescope strands a whole bucket of sessions at once. What
  changes is that the steady-state mean is no longer zero, so the link carries
  17.5 Gbps between bursts rather than 2.1 Mbps.
- The margin remains tunable in flight, and this decision is not contingent on
  its value the way ADR-0021 was. Moving the margin up reduces how often the
  feature is used; it does not remove the need for it, because the only setting
  that removes migrations entirely costs more thinking time than the RFP has.
- Recovery latency after an anchor *failure* is still a prefill-bound number
  that nothing has measured. ADR-0021 opened that item and it stays open.
