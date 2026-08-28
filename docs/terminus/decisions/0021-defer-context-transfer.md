# ADR-0021: Context transfer is deferred out of the first release

Status: accepted
Date: 2026-08-27
Requirements: TER-REQ-003, TER-REQ-013, TER-REQ-014
Evidence: `cargo run --release -p terminus-orbits --example link_throughput` (sections C and D)

## Decision

The **first release does not build streaming context transfer.** Moving a
session's working memory from one anchor to another — make-before-break, 10.7 GB
in 0.86 s on a 100 Gbps link — is deferred to a later block.

A session whose anchor fails resumes by **replaying its transcript from the
vault** (ADR-0004), which the architecture already commits to and which bounds
the loss at the exchange in flight. Planned maintenance uses the same path.

## Why

The feature has no steady-state customer at the adopted anchor policy. Under
ADR-0020's 20,000 km margin a session, once anchored, never moves: nothing
forces it to, because a ring reaches every anchor at every instant, and the
margin exceeds the whole observed spread of path lengths.

So the sessions that must move are exactly the ones whose anchor failed — and
those cannot be helped by context transfer anyway. The working memory dies with
the machine; the transcript in the vault is what survives. Building a streaming
path would serve only *planned* migrations, of which the first release has
none.

The cost of not having it is a **prefill rather than a conversation**: a
recovering session recomputes its KV cache from the transcript instead of
receiving it. That is compute time on the destination anchor, not link time,
and it falls within the loss bound TER-REQ-014 already accepts.

This also removes the largest single item from the backbone's traffic budget.
At the million-terminal ceiling the busiest feeder link carries **2.1 Mbps of
conversation and no working memory at all**, against 156.5 Gbps at the
migration-chasing end of the policy range.

## Consequences

- The first release ships with: anchor selection, the vault, transcript replay.
  It does not ship: KV-cache streaming, make-before-break overlap, or the
  bandwidth reservation either would need.
- Consistent with the compliance matrix, which already lists make-before-break
  machinery under Series 2 for TER-REQ-013. This ADR supplies the reason and
  makes the deferral deliberate rather than incidental.
- **The deferral is contingent on the margin.** If anchor compute load forces
  the margin down — the debt ADR-0020 records — planned migrations reappear,
  and context transfer returns with them. The feature is deferred, not
  cancelled, and the trigger to revisit it is a change to that parameter.
- Recovery latency after an anchor failure becomes a prefill-bound number that
  nothing has yet measured. That is the open item this decision creates.
