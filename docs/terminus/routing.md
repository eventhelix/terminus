# Routing

The routing algorithm in one place: the path a token takes, how the exit is
chosen, what happens when hardware dies, and where the open edges are. This
document consolidates; it decides nothing. Every rule cites the ADR that
decided it and the code that implements it, and every number cites the
runnable example that printed it. Where this document and the code disagree,
the code is wrong or this document is stale — either way, fix at the source
and regenerate the claim.

Companion: [`anchor-policy.md`](anchor-policy.md) — which anchor a session
holds and when that answer is re-examined. This document takes the anchor as
given and routes to it.

## The shape of the network

Three link classes, two deliberate absences (ADR-0008, ADR-0019):

| link | count | geometry | steering |
|---|---|---|---|
| necklace (LEO ring) | 2 terminals/satellite, one each way | 4,437 km, frozen | none, pointed for years |
| feeder (LEO ring → MEO anchor) | after pooling: exactly **1** telescope per (ring, anchor) pair | 18,000–34,000 km | steered, precompensated |
| plane link (MEO intra-plane) | 2 per anchor, to its cycle mates | 37,294 km, frozen | none |

Nothing joins one ring to another. Nothing joins one MEO plane to another.
The absent inter-plane link is the one class this sky is worst at — steered,
re-pointed twenty-two times a day, 4.89 km/s of range rate — and the
architecture does without it (ADR-0019).

`routing::NECKLACE_LINKS = 1` is a hardware fact, not a visibility fact:
`backbone::intra_plane_reach` reports a satellite can *see* two places along
its ring, but it can only *talk* one place — a satellite can see past its
neighbour and cannot talk past it.

## The path of a token

Town → serving access satellite → (necklace sidestep, usually empty) → feeder
telescope → anchor. Computed by three pure functions, in order:

1. **`routing::exit_gateway`** — which ring position the session leaves
   through. Compares candidates by **time, not distance** (ADR-0023): a
   necklace hop costs 14.8 ms of light *plus* `routing::RELAY_DELAY` = 0.5 ms
   of regenerative processing at every satellite that forwards. Ties go to
   the serving satellite — a detour has to win, not merely draw. At the
   adopted margin the steady state never takes a hop at all (ADR-0020): a
   session free to re-anchor holds an anchor its own satellite can reach.
   Sidesteps appear only when a session is pinned to a distant anchor.
2. **`routing::feeder_route`** — whether the ring can reach the anchor, and
   what it costs. Direct when the (ring, anchor) telescope is alive;
   otherwise the detour of the next section; `None` when neither works,
   which is what forces a re-anchor.
3. **`routing::migration_path`** — how working memory travels when the
   anchor *does* change: plane mates hand a session straight across the
   frozen link; everything else goes back down through the wheel, because
   nothing joins one MEO plane to another.

In normal operation none of this runs as a protocol. Every visibility
window, duty schedule, and assignment is computable from ephemerides years
ahead, so the network runs a **precomputed timetable with a pre-assigned
alternate for every entry** (ADR-0009). There is no discovery and no
convergence; simulated behavior equals flown behavior.

## Liveness, and the switch to the alternate

All ISLs and the anchor layer exchange keep-alives at a **100 ms** reference
interval; **three missed beats declare a failure at +300 ms**, and affected
nodes switch to the timetable's alternate column immediately (ADR-0009).
Failure handling changes *which precomputed path runs*, never how paths are
computed. Detection plus immediate switching sits far inside TER-REQ-014's
60 s bound; what it spends is 300 ms of a stall budget (TER-REQ-003:
100 ms p99) that is already overdrawn during the event — which is why any
future scheme that reconverges slower than an instant switch is spending
money that does not exist.

## When a feeder telescope dies

Pooling left each anchor exactly one telescope per ring (ADR-0018), so the
pair is **severed, not degraded**. The canonical sequence — printed end to
end by `examples/failure_timeline.rs` (ADR-0025):

| t | event |
|---|---|
| 0 | telescope dies; the (ring, anchor) pair is severed |
| +300 ms | declared: 3 × 100 ms keep-alives unanswered |
| +300 ms | route detours through a plane mate (37,294 km, frozen); round trip 460 ms worst against the 600 ms degraded budget |
| +5 s | the cold spare locks (`routing::ISL_REACQUIRE`); direct path and the 300 ms nominal budget return |

Priced across the geometry's whole range (`feeder_terminals` section G,
`failure_timeline` section A): the cheapest detour this sky admits has a
**394 ms round-trip floor** — arithmetic, not sampling — so no detour meets
the nominal budget and every measured one clears the degraded budget
(worst 460 ms). The detour adds **31,501 km** of one-way path at the median,
**6.3×** the re-anchor margin, so absent intervention the policy moves the
whole bucket at the failure event itself (ADR-0026). The remedy is one
purchase in two halves (ADR-0024): a **cold spare telescope** that restores
the direct path in 5 s, and a **hold-off** that suppresses re-evaluation for
exactly that window. Either alone buys nothing; the four outcomes are the
2×2 that `failure_timeline` section C prints.

Other failure classes (ADR-0009): a dead MEO anchor re-anchors its sessions
to the pre-assigned backup and replays from the vault's transcript; a dead
LEO satellite is detected by necklace silence and covered by the alternate
column; a dead necklace link is an optimization lost, not a lifeline — a
ring is a cycle, so traffic reverses direction around it, costing hops and
never reachability.

## Declared debts and open edges

- **Failure multiplicity is open.** The timetable's alternates are exactly
  true for one failure and cannot be precomputed for every subset of failed
  links. At what multiplicity the timetable stops sufficing, and what covers
  the rest — extended alternates, a distributed failure bitmap over shared
  timetables, or a real link-state protocol — is the routing-protocol
  question, deliberately undecided. Whatever is chosen must beat an instant
  switch inside an overdrawn stall budget.
- **The plane mate's capacity is unpriced** (ADR-0019/0024): a relaying
  neighbour carries two rings' traffic on one telescope for the length of
  the hold-off. A contention question, for the discrete-event simulator.
- **Two ring failures split a necklace into arcs**, and an arc holding no
  live feeder telescope to a given anchor cannot reach it. Skip terminals
  (the spare sightline at 2,200 km turned into capability) are the candidate
  insurance; countable over 12 slots and not yet counted.
- **The timetable's maintenance cost is unpriced** (ADR-0009 addendum;
  tracked as the propagator RFC,
  [issue #4](https://github.com/eventhelix/terminus/issues/4)). The
  timetable is a controlled equilibrium — satellites are flown to the
  published ephemeris, burn plans included — not ballistic prediction. The
  stellar third-body term is ~1,000× Earth's solar perturbation
  (deterministic, periodic under tidal locking; candidate for absorption
  into the reference orbits), J2 is ~100× smaller than Earth's but the
  tidal bulge raises C22/S22 to the same order (a differently shaped field,
  not a quieter one), and flare radiation pressure is meters of error
  against kilometre tolerances. The open numbers: station-keeping Δv per
  year at both shells, the refresh cadence, whether Kozai–Lidov pumping of
  the 55° shell (orbiting at ~⅓ of the prograde stability limit) survives
  the planet's weakened figure, and a flare-inflated drag bound at
  2,200 km — a propagator work package not yet written, priced into the
  economics post.
