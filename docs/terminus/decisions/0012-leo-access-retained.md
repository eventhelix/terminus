# ADR-0012: LEO access retained — the MEO-direct alternative priced and declined

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-005, TER-REQ-006, TER-REQ-016
Evidence: `cargo run -p helixsim-orbits --example access_trade` (tag: terminus-post-15)

## Decision

The LEO access rings (ADR-0003) are retained. The alternative raised in
issue #2 (Option C / "MEO may eventually eliminate LEO") — serving users
directly from the MEO shell, collapsing access and compute onto one
constellation — is evaluated and declined for the baseline, and kept as a
contingency case for the economics post.

## Why

MEO-direct access has real merits, and we state them first: 3.4 h dwell
versus 16.6 min (handovers become rare events), a Doppler window of
±85 kHz versus ±460 kHz before precompensation, no feeder hop, and a
10 s beacon raster. If terminals were engineered artifacts we could
grow at will, Option C would be attractive.

They are not, and that decides it:

- **Aperture.** Worst-case slant grows from 3,642 km to 23,039 km:
  +16.0 dB of path loss. Recovered at the terminal alone, the 0.5 m dish
  becomes 3.2 m (or 40× the transmit power); even with the satellite
  paying half, the terminal needs 1.26 m. A parachuted, decade-unattended
  box (TER-REQ-006) cannot carry it.
- **Spot reuse.** A 1° beam paints a 175 km spot from MEO versus 19 km
  from LEO — 83× less spatial frequency reuse, working directly against
  the 10⁴ → 10⁶ terminal growth requirement (TER-REQ-005).
- The evaluation currency (TER-REQ-016) is total system cost: Option C
  moves cost from ~72 relay satellites onto ten thousand ground
  terminals and every future one — the wrong direction for this
  civilization.

## Consequences

- The two-layer architecture (LEO access + MEO service) is now justified
  against its strongest challenger, not merely asserted; the economics
  post re-runs Option C with the capacity model as due diligence.
- MEO-direct remains a designed-degraded contingency: a settlement
  willing to host one larger community dish could reach the MEO shell
  directly if its duty-ring service were lost — an input to the
  redundancy sizing work.
- The retained LEO layer also serves Series 3: hybrid MEO+LEO ranging —
  LEO's fast orbital motion sweeps out strong Doppler and geometry
  variation that the slow MEO shell cannot provide — shall be evaluated
  as a PNT accuracy aid (issue #2, PNT §6).
