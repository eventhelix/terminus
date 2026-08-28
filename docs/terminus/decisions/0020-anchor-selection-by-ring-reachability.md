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

**The default margin is 20,000 km of path, and it is an operating parameter
rather than a constant of the sky.** It must be tunable in flight.

At that setting a session, once anchored, never moves again.

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

Which makes migration a purchase rather than a certainty. Sweeping the margin:

| margin (km) | changes/session/day | mean path | worst path | migration, busiest feeder |
|---:|---:|---:|---:|---:|
| 0 | 113.37 | 18,569 km | 20,374 km | 156.5 Gbps |
| 2,500 | 19.10 | 19,086 km | 22,640 km | 26.4 Gbps |
| 5,000 | 12.06 | 20,117 km | 25,205 km | 16.7 Gbps |
| 10,000 | 6.40 | 22,313 km | 30,094 km | 8.8 Gbps |
| **20,000** | **0.00** | 25,843 km | 34,198 km | **0.0 kbps** |

Every row meets TER-REQ-003's 300 ms round trip, the last at 253 ms. The whole
span of mean path length is 24 ms one way, bought with four orders of magnitude
of backbone bandwidth. The trade is so lopsided that the cheap end is the only
defensible one: 20,000 km exceeds the observed spread of path lengths, so no
rival ever clears it.

The necklace detour is not hypothetical. Following one town for a day, a ring
mate offers the shorter path **36% of the time**, saving 5.8 ms one way against
the 14.8 ms a hop costs — because the serving satellite is chosen for its
elevation over a *town*, which has nothing to do with where the anchors are.

## Consequences

- The claim that an anchor "sinks toward the horizon of the region it serves"
  and its sessions "must move" is **false**. Geometry designs that migration
  away; only maintenance and failure still force one.
- The backbone is sized by a policy decision rather than by its users. See
  ADR-0021.
- **A session that never moves never rebalances.** Nothing in this crate models
  anchor compute, so nothing notices sessions piling onto whichever anchor was
  nearest when they began. This is the reason the margin must be a parameter,
  and the reason to expect it to come down in service.
- `select_anchor` is now pure — `(anchors, path_cost, current, margin)` — and
  knows nothing about orbits, so the policy is testable without propagating
  anything.
