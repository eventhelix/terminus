# Anchor policy

Which anchor a session holds, and when that answer is re-examined — in one
place. This document consolidates; it decides nothing. Every rule cites its
ADR and its code; every number cites the example that printed it.

Companion: [`routing.md`](routing.md) — how a token reaches the anchor this
policy chose.

## The rule

`backbone::select_anchor(anchors, path_cost, current, margin)` — pure, and
deliberately ignorant of orbits:

- **Candidates are what the whole ring can reach**, not what the serving
  satellite can see (ADR-0020). Every ring sees every anchor at every
  instant — measured over a full rotation, zero rings ever lack a sightline
  (`feeder_terminals`) — so reachability is never the binding constraint.
  Latency is.
- **The session holds its current anchor unless a rival beats it by the
  margin.** Cost is metres of one-way path, hops priced at what they cost in
  time's currency (relays are a thirtieth of a hop — they decide which door a
  session leaves by, never which anchor it holds).
- **`backbone::REANCHOR_MARGIN` = 5,000 km, and it is an operating
  parameter, not a constant of the sky.** It must stay tunable in flight.

## Why 5,000 km

Section H's sweep (`feeder_terminals`), round trips measured end to end —
radio leg, backbone path, a relay at every forwarding satellite (ADR-0023):

| margin (km) | changes/day | p95 round trip | thinking time left | migration, busiest feeder |
|---:|---:|---:|---:|---:|
| 0 | 113.37 | 157 ms | 143 ms | 156.5 Gbps |
| 2,500 | 19.13 | 164 ms | 136 ms | 26.4 Gbps |
| **5,000** | **12.70** | **178 ms** | **122 ms** | **17.5 Gbps** |
| 10,000 | 7.08 | 208 ms | 92 ms | 9.8 Gbps |
| 20,000 | 4.05 | 263 ms | 37 ms | 5.6 Gbps |
| 25,000 | 0.00 | 290 ms | 10 ms | none |

Neither end can be bought: chasing the shortest path costs more backbone
than a 100 Gbps feeder link carries; never moving takes a 25,000 km margin
and leaves 10 ms to produce a first token in. At 5,000 km both currencies
are affordable, and a session moves **12.70 times a day** — about every 113
minutes (ADR-0020).

## When the policy runs (ADR-0026)

**Never on a poll.** Two kinds of instant only:

1. **Scheduled** — the crossing instants at which a rival first beats the
   held anchor by the margin are computable years ahead, so they are
   timetable entries like every other handover (ADR-0009). The 12.70/day
   *are* that schedule, counted. Retuning the margin in flight recomputes
   the schedule and ships it as a timetable update.
2. **Event-triggered** — a keep-alive declaration (+300 ms), a spare
   locking, a hold-off expiring. Evaluation runs at the event itself, which
   is what decides the failure race: a detoured session is 6.3× over the
   margin and leaves at +300 ms unless held.

**The hold-off** (ADR-0024) is precisely a suppression of event-triggered
evaluation while the session's anchor is reconfiguring a feeder telescope —
the `routing::ISL_REACQUIRE` = 5 s acquisition window. Scheduled entries
falling inside the window defer to its end. The spare is bought on the
condition that the policy waits for it; take either half away and the other
buys nothing.

## When the session does move

Make-before-break context transfer ships in the first release (ADR-0022,
reversing ADR-0021), because every one of the 12.70 daily migrations happens
inside a live conversation: 10.7 GB of working memory in 0.86 s on a
100 Gbps link, across the frozen plane link when the anchors share a plane
and down through the wheel otherwise (`routing::migration_path`). An anchor
that *dies* is the one case transfer cannot serve; the vault's transcript
replica answers that instead (ADR-0004, ADR-0009).

## What this policy does not do

- **Load balancing.** A session moves to shorten its own path, never to
  spare an anchor's compute; nothing here notices sessions piling onto the
  anchor over a crowded stretch of the band. The margin is the only lever,
  it moves every session at once, and that is why it must remain a
  parameter (ADR-0020).
- **Steady-state necklace hops.** At the adopted margin the routing never
  takes one — a session free to re-anchor holds an anchor its own satellite
  reaches. Hops appear from 20,000 km up, where a pinned session finally
  makes a ring mate's geometry pay (ADR-0020).
