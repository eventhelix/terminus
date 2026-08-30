# ADR-0006: Spot-beam access with per-beam timing/Doppler precompensation

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-006, TER-REQ-009, TER-REQ-010
Evidence: `cargo run -p terminus-orbits --example spot_beams` (tags: terminus-post-8, terminus-post-8b)

## Decision

Access satellites serve the band through narrow phased-array spot beams
(reference: 1° beams at Ka ⇒ ~19 km spot radius from 2,200 km), not a
single footprint-wide beam. Each beam's downlink is pre-shifted in
frequency and pre-timed to its spot center; uplink alignment uses
satellite-commanded timing advance. Terminals perform no blind timing or
frequency search, ever (TER-REQ-009's residual budgets are set here).

## Why

Across a full 2,200 km footprint, satellite motion imposes a Doppler
uncertainty of ±460 kHz at Ka and a propagation-delay window 4.81 ms wide —
windows a terminal would have to search blind. Across one 19-km-radius
spot the spreads collapse, and the two worst cases live at opposite ends
of the footprint: Doppler spread is largest under the *nadir* spot
(11.9 kHz — overhead the shift sweeps steeply through zero) and delay
spread is largest under the *edge* spot (116 µs; only 2.25 kHz of Doppler
there, near the stationary maximum of the Doppler curve). With the
satellite precompensating each beam to its spot center, a terminal's
residuals are at most **±6 kHz and ±58 µs** — inside any receiver's
ordinary tracking range, ×77 and ×80 smaller than the blanket windows.
(Corrected 2026-08-30: this ADR originally called the edge spot the
Doppler worst case and budgeted ±1.2 kHz; the nadir-spot sweep through
zero is 5.3× steeper.) The satellite knows its own orbit and every spot's
position; the terminals, which must survive ten years untouched after a
parachute landing (TER-REQ-006), know nothing and need to know nothing.
The complexity lands on the spacecraft, which the AI can build arbitrarily
well, instead of on ten thousand unattended boxes.

Reference budgets recorded for TER-REQ-009: residual carrier offset
≤ ±6 kHz; residual timing offset ≤ ±60 µs within any spot; uplink
timing-advance closed loop converges in one round trip because the
uncertainty is bounded by spot geometry.

## Consequences

- Satellite antennas are phased arrays with per-beam digital
  precompensation; array aperture ~0.7 m at Ka for 1° beams (70·λ/D).
- Beam management (spot layout over settlements, beam-to-terminal
  assignment, revisit for sparse terminals) becomes satellite-side
  software; settlements are fixed, so spot maps are static per ring pass.
- The waveform's guard intervals need only cover the residual budgets
  above, not geometric worst cases — a Series 2 design input.
- Acquisition (post 9) builds on this: a cold-start terminal listens for a
  beacon that is already frequency-true, so even first contact involves no
  wide search.
