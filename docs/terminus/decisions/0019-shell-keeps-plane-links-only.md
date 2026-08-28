# ADR-0019: The shell keeps its plane links and refuses its mesh

Status: accepted
Date: 2026-08-27
Requirements: TER-REQ-004, TER-REQ-014, TER-REQ-016
Evidence: `cargo run --release -p terminus-orbits --example feeder_terminals`

## Decision

Each MEO anchor carries **two intra-plane laser links** and **no inter-plane
links at all**. With its six feeder telescopes that is eight per anchor, 192
across the shell, and 480 across the fleet.

Anchor-to-anchor traffic normally descends through the wheel and climbs back
up. The plane links exist for one purpose: to route around a failed feeder
telescope.

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
sessions the busiest such telescope carries 134, and losing it does not degrade
that bucket, it strands it — every session re-anchoring at once, in a migration
storm out of one component.

The two remedies are not the same shape:

| remedy | fleet cost | character |
|---|---:|---|
| a seventh, steerable, cold spare per anchor | +24 | must acquire before it carries |
| **two intra-plane links per anchor** | **+48** | 37,294 km, **0.00 km/s**, pointed once at launch |

The plane links win on more than redundancy: the sessions never move at all, a
severed link becoming a detour rather than a dead end.

Inter-plane links stay out because they are the hardest class in the
architecture. Within a plane, satellites are frozen relative to each other.
Across planes they are not, however identical their periods — two inclined
orbits cross at an angle — and an anchor's nearest partner in another plane
**changes 22 times a day, holding 1.7 h at most, at 4.89 km/s**. Note that
`max_shell_range_rate` reports 0.00 for equal altitudes and is simply
inapplicable across planes; `max_intra_shell_range_rate` is the one to use.

A four-satellite plane closes as a cycle: each anchor sees both plane mates at
90°, and only the 180° diagonal is occulted, giving a diameter of two hops.

## Consequences

- The one link class this sky is worst at is the one the architecture does
  without.
- A plane mate relaying for a neighbour carries **two rings' traffic on one
  telescope**. That capacity is not priced here and is an open debt.
- No arrangement of links saves a session from an anchor that dies outright:
  the working memory dies with the machine, and the vault (ADR-0004) answers
  that, as it always did.
