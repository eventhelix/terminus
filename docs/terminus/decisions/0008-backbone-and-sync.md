# ADR-0008: Backbone routing, ISL synchronization, and PNT placement

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-003, TER-REQ-013, TER-REQ-015, TER-REQ-016
Evidence: `cargo run -p helixsim-orbits --example backbone` (tag: terminus-post-11)

## Decision

1. **No inter-ring LEO↔LEO links.** At any moment one duty ring serves the
   band (two adjacent rings only during the ~22.4 h seam window), and every
   ring reaches the MEO shell directly, so access rings never need to talk
   to each other. Each ring carries optional intra-ring laser links to its
   two neighbors for aggregation and resilience.
2. **LEO→MEO laser feeder links** carry all user traffic from the serving
   access satellite to its session anchor. Anchor selection stays within
   ~60° of shell separation to hold the ADR-0004 latency budget.
3. **Two-way time transfer over the same links**, with MEO master clocks
   disciplining each ring's satellites; ISL Doppler is precompensated from
   known ephemerides, per-link, exactly as ADR-0006 precompensates user
   beams.
4. **PNT collocates on the MEO shell**: the same spacecraft that carry the
   minds carry the atomic clocks and will broadcast the navigation signal
   (X-band, inheriting ADR-0005's stellar exclusion). Waveform, geometry,
   and integrity design remain Series 3; the placement is settled now.

## Why

- **Intra-ring links are free-running:** satellites sharing one circular
  ring never move relative to each other — neighbor range is a constant
  4,437 km chord (12/ring at 2,200 km), zero range rate, zero Doppler:
  point once, hold forever.
- **The feeder geometry is generous:** from any access satellite, 73% of
  the entire MEO shell sits above the planet's limb (visibility to 118°,
  ranges 17,800–31,323 km), so a handful of anchors always offers several
  reachable choices inside the 60° budget policy.
- **Feeder Doppler is large but knowable:** worst-case range rate
  ≈ 5.56 km/s — entirely deterministic between two known orbits, hence
  precompensable to tracking-loop residuals; no ISL ever searches.
- **Sync begets navigation:** the two-way-time-transfer fabric needed just
  to run the network is the seed of the PNT service; collocating clocks
  with the compute payloads (large arrays, radiators, long dwell) buys the
  navigation layer for the mass of a clock, not a constellation
  (TER-REQ-016).

## Consequences

- Compliance: TER-REQ-015 moves Open → Partial (placement and timing
  fabric decided; service design in Series 3); TER-REQ-013's routing path
  is now concrete (user ↔ duty ring ↔ feeder ↔ anchor).
- MEO anchor count must satisfy feeder reachability within 60° as well as
  compute capacity — one more input to the economics post.
- Seam windows briefly double feeder traffic (two rings active); capacity
  margins must cover it — Series 2 traffic modeling input.
