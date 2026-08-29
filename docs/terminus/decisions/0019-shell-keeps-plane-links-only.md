# ADR-0019: The shell keeps its plane links and refuses its mesh

Status: accepted
Date: 2026-08-27 (amended 2026-08-28)
Requirements: TER-REQ-003, TER-REQ-004, TER-REQ-014, TER-REQ-016
Evidence: `cargo run --release -p terminus-orbits --example feeder_terminals` (sections F and G)

> **Amended 2026-08-28.** The decision stands unchanged — the plane links stay,
> the inter-plane mesh stays out. What was wrong was the reason. This ADR
> asserted that with plane links "the sessions never move at all"; measuring the
> detour shows one moves anyway, by 6.3x the re-anchor margin. The plane link
> does not prevent the migration. It keeps the session *served* while the
> migration happens, and that is a different purchase at a different price. The
> "Why" and "Consequences" below are rewritten to the measured numbers, and one
> stale load figure is corrected: the busiest (ring, anchor) telescope carries
> **113** of 1,000 sessions, not the 134 this ADR quoted from before the
> necklace hop correction (ADR-0020). The reasoning that follows is the
> measured version; nothing about the hardware chosen has changed.

## Decision

Each MEO anchor carries **two intra-plane laser links** and **no inter-plane
links at all**. With its six feeder telescopes (ADR-0018) that is eight per
anchor, 192 across the shell, and 480 across the fleet.

This ADR decides the *link classes*, not the final telescope count: ADR-0024
adds a seventh feeder telescope as a cold spare, taking an anchor to nine, the
shell to 216 and the fleet to 504.

Anchor-to-anchor traffic normally descends through the wheel and climbs back
up. The plane links exist for one purpose: to keep a session reachable at its
own anchor when the feeder telescope it was using goes dark.

## Why

**The shell needs no links to be reachable.** Across 13,248 anchor pairs
sampled through a day, every pair had an access satellite that could see both
ends, and the worst-served pair still had 22 candidates. The wheel is the
shell's control segment, and a migration is two feeder hops through a single
access satellite — which is what its second feeder telescope is for.

The GPS precedent is often cited here and needs care. Classic GPS flew without
crosslinks because a *ground segment* did the ephemeris and clock work, and
later blocks added crosslinks anyway for autonomy when that segment is out of
reach. This system has no ground segment at all; the wheel stands in for one.

**But reachable is not resilient.** Pooling (ADR-0018) reduces an anchor to one
telescope per ring, so all 144 (ring, anchor) pairs are singly loaded. Of 1,000
sessions the busiest such telescope carries **113**, and the busiest anchor
holds **170** across all six of its links. Losing one telescope does not degrade
that bucket, it strands it — every session in it re-anchoring at once, in a
migration storm out of one component.

**What the detour costs.** Walking every (ring, anchor) pair from ring slot 0
at 24 hourly instants — the geometry's whole range, not the anchors the policy
actually holds:

| | |
|---|---:|
| pairs walked | 144 distinct, 3,456 samples |
| extra round trip | median **210 ms**, worst **315 ms** |
| worst round trip | **460 ms** |
| over the 300 ms nominal budget | **3,456 of 3,456 samples (100%)** |
| over the 600 ms degraded budget | **0 of 3,456** |
| samples with no plane mate | 0 |

The floor is stronger than the samples. The cheapest detour this geometry
admits — the radio leg at the footprint edge, the shortest possible feeder
straight up, the plane link itself, and two relays, summed one way and doubled
— comes to **394 ms of round trip**. No detour here can meet the 300 ms
budget, and that is arithmetic rather than sampling.

**Which is why TER-REQ-003 was amended.** Its 300 ms to the first token now
applies to failure-free operation, and a **600 ms degraded budget** prices the
failure case. Without that amendment the detour is permanently out of spec and
these two telescopes per anchor could not be justified at all. With it, every
detour measured is compliant with room to spare — 460 ms worst against 600 —
and the cure is priced rather than asserted. The 300 ms budget comes back only
when the direct path does (ADR-0024).

**What the plane link buys is not stillness.** The detour adds **31,501 km** of
one-way path at the median, against ADR-0020's **5,000 km** re-anchor margin —
**6.3x the margin**. A detoured session is beaten by any rival anchor several
times over, so the policy moves it at its next evaluation. The migration
happens either way.

What changes is whether the session is still being served while it moves. On
the plane link it keeps talking to its own anchor at the degraded budget for as
long as the move takes, which is exactly the overlap make-before-break needs:
answering from the old anchor while the working memory streams to the new one
(ADR-0022). Without the plane link the whole ring loses that anchor at once,
and all 113 sessions on the busiest pair break before anything is made. The
plane link turns a stranding into a migration the session is served through.
It does not turn it into stillness.

Inter-plane links stay out because they are the hardest class in the
architecture. Within a plane, satellites are frozen relative to each other.
Across planes they are not, however identical their periods — two inclined
orbits cross at an angle — and an anchor's nearest partner in another plane
**changes 22 times a day, holding 1.7 h at most, at 4.89 km/s**. Note that
`max_shell_range_rate` reports 0.00 for equal altitudes and is simply
inapplicable across planes; `max_intra_shell_range_rate` is the one to use.

A four-satellite plane closes as a cycle: each anchor sees both plane mates at
90°, and only the 180° diagonal is occulted, giving a diameter of two hops.
Either neighbour will do.

The measurement also settles what this ADR originally got wrong about the
alternative. The two remedies are not competitors:

| remedy | fleet cost | what it does |
|---|---:|---|
| **two intra-plane links per anchor** | **+48** | 37,294 km, **0.00 km/s**, pointed once at launch; carries the bucket through the failure at the degraded budget |
| a seventh, steerable, cold spare (ADR-0024) | +24 | must acquire before it carries; ends the failure and gives back the nominal budget |

The plane link covers the acquisition window and the spare ends it. Neither
alone closes the gap between what a lost telescope breaks and what a session
needs, which is why both are bought.

## Consequences

- The one link class this sky is worst at is the one the architecture does
  without.
- **The plane link cannot restore the nominal budget**, at any hour, over any
  pair. It buys reachability at 600 ms, not latency at 300 ms, and the
  proposal must not claim otherwise.
- **The justification depends on the amended TER-REQ-003.** If the degraded
  budget is withdrawn, the detour is non-compliant everywhere and this decision
  loses its reason even though its hardware would still be the cheapest cure.
  The requirements baseline and the compliance matrix have to carry the
  amendment.
- A plane mate relaying for a neighbour carries **two rings' traffic on one
  telescope**. That capacity is not priced here and is an open debt; ADR-0024
  bounds how long it must be carried but raises the peak, because a held-off
  bucket does not drain.
- The working memory is never what the plane link protects. The anchor is
  alive; only its view of one ring is gone. What is at risk is the session's
  continuity of service across the window.
- No arrangement of links saves a session from an anchor that dies outright:
  the working memory dies with the machine, and the vault (ADR-0004) answers
  that, as it always did.
