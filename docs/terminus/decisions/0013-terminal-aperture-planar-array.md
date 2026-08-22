# ADR-0013: The terminal aperture is a fixed, electronically steered planar array

Status: accepted
Date: 2026-08-22
Requirements: TER-REQ-006, TER-REQ-007, TER-REQ-013
Evidence: `cargo run -p helixsim-orbits --example terminal_aperture` (tag: terminus-post-16)

## Decision

The ground terminal's satellite aperture is a **fixed, face-up planar
phased array with no moving parts** — 0.5 m effective aperture, 0.6
efficiency, the same numbers ADR-0005 fixed — not a mechanically pointed
parabolic reflector. Steering, tracking, handover repointing, and the
broad-beam receive mode used for cold-start acquisition (ADR-0007) are all
electronic. Link budgets from here on carry an explicit **scan-loss term**
for the terminal, sized at **−4.5 dB** at the 25° elevation floor.

## Why

Earlier posts priced the terminal as "a 0.5 m dish" and left its pointing
mechanism unstated. Three settled decisions make a reflector impossible,
so the choice was already forced — this ADR only writes it down and pays
for it.

- **Nothing stands still.** ADR-0002 excludes a stationary orbit: no
  satellite ever holds a fixed point in this sky. A serving satellite
  crosses in ~16.6 min (ADR-0003), so a 1.4° Ka beam (ADR-0005) must be
  repointed continuously and re-aimed at a new satellite at every
  handover.
- **A gimbal cannot meet TER-REQ-007.** Ten Earth years with no field
  service, after a parachute landing, is not a duty cycle any bearing or
  motor survives. The compliance matrix already claims "no moving parts
  asked of terminal"; a reflector contradicts it.
- **Acquisition needs two beamwidths.** ADR-0007 requires a "broad-beam
  receive mode" to hear the beacon, then a narrow beam for traffic. A
  reflector has exactly one beamwidth, set by its shape. An array changes
  beamwidth by changing the excitation — it is the only aperture that can
  do both.
- **Handover must be a routing event (TER-REQ-013, ADR-0004), ≤100 ms.**
  Electronic steering repoints in microseconds and can form the new beam
  before dropping the old one; a mechanical mount cannot make-before-break
  at all.

The cost is scan loss. A flat face tilted θ off boresight presents only
cos θ of its area, and the elements dim off-axis on top of that; we model
it as `10·rolloff·log₁₀(cos θ)` with rolloff 1.2 (1.0 is the ideal
projected-aperture law, quoted alongside for reference):

```text
elev (deg)  scan θ    cosθ loss    ×1.2 loss  gain (dBi)  beam (deg)
        90      0°        +0.00        +0.00       41.71        1.40
        60     30°        -0.62        -0.75       40.96        1.62
        45     45°        -1.51        -1.81       39.90        1.98
        25     65°        -3.74        -4.49       37.22        3.31
```

Since ADR-0003 guarantees a serving satellite at least 25° up, the panel
never steers past 65°, and **−4.5 dB is the whole exposure** — a bounded,
worst-case, once-and-for-all number, not an open risk. Holding boresight
gain at that floor instead would want a 0.84 m panel (1.68× wider, 2.8×
the area). We take the 4.5 dB from margin rather than grow the box: the
whole thrust of ADR-0012 is that ground hardware is the most expensive
hardware in the system.

Two things this does **not** disturb:

- **The band trade is untouched.** Scan loss has no frequency term, so it
  cancels from any same-geometry band comparison: Ka's advantage over
  L-band is +25.5 dB at boresight and +25.5 dB at the 25° floor
  (ADR-0005 stands as written).
- **No published number changes.** The gain law `10·log₁₀(η·(πD/λ)²)` is
  aperture-area physics, identical for a reflector and for an array of the
  same effective area. Every figure in posts 7–15 was a boresight or
  ratio figure and remains exact; `radio::dish_gain_dbi` keeps its name and
  its evidence tags, and stays the right model for the mechanically aimed
  MEO community dish of ADR-0012 and for satellite feeder links.

## Consequences

- Terminal link budgets carry a mandatory scan-loss term; the reference
  worst case is −4.5 dB at 25° elevation, and the traffic beam broadens
  from 1.4° to 3.3° there. Series 2 waveform and margin work inherits both.
- The broad-beam acquisition mode of ADR-0007 is now a defined capability
  of the aperture (a low-directivity excitation of the same panel), not an
  unbacked assumption.
- The terminal and the satellite are now the **same kind of antenna** at
  different scales — 0.5 m face-up on the ground, ~0.7 m nadir-pointing in
  orbit (ADR-0006). One technology, one supply chain, one narrative.
- The panel is face-up rather than tilted: the six access rings (ADR-0003)
  deliver satellites from every azimuth, so there is no preferred bearing
  to tilt toward, and a tilt that helps one ring's geometry hurts another's.
- TER-REQ-007's "no moving parts" claim in the compliance matrix is now
  backed by a decision rather than an assumption. The requirement stays
  Open on hardware, power, and environment grounds — the aperture is one
  of its several gaps, and only that one is closed here.
- `helixsim-orbits::radio` gains `scan_loss_db`, `planar_array_gain_dbi`,
  and `scanned_beamwidth_deg` — generic aperture math with no Terminus
  constants, per the independence invariant.
