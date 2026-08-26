# ADR-0003: Access constellation baseline — 6 polar rings × 12 at 2,200 km

Status: accepted (amended 2026-08-25; see ADR-0016)
Date: 2026-08-21
Requirements: TER-REQ-001, TER-REQ-016
Evidence: `cargo run --release -p terminus-orbits --example access_constellation` (tag: terminus-post-5)

## Decision

The access layer baseline is 6 circular polar rings with ascending nodes
spread evenly over 180°, 12 satellites evenly phased per ring, at 2,200 km —
72 satellites. This supersedes the issue #2 working seed of 6 × 12 at
1,800 km, and updates the reference orbit label from "LEO access ~1,800 km"
to "LEO access 2,200 km".

## Why

Full-rotation coverage simulation of the ±20° band at 25° minimum elevation
(30 s / 72-azimuth sampling) shows the 1,800 km seed leaves band points with
zero usable satellites: at the seam midway between rings, a point sits ~15°
cross-track while a ring's along-track coverage window is only ±13.4°,
narrower than the 30° satellite spacing. Two fixes close the gap: 12 more
satellites at 1,800 km (84 total, min visible 1), or the same 72 satellites
raised to 2,000–2,400 km. 2,000 km clears by only 2.8% of the along-track
window — too thin to bet a civilization on. 2,200 km clears with real
margin (mean 3.45 visible vs 3.07) for an edge-latency cost of ~12 ms vs
~10 ms one way. Free altitude beats 16% more spacecraft (TER-REQ-016).

Inter-plane phase stagger does not rescue 1,800 km: coarse sampling
(120 s / 36 azimuths) suggested it did, but 30 s / 72-azimuth sampling shows
gaps at every stagger tried. Recorded as a methodology caution: coverage
minima must be confirmed at fine sampling.

## Consequences

- Minimum visible = 1 is bare coverage with no failure redundancy
  (TER-REQ-014) and no dual-coverage guarantee for make-before-break
  handover. Later posts must size overlap explicitly; satellite count may
  rise again for redundancy, not geometry.
- Preferred-ring handoff cadence: the terminator aligns with the next ring
  every ≈ 22.4 hours (30° at 32.14°/day). The preferred ("duty") ring is the
  ring carrying the most traffic, not the only ring serving — see ADR-0016.
- Latency/link budgets in later posts use 2,200 km for the access layer.
