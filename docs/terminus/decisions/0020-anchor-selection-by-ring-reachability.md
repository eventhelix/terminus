# ADR-0020: Anchor selection answers to the ring, and migration is a policy

Status: accepted
Date: 2026-08-27
Requirements: TER-REQ-003, TER-REQ-013, TER-REQ-016
Evidence: `cargo run --release -p terminus-orbits --example feeder_terminals` (section H); `--example link_throughput` (section C)

## Decision

A session's anchor is chosen by the **shortest path the whole ring can offer**,
not by what the serving access satellite can see. Selection prefers the
smallest total of necklace hops plus feeder link (`routing::exit_gateway`), and
holds the current anchor unless a rival beats it by `REANCHOR_MARGIN`.

**The default margin is 5,000 km of path, and it is an operating parameter
rather than a constant of the sky.** It must be tunable in flight.

At that setting a session changes anchor **12.70 times a day** — about once
every 113 minutes, which is roughly ten access handovers — and the backbone
carries its working memory each time.

## Why

The earlier policy filtered candidates by separation from the serving satellite
(≤118°, the LEO–MEO limb) and held an anchor while it stayed visible *from that
satellite*. Once a session can leave its ring through any ring mate (ADR-0018),
that is the wrong horizon, and the error is not small:

```
57,600 (ring, anchor, instant) checks over a rotation
  rings with no satellite able to see the anchor:   0
  fewest ring satellites seeing an anchor:          7 of 12
```

Every ring sees every anchor, always. So **reachability stopped being the
binding constraint**, and the dwell tiebreak went with it — dwell only ever
predicted when reachability would expire. What binds instead is latency.

Which makes migration a purchase rather than a certainty. Sweeping the margin,
with the round trip measured end to end — the town's radio leg, the backbone
path, and a relay at every satellite that forwards (ADR-0023):

| margin (km) | changes/day | mean path | p95 path | worst path | p95 round trip | thinking time left | migration, busiest feeder |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 113.37 | 18,569 km | 19,807 km | 20,374 km | 157 ms | 143 ms | 156.5 Gbps |
| 2,500 | 19.13 | 19,084 km | 20,799 km | 22,640 km | 164 ms | 136 ms | 26.4 Gbps |
| **5,000** | **12.70** | **20,075 km** | **22,925 km** | **25,205 km** | **178 ms** | **122 ms** | **17.5 Gbps** |
| 10,000 | 7.08 | 21,915 km | 27,337 km | 30,279 km | 208 ms | 92 ms | 9.8 Gbps |
| 20,000 | 4.05 | 25,494 km | 35,471 km | 39,714 km | 263 ms | 37 ms | 5.6 Gbps |
| 25,000 | 0.00 | 28,385 km | 39,332 km | 41,036 km | 290 ms | 10 ms | none |

"Thinking time left" is TER-REQ-003's 300 ms to the first token, less the p95
round trip. It is what the model has to produce that token in, and it is the
column that settles this decision.

**Neither end of the curve can be bought.** Chasing the shortest path costs
more backbone than a 100 Gbps feeder link can carry. Holding an anchor until
nothing ever beats it — the zero-migration row — takes a 25,000 km margin and
leaves 10 ms to think in, which is not a budget. Once latency is counted
honestly the trade is lopsided in neither direction, and 5,000 km is where both
currencies are still affordable.

This reverses the earlier reading of the same sweep. That reading was taken
before the necklace hop correction, which found routing counting hops against
what a satellite can *see* — two ring mates, at 2,200 km — rather than what it
has a terminal aimed at, which is one (ADR-0018). Fixing it doubled the price
of a hop, lengthened every held path, and moved the whole curve. The 20,000 km
margin had been adopted because it produced *zero* migrations at a 253 ms worst
case; it produces four a day and leaves 37 ms.

The same correction overturns the claim that a ring mate offers the shorter
path **36% of the time**. At twice the price a detour rarely pays, and at the
adopted margin the routing **never takes a necklace hop at all** across a day
of following a thousand towns: a session free to re-anchor holds an anchor its
own satellite can already see. Hops appear only at 20,000 km and beyond, where
a session is pinned to a distant anchor and a ring mate's geometry finally
wins. The necklace still earns its keep — on pooled feeder terminals, on reach
where the radios are dark, and on surviving a lost telescope (section G) — but
not on steady-state routing at this margin.

## Consequences

- The claim that an anchor "sinks toward the horizon of the region it serves"
  and its sessions "must move" is **false**. Geometry designs that migration
  away; what puts it back is this policy, deliberately.
- The backbone is sized by a policy decision rather than by its users.
- **A session moves in steady state, so context transfer has a customer.**
  ADR-0021 deferred streaming working memory on the premise that nothing ever
  moved. That premise is gone, and ADR-0022 reverses the deferral.
- Load balancing is still not bought. A session moves to shorten its own path
  and never to spare an anchor's compute, so nothing here notices sessions
  piling onto whichever anchor sits over a crowded stretch of the band. The
  margin is the only lever and it moves every session at once, which remains
  the reason it must be a parameter.
- Anchor retention is now a policy number rather than a geometric one: 113
  minutes at this margin, not the ~19 access handovers of a MEO pass. That
  figure describes visibility and never described a session's fate.
- `select_anchor` is pure — `(anchors, path_cost, current, margin)` — and knows
  nothing about orbits, so the policy is testable without propagating anything.
  It still argues in metres of path: relays are a thirtieth of what a hop
  costs, so they decide which door a session leaves by, not which anchor it
  holds.
