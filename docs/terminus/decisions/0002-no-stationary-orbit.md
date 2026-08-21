# ADR-0002: No stationary orbit — the regime survey excludes it

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-001, TER-REQ-016
Evidence: `cargo run -p helixsim-orbits --example regime_survey` (tag: terminus-post-4)

## Decision

The constellation uses no stationary (synchronous) satellites. Architecture
work proceeds on the LEO and MEO shelves; every regime from VLEO to high MEO
remains in play for the coverage and compute trades.

## Why

A stationary orbit around this planet would sit at ≈ 211,500 km from the
center (altitude ≈ 205,000 km), because "one orbit per rotation" takes 11.2
Earth days. But the planet orbits a 0.122-solar-mass star at only 0.0485 AU,
so its Hill radius — the region where its gravity dominates the star's — is
≈ 146,000 km, and long-lived prograde orbits are only trusted out to roughly
half of that, ≈ 73,000 km. The synchronous radius is ≈ 2.9× that limit: the
star strips any satellite placed there. Even if it existed, edge latency
would be ≈ 0.7 s one way, hostile to interactive service (TER-REQ-003).

## Consequences

- No fixed point in the sky: fixed ground antennas cannot stare at one
  spot; every architecture must handle satellite motion and handover.
- The screening table (footprint, edge latency, pass duration per shelf)
  becomes the working menu for posts 5–6: LEO shelves for low-latency
  access, MEO shelves for long dwell and large footprints.
- L1/L2 (≈ 146,000 km) remain the only quasi-fixed locations in the
  planet's frame, reserved for non-interactive infrastructure roles.
