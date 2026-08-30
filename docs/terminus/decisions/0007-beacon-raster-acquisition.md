# ADR-0007: Cold-start acquisition by beacon raster

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-006, TER-REQ-008, TER-REQ-009
Evidence: `cargo run -p terminus-orbits --example first_contact` (tag: terminus-post-9)

## Decision

Every access satellite continuously rasters a beacon beam across its full
footprint, spot position by spot position (reference: 10 ms dwell per spot),
in addition to the dedicated beams serving known settlements. A cold-start
terminal simply listens; the beacon it eventually hears is precompensated
(ADR-0006) and carries the spot's identity and the uplink response grant.
After first contact the terminal's spot joins the served map, making every
later outage a warm start against a scheduled beam.

## Why

- **Coverage makes the sky never empty** (ADR-0003: minimum one satellite
  ≥ 25° up everywhere, always), so a cold terminal never waits for a
  satellite — only for the lamplighter's lantern to swing its way.
- **The raster is short.** A footprint (radius 2,518 km) tiles into
  ≈ 16,983 spot positions of 19.2 km radius; at 10 ms dwell a full raster
  takes ≈ 170 s. Worst-case cold start — full raster wait, zero frequency
  search (precompensation), one round trip of timing alignment (24 ms),
  and a 30 s registration allowance — totals ≈ 3.3 minutes, 4.5× inside
  TER-REQ-008's 15-minute bound. The count deliberately prices every
  position at nadir-spot size, ignoring the elongation of leaning spots
  (ADR-0006's farther/flatter/fatter, up to 5.3×): a raster stepped at
  uniform nadir pitch over-tiles the rim, so 170 s is a ceiling — the
  true beam-space count is roughly a third of it — and the overlap lands
  where the link budget is thinnest.
- **Warm start is trivial by construction.** A remembered spot identity
  plus the scheduled beam plan bounds reacquisition by one beam revisit —
  seconds against the 30 s requirement.
- The terminal's role remains "listen, lock, answer": no stored almanac,
  no clock, and no position are ever required (TER-REQ-006), keeping all
  acquisition complexity on the spacecraft.

## Consequences

- Beacon capacity is a fixed overhead per satellite (one beam of the
  array, always rastering); it also serves as the network's discovery
  channel for terminals delivered to unregistered locations.
- The beacon waveform must be decodable at acquisition-beam gain by a
  terminal's broad-beam receive mode — a Series 2 waveform design input.
- Registration traffic (identity, spot fix, capability exchange) gets a
  30 s allowance in the budget; the protocol design must fit it.
