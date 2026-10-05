# ADR-0031: Terminal attitude, handover pointing, and the landing requirement

Status: accepted
Date: 2026-10-05
Requirements: TER-REQ-006, TER-REQ-007, TER-REQ-008, TER-REQ-013
Evidence: `cargo run -p terminus-orbits --example terminal_attitude`; `crates/orbits/src/attitude.rs` unit tests (tag: terminus-post-9l)

Completes ADR-0027. The wavefront compass hands the box one bearing; this
ADR settles what one bearing cannot do, how the terminal finds and points
at each next satellite, and what the box's landing must guarantee.

## Decision

- **The terminal estimates its own attitude** from two sources, both
  solid-state: a **MEMS accelerometer** (gravity, ≈0.25° rms) and the
  **bearings** it already measures. A TRIAD from gravity and one bearing
  serves while the serving satellite stands at least 30° from the zenith;
  near the zenith, two bearings at least 40° apart along the pass serve
  instead. The satellite supplies each bearing's direction in the local
  level frame live, in the downlink — nothing is stored.
- **Handover is three steps.** (1) The attitude says where the incoming
  satellite will be, to within ≈3.7° (95th percentile, cold-start
  bearings). (2) The box forms a **listening beam sized to that
  uncertainty** — 5° half-width, a 0.25 m patch of the face — hears the
  incoming satellite's scheduled X dwell, and reads its bearing off the
  wavefront to ≈0.29° rms. (3) The Ka pencil points there and **tracks
  closed-loop** (monopulse); every tracked bearing refreshes the attitude.
- **The landing requirement: upright to within 5°.** The box's weighted,
  rounded base rights it, as a tumbler toy rights itself. A box its
  accelerometer finds still tilted past 5° — wedged on a stone, propped
  in a furrow — says so in words and light and asks the villagers to set
  it level, confirming when it is. No technician, no tool.

## Why

- **One bearing fixes two attitude angles, not three.** A rotation about
  the line to the satellite leaves that bearing unchanged. That is
  harmless for the conjugate reply (ADR-0027), which retraces the arrival
  with no attitude at all, and for tracking the serving satellite
  closed-loop. It is fatal for handover: "the next satellite is there" is
  known in the local level frame, and turning it into the panel's frame
  needs all three angles.
- **Attitude alone cannot point a Ka pencil.** With cold-start bearings
  (0.87°) the worst handover pointing error over the sky is 1.9–3.4° for
  gravity + bearing (satellite 64°–30° from the zenith) and 2.7–3.7° for
  two bearings 60°–40° apart, against a Ka pencil 0.70° wide each side
  overhead. Gravity + bearing collapses near the zenith (19° at 5°) where
  the two directions nearly coincide; two bearings cover that case, and
  close bearings (13.6° at 10° apart) are no use. The policy bound is
  3.66°.
- **A listening beam bridges the gap without searching.** The phased
  array can form a beam of any width; one just wide enough to cover the
  attitude bound has nowhere else to look. At the worst case — rim slant
  3,642 km, satellite steered 42°, 3 dB beam-edge loss, the box leaned
  70° (65° plus the landing tolerance) — it gains +19.0 dBi against
  −0.6 dBi for a bare element, the X dwell lands at 32.9 dB in 50 kHz,
  and the bearing reads to 0.29° rms: inside the Ka pencil's 2.05°
  half-width at that lean, with room to spare.
- **Tilt spends scan-loss budget fast.** ADR-0013's −4.49 dB assumed a
  level face. Tilted τ, the worst lean is 65° + τ: −5.59 dB at 5°
  (−1.10 dB over budget), −7.04 dB at 10°, −9.12 dB at 15°, −12.72 dB at
  20°. A 5° requirement costs 1.1 dB, and the first-contact beacon's
  weakest point falls from 15.1 dB to 14.1 dB — still a single-dwell
  detection. Beyond that the losses climb too steeply to absorb, so the
  box must be level, by its own shape or by willing hands.
- **People are the field service the RFP allows.** TER-REQ-007 rules out
  technicians, not villagers. A box that can ask to be set straight turns
  the one mechanical failure a parachute landing can cause into a
  thirty-second chore for whoever is standing nearby.

## Consequences

- The terminal carries an accelerometer and a voice (speaker, with a
  light ring for the hard of hearing): solid-state, no moving parts,
  consistent with ADR-0013.
- The handover message gains the incoming satellite's direction in the
  local level frame and the time of its X dwell over the served spot.
- Ka service link budgets carry a 1.1 dB tilt allowance on top of
  ADR-0013's −4.49 dB scan loss.
- Bearings taken while tracking, with the full array, are sharper than
  the 0.87° priced here; once the Ka service link budget is modeled they
  will shrink the attitude bound and the listening beam with it.
