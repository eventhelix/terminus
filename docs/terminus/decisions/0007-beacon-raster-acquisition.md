# ADR-0007: Cold-start acquisition by X-band beacon raster

Status: accepted (amended 2026-08-30: lantern moved from Ka to X)
Date: 2026-08-21
Requirements: TER-REQ-006, TER-REQ-008, TER-REQ-009, TER-REQ-012
Evidence: `cargo run -p terminus-orbits --example first_contact` (tags: terminus-post-9, terminus-post-9b, terminus-post-9c)

## Decision

Every active access satellite continuously rasters a beacon beam across
its footprint's share of the habitable band, spot position by spot position (reference: 10 ms dwell per
spot) — **on X band, downlink and uplink alike**. A cold-start terminal
simply listens; the beacon it eventually hears is precompensated
(ADR-0006) and carries the spot's identity and the uplink response grant.
The terminal answers on X and **requests illumination**: the network folds
its spot into the Ka service plan, and every later outage is a warm start
against a scheduled beam. The same handshake is the network's all-weather
fallback trigger: a terminal whose Ka beam is drowned by rain answers the
lantern and requests sustained X-band service for its spot.

## Why

- **Coverage makes the sky never empty** (ADR-0003: minimum one satellite
  ≥ 25° up everywhere, always), so a cold terminal never waits for a
  satellite — only for the lamplighter's lantern to swing its way.
- **X costs nothing in frequency certainty and buys a 13× shorter round.**
  A beam's Doppler spread is (f/c)·v·β, and a diffraction-limited beam has
  β = k·λ/D — so the spread is v·k/D, set by the aperture alone, the same
  for every band. The 0.7 m array that throws a 1° pencil at Ka throws a
  3.57° beam at X with the identical ±6 kHz residual, and the footprint
  tiles into ≈ 1,333 positions of 68.5 km radius instead of ≈ 17,000: a
  full raster takes ≈ 13.3 s. Worst-case cold start — full raster wait,
  zero frequency search (precompensation), one round trip of timing
  alignment (24 ms), and a 30 s registration allowance — totals ≈ 43 s,
  21× inside TER-REQ-008's 15-minute bound. The count still prices every
  position at nadir-spot size, ignoring the elongation of leaning spots
  (ADR-0006's farther/flatter/fatter): a raster stepped at uniform nadir
  pitch over-tiles the rim, so 13.3 s is a ceiling, with the overlap
  landing where the link budget is thinnest.
- **X is the band storms cannot take.** The lantern lives where rain
  costs 1.9 dB instead of Ka's 23.5 dB (ADR-0005's diversity band), so
  first contact works in the weather a new terminal may land in — and the
  same raster is the recovery path when weather silences a served spot's
  Ka beam (TER-REQ-012): one lantern round, one answer, and the network
  schedules longer-dwell X service for the impacted spots.
- **The wide spot's timing spread lands in orbit.** The X spot at the
  footprint rim stretches to ±377 km, so a first reply arrives within
  ±1.1 ms of the satellite's expectation — a wide acceptance window the
  satellite absorbs, exactly where complexity belongs; Ka service keeps
  its ±308 µs swept worst case (budgeted ≤ ±310 µs, ADR-0006).
- **Warm start is trivial by construction.** A remembered spot identity
  plus the scheduled beam plan bounds reacquisition by one beam revisit —
  seconds against the 30 s requirement, and the lantern's 13.3 s round
  bounds it even when a storm has taken Ka.
- **The raster region is one generic rule: footprint ∩ habitable band.**
  Only active satellites raster (ADR-0016/0017's duty ring plus
  hole-fillers; dark satellites carry no lantern — safe, because
  TER-REQ-001 guarantees every band point an *active* satellite), and
  each evaluates the same intersection (plus a drop-wind margin) rather
  than special-casing its role. `band_raster_fraction` prices it: a
  duty-ring satellite riding the terminator keeps 95% of its footprint
  (a 22.65° cap against the ±20° band), a satellite 10° off keeps 78%,
  20° off 52%, and a hole-filler 30° off just 24% — a 3.2 s round
  instead of wasting most of 13.3 s on nightside ice. Trimming only ever
  shortens rounds, so the 13.3 s full-footprint ceiling stands.
- The terminal's role remains "listen, lock, answer": no stored almanac,
  no clock, and no position are ever required (TER-REQ-006), keeping all
  acquisition complexity on the spacecraft.

## Consequences

- Beacon capacity is a fixed overhead per satellite (one X beam of the
  array, always rastering); it also serves as the network's discovery
  channel for terminals delivered to unregistered locations and as the
  service-request channel for weather fallback.
- The beacon waveform must be decodable at acquisition-beam gain by a
  terminal's broad-beam X receive mode — a Series 2 waveform design input,
  as is the capacity plan for sustained X service in impacted spots
  (X carries less than Ka; the fallback trades rate for continuity).
- Registration traffic (identity, spot fix, capability exchange) gets a
  30 s allowance in the budget; the protocol design must fit it.
- The constituent techniques (signaling on a wide lower-band beam beside
  narrow service spots, forward-channel logon, demand-driven beam
  illumination) are decades-old practice documented in expired patents
  and open standards (Intelsat global/spot beams, Iridium ring alert,
  DVB-RCS logon, DVB-S2X beam hopping, 3GPP NTN); a formal
  freedom-to-operate search is only required if this design is ever
  practiced as a real system rather than published and simulated.
