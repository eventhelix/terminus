# ADR-0014: The service shell is sized as a navigation constellation — 24 satellites, six planes of four at 55°

Status: accepted
Date: 2026-08-23
Requirements: TER-REQ-015, TER-REQ-012, TER-REQ-004
Evidence: `cargo run -p terminus-orbits --example navigation_shell`

## Decision

The MEO service shell at 20,000 km is a Walker-style shell of 24
satellites: six planes at 55° inclination with ascending nodes spread over
the full 360°, four satellites per plane, Walker phasing F = 1. ADR-0008
placed the navigation service on this shell ("clocks beside the minds")
but left its size open; this fixes the size, and the size is set by
navigation, not by compute.

## Why

TER-REQ-015 asks for four navigation satellites visible at all times
throughout the service region. Sweeping the shell over one day of geometry,
a ±20° band, and a 10° ground mask:

| Shell | Satellites | Worst case in view | Typical |
|---|---|---|---|
| 6 × 1 | 6 | 0 | 1.8 |
| 6 × 2 | 12 | 1 | 3.6 |
| 6 × 3 | 18 | 3 | 5.4 |
| **6 × 4** | **24** | **4** | **7.2** |
| 6 × 5 | 30 | 6 | 9.1 |

Twenty-four is where the floor is met; eighteen leaves band points with
three in view. At 24 the count matters more than the split — 4 × 6 and
8 × 3 also clear the floor, with 4 × 6 holding one more in the worst case —
so the GPS-like 6 × 4 is adopted and the split left open to the geometry
work.

Two properties of the shell come from navigation rather than from
anchoring, and they are the reason this ADR exists:

- **Inclined planes, nodes over 360°.** Polar planes are their own 180°
  opposites and stack satellites over the poles; a navigation fix wants
  satellites spread in azimuth over the same town, which is what 55° planes
  spread over the full circle provide.
- **Twenty-four, not six.** Session anchoring needs only a handful of
  reachable compute nodes — the 6 × 1 row anchors every session on the band
  perfectly well while providing no fix at all. Navigation is what sizes the
  shell. The marginal 18 satellites are the price of the PNT service, and
  that is where they should be charged in the economics post.

## Consequences

- TER-REQ-015's visibility floor moves from Open to Designed; the accuracy
  and integrity half stays open. The evidence here is a **count, not a
  geometry quality** — four satellites bunched in one quarter of the sky
  dilute precision, and the GDOP, waveform, and integrity work that turns
  this count into 10 m / 100 ns remains Series 3.
- The compute-placement arithmetic of ADR-0004 is unaffected: the anchors
  ride a shell that now has 24 slots instead of an unstated handful, which
  can only help anchor diversity and failover (TER-REQ-014).
- The hybrid MEO + LEO ranging aid (ADR-0012) is evaluated against this
  shell as the baseline.
- `terminus-orbits` grows a generic `walker` module: inclined circular
  shells, with the polar constellation as the inclination = 90° special
  case. A test pins that reduction, so the two modules cannot drift.
