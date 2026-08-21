# ADR-0001: Reject active terminator-tracking orbital planes

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-001, TER-REQ-016
Evidence: `cargo run -p helixsim-orbits --example terminator_tracking` (tag: set when post 3 publishes)

## Decision

The constellation uses inertially fixed orbital planes with service handed
from plane to plane as the terminator rotates. No plane actively precesses to
follow the terminator.

## Why

The terminator rotates at 32.14°/day in inertial space (11.2-day synchronous
rotation). Holding a LEO plane on it costs an ideal lower bound of
Δv ≈ v·ΔΩ ≈ 3.9–4.2 km/s per day across 600–2,000 km altitudes, which is a
continuous cross-track acceleration of ~0.045 m/s² (~23 N for a 500 kg
spacecraft). Even at Isp = 3,000 s the rocket equation burns ~12.5% of
spacecraft mass per day. No long-lived constellation survives this.

## Consequences

- Coverage of the inhabited band (TER-REQ-001) comes from multiple fixed
  planes plus a preferred-plane handoff (~every 22.4 h for 6 planes).
- The excellent solar geometry of a terminator-aligned plane is only ever
  transiently available; power design must assume eclipse cycles and
  articulated arrays.
- Post 3 ("The seductive wrong answer") presents this trade; posts 4–5 build
  on fixed planes.
