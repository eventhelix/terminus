# ADR-0016: The duty ring is a scheduling idea, not a coverage mechanism

Status: accepted
Date: 2026-08-25
Requirements: TER-REQ-001, TER-REQ-014, TER-REQ-016
Evidence: `cargo run --release -p helixsim-orbits --example duty_ring_trade`; `--example phasing_options` (tag: terminus-post-5b)

## Decision

Coverage of the inhabited band is a property of the **whole wheel**, not of
whichever ring is nearest the terminator. The "duty ring" is retained as an
operational and narrative device — it names the ring carrying the most traffic
and it changes on a predictable 22.4-hour beat — but no coverage, handover, or
capacity claim may assume the duty ring is the only ring serving.

Two further constraints follow and are adopted as standing rules:

1. **No coverage claim may depend on a phase relationship between rings.**
   Each ring's along-orbit phase is treated as arbitrary and uncoordinated.
   The flip side is that the baseline phasing must still be *verified* rather
   than assumed harmless: the half-slot alternation is excluded because it is
   measurably worse, not because it is unfashionable.
2. Handover cadence is quoted as the in-ring spacing (11.0 min at 2,200 km)
   only as the *nominal* rhythm; the serving satellite may belong to a
   neighbouring ring, so real cadence is that figure or shorter.

The baseline of ADR-0003 is unchanged: 6 rings × 12 at 2,200 km.

## Why

**A strict duty-ring design is geometrically impossible on this shelf, at any
satellite count.** The band reaches 20° either side of the terminator and the
duty ring is up to 15° misaligned at handover (half of the 30° plane spacing),
so a town can sit 35° cross-track of the duty ring's ground track. A satellite
at 2,200 km with a 25° mask has a footprint half-angle of only 22.65°. That is
a *reach* failure, not a spacing failure, so adding satellites to the ring
cannot fix it — simulation confirms minimum visible in the duty ring stays 0 at
12, 24, and 48 satellites per ring (72, 144, 288 total).

Rescuing a strict duty ring requires moving the footprint past 35°:

| option | altitude | mask | satellites | min (duty only) | edge one-way |
|---|---|---|---|---|---|
| baseline, for reference | 2,200 km | 25° | 72 | **0** | 12.1 ms |
| climb | 6,000 km | 25° | 84 | 1 | 27.5 ms |
| climb | 7,300 km | 25° | 54 | 1 | 32.4 ms |
| drop the mask | 2,200 km | 5° | 84 | 1 | 17.4 ms |
| more rings | 2,200 km | 25° | ~1,440 | 1 | 12.1 ms |

Every rescue costs more than it buys. The 7,300 km option nearly triples edge
latency; the 5° mask surrenders the rain, cloud, and terrain margin the
inhabited band's weather (world bible: steady dayward wind, persistent cloud)
makes essential, and still costs 17.4 ms because the slant range at 5°
elevation is far longer.

**The multi-ring design already has the one property a strict duty ring was
wanted for.** The reason to prefer a single ring was to avoid depending on
inter-ring phasing, which is expensive to establish at launch and to hold.
Coverage was tested against 64 independent random per-ring phase offsets per
configuration, over a full rotation at 30 s × 72 azimuths × 3 band offsets, and
the minimum never moved: the phase-locked minimum equals the worst random
minimum in every configuration tested, with zero failures in 256 phase
vectors.

That is robustness to *uncoordinated* phasing, which is the operationally
relevant case — a launch campaign produces arbitrary offsets, not adversarial
ones. It is **not** immunity to every phasing. A structured pattern does break
it: offsetting each ring half a slot from the last, the triangular interleave a
cellular network would use, opens a gap at the 12-satellite baseline that the
aligned wheel does not have (see the `phasing_options` example, and the
`the_half_slot_alternation_breaks_the_baseline` test that pins it). The
conclusion stands but must be stated precisely: **the fleet is free to ignore
inter-ring phase, not free to choose it badly.**

**How many rings actually serve depends on latitude**, because polar planes are
30° apart at the equator but all converge at the poles:

| \|latitude\| | rings in reach | rings serving | duty ring's share |
|---|---|---|---|
| 0–15° | 1–2 | 1–2 | 49% |
| 15–30° | 1–2 | 1–2 | 44% |
| 30–50° | 1–3 | 1–3 | 37% |
| 50–70° | 2–6 | 1–6 | 19% |
| 70–90° | 6 | 4–6 | 17% |

So the duty ring genuinely does the heavy lifting where most of the band lies —
about half the visible satellites below 15° latitude — and the idea dissolves
toward the poles, where its share falls to 17%, the level a uniform share of
six rings would give anyway.

## Consequences

- **Launch campaigns are unconstrained in order and timing.** A plane's launch
  window recurs once per 11.2-day rotation, with successive planes' windows
  22.4 h apart. Because coverage is phase-agnostic, a slipped window costs
  nothing but the missing spacecraft, and no launch must hit an along-orbit
  slot relative to another ring.
- **No propellant is budgeted for inter-ring phase maintenance.** Holding a
  phase would be a permanent obligation: a 100 m semi-major-axis injection
  error walks a satellite through a full 30° in-ring slot in about 14 months
  (≈40 mm/s to null, but recurring forever, on every spacecraft). Intra-ring
  slot maintenance is still required.
- **Handover design must select across rings**, not just along one ring
  (ADR-0015). A town's serving ring changes as well as its serving satellite.
- **Dual coverage is priced.** 6 × 18 = 108 satellites at 2,200 km raises the
  minimum to 2, phase-robustly — the make-before-break redundancy TER-REQ-014
  needs and ADR-0003 deferred.
- The `duty_plate` and orbit-explorer animations keep the amber duty ring, but
  must not imply the other rings are idle.
