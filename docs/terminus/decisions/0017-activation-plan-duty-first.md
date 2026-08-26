# ADR-0017: Activation plan — light the duty ring, then patch the holes

Status: accepted
Date: 2026-08-25
Requirements: TER-REQ-001, TER-REQ-014, TER-REQ-016
Evidence: `cargo run --release -p helixsim-orbits --example activation_plan` (tag: terminus-post-5b)

## Decision

Satellites are **not all transmitting all the time**. At each instant the fleet
runs a precomputed *activation plan*: the duty ring is switched on as a block,
and satellites from other rings are added only where the band would otherwise
be unserved. The plan is computed on the ground as a timetable (ADR-0009) and
uploaded, not negotiated in orbit.

Satellites are lit a **warm-up lead** ahead of first service so a dark
spacecraft is never asked to take a handover cold.

## Why

Being visible is not the same as being needed. ADR-0016 recorded that polar
rings converge at the poles, so a high-latitude town can have all six rings
overhead — but it needs no more service than a town on the equator. Counting
visible satellites therefore overstates what the fleet must operate.

Choosing the smallest serving set at each instant is a minimum set cover.
Over a full rotation sampled every 60 s (72 azimuths × 3 band offsets):

| policy | mean lit | max lit | duty cycle | switches/h | coverage gaps |
|---|---:|---:|---:|---:|---|
| all on | 72.0 | 72 | 100% | 0 | none |
| duty ring only | 12.0 | 12 | 17% | 0 | **every instant** |
| duty first | 24.2 | 32 | 34% | 204 | none |
| duty first + prune | 23.1 | 30 | 32% | 253 | none |
| greedy set cover | 22.4 | 30 | 31% | 373 | none |
| **exact minimum** | **21.4** | **28** | **30%** | 423 | none |

The exact branch-and-bound closed at all 16,128 sampled instants, so the 21.4
figure is a proved optimum and not a heuristic's guess. Roughly **70% of the
fleet can be dark at any moment** while the band stays fully served.

**Duty-first is chosen over the exact minimum** despite costing ~2.8 more
satellites lit on average, because:

- It is explainable. An operator, and a reader, can state the rule in one
  sentence, and the duty ring stays the organising idea of the architecture.
- It switches **half as often** (204/h against 423/h). Every switch is a
  thermal and power-electronics cycle, and the exact plan's freedom to
  reshuffle its whole set each step is what buys its last two satellites.
- The advantage evaporates under warm-up anyway (below).

Charging each policy for lighting satellites ahead of service:

| policy | no lead | 2 min | 5 min | 10 min |
|---|---:|---:|---:|---:|
| duty first + prune | 23.1 | 26.7 | 30.4 | 34.9 |
| duty first | 24.2 | 27.2 | 30.5 | 34.9 |

At any realistic warm-up the pruned and unpruned plans converge, so the
**simpler unpruned policy is adopted**. Even at a ten-minute lead the fleet
runs under half lit.

## Consequences

- **Power, thermal and interference budgets size to ~35% mean and ~45% peak
  duty cycle**, not 100%. Peak lit is 32 of 72 before warm-up.
- **The warm-up lead is a protocol input**, not an operations detail: handover
  selection (ADR-0015) may only choose a target already lit, so the activation
  timetable and the handover plan must be generated together.
- **A hysteresis (lazy-off) hold is required, and is nearly free.** Planning
  each step independently leaves satellites flapping: 787 one-step on/off
  blinks per simulated day, each a wasted thermal and power cycle. Holding a
  satellite lit until it has gone 120 s without being needed removes every one
  of them for 1.6 extra satellites lit (34% → 36% duty cycle) and cuts
  switching from 196/h to 153/h. The curve knees hard there — 600 s of hold
  would buy 100/h but cost ten extra satellites — so the hold is set just above
  the knee, at 180 s. Implemented as `activation::smooth_schedule`.
- A dark satellite is still in its orbit and still tracked; "off" means its
  service payload is not radiating, not that it is unavailable for the plan.
- **This does not reduce the fleet.** All 72 spacecraft are still required —
  which ones are needed changes continuously, and the set that is dark now is
  serving in twenty minutes. Duty cycle is an operating economy, not a
  constellation-sizing one.
- `duty_ring_only` is recorded as refuted, not deferred: it leaves the band
  uncovered at every instant sampled (ADR-0016).
