# ADR-0009: Timetable operations, keep-alive liveness, and failure response

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-013, TER-REQ-014, TER-REQ-015, TER-REQ-016
Evidence: `cargo run -p terminus-orbits --example clock_rates` (tag: terminus-post-12)

## Decision

1. **Routing is a precomputed timetable, not a discovery protocol.** Duty
   schedules, feeder visibility windows, anchor assignments, and beam maps
   are all computable from ephemerides far in advance; every node carries
   the plan *and* a pre-assigned alternate for each entry (backup anchor,
   alternate feeder, reverse-ring path). No dynamic route computation runs
   in normal operation, and simulated behavior equals flown behavior.
2. **Keep-alive liveness chooses between plan and alternate.** All ISLs
   (necklace and feeder) and the anchor layer exchange keep-alives at a
   100 ms reference interval; three missed declares a failure in 300 ms,
   and affected nodes switch to the timetable's alternate column
   immediately. Failure handling changes *which precomputed path runs*,
   never how paths are computed.
3. **Failure responses by class:**
   - *MEO anchor loss:* sessions re-anchor to the pre-assigned backup
     anchor (73% shell visibility guarantees alternates); the L1/L2
     vault's transcript replica bounds conversation loss to the in-flight
     exchange (ADR-0004).
   - *LEO satellite loss:* neighbors detect via necklace-link silence;
     the timetable's alternate offers adjacent-ring service near seams
     and degraded-elevation service otherwise; full restoration is a
     re-phased in-ring spare. Closing this gap completely requires the
     declared redundancy sizing (min visible ≥ 2 + spares) — economics
     post.
   - *Link loss (necklace or feeder):* every LEO satellite carries its
     own feeder terminal, so necklace links are optimization, not
     lifeline; a ring is a cycle, so intra-ring traffic reverses
     direction around it.
4. **Relativistic clock corrections are part of the timing fabric.** MEO
   clocks run net +38.35 µs/day fast versus surface clocks (−7.27
   velocity, +45.61 gravitational) — corrected by design, as in GPS. The
   stellar tidal modulation (~28 ns/day, ~1,054× Earth's solar term) is
   NOT in the GPS playbook and must be modeled explicitly against the
   100 ns budget — a named Series 3 input.

## Why

- Determinism is the architecture's cheapest reliability: a timetable
  cannot flap, converge slowly, or loop, and its alternates are verified
  in simulation before they are ever needed. The only stochastic inputs —
  load and failures — are handled by margin and by the liveness layer.
- 300 ms detection + immediate alternate switching sits orders of
  magnitude inside TER-REQ-014's 60 s bound; the *coverage* consequence
  of a lost LEO satellite, not the switching, is the remaining exposure,
  and it is priced into the redundancy debt.
- An Earth-mass planet with a ~GPS-radius shell reuses GPS's relativity
  wholesale (+38.35 vs +38.6 µs/day) — except for the stellar tide,
  which is three orders larger than Earth's and genuinely new.

## Consequences

- Series 2 protocol work inherits the keep-alive interval and the
  alternate-switching model; interruption measurements (TER-REQ-013)
  include the 300 ms detection worst case.
- The economics post sizes: min-visible ≥ 2 constellations, in-ring
  spare count and re-phasing Δv, and anchor N+1 capacity.
- Series 3 must model the stellar tidal clock term and the flare-driven
  ionosphere; it inherits fixed-receiver, known-altitude, network-assisted
  solving and X-band's ~28× ionospheric advantage over L-band.

## Addendum (2026-08-29): the timetable is flown, not merely predicted

"Computable from ephemerides far in advance" is a statement about a
**controlled equilibrium, not ballistic flight**. Real orbits are held to the
published ephemeris by station-keeping inside tolerance boxes, and the burn
plan — including continuous low-thrust arcs — is itself part of the
timetable. The same ~1,054× stellar factor this ADR names for clocks acts on
orbits as the third-body term; it is deterministic and, with the planet
tidally locked, periodic in a fixed geometry, so the first response is to
absorb it into the reference orbits rather than fight it with propellant.
The stochastic residuals (flare radiation pressure, actuator noise) are
meters against kilometre-scale timetable tolerances. What the thesis
actually requires is only that the prediction horizon vastly exceed the
timetable-update distribution time.

Three refinements from the propagator RFC
([issue #4](https://github.com/eventhelix/terminus/issues/4)):

- **The gravity field is differently shaped, not merely quieter.** Slow
  rotation shrinks J2 by ~125×, but the permanent tidal bulge toward the
  star makes the figure triaxial: the sectorial C22/S22 terms — negligible
  beside J2 on Earth — rise to the same order as the shrunken zonal term,
  locked along the sub-stellar axis in a frame that turns once in 11.2 days.
  New resonance structure, not Earth's with smaller coefficients.
- **The shell's long-term stability is a question, not an assumption.** The
  anchors orbit at roughly a third of the prograde stability limit
  (`hill::hill_radius` ≈ 146,400 km, limit ≈ 73,200 km), and the 55°-inclined
  planes sit above the critical angle at which a dominant third body pumps
  eccentricity (Kozai–Lidov). Earth's GPS is protected by fast figure-driven
  precession; this planet's figure is ~125× weaker, so whether the pumping
  is suppressed must be settled by direct numerical integration — the
  propagator's sharpest validation target, at the real shells (2,200 km;
  20,000 km at 55°), not generic altitudes.
- **Flare-inflated drag at 2,200 km needs a bound.** Earth-like scale
  heights make the wheel's altitude untouchable; a superflared M-dwarf
  thermosphere is where that intuition is weakest. Note the response is
  day-side-locked — the same fixed geometry as everything else here — so it
  is more modelable than Earth's storm response once bounded.

**Declared debt, unpriced:** station-keeping Δv per year at both shells,
the timetable refresh cadence the residuals demand, and the three items
above — a perturbation/propagator work package (issue #4) plus a slot in
the economics post's propellant accounting. Until it is priced, "computed
before launch" should be read as "computed far beyond any protocol
timescale, and refreshed as routinely as any ephemeris service."
