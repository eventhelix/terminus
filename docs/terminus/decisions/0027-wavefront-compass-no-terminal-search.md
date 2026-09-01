# ADR-0027: The terminal receive side never searches — wide-listen and the wavefront compass

Status: accepted
Date: 2026-09-01
Requirements: TER-REQ-006, TER-REQ-007, TER-REQ-008, TER-REQ-009
Evidence: `cargo run -p terminus-orbits --example first_contact` and the
`acquisition`/`radio` tests (tag: terminus-post-9d)

## Decision

During cold start the terminal forms **no receive beam at all**. It
listens with the bare element pattern of its panel (ADR-0013) — every
element hears the whole visible sky at once, ~5 dBi — and detects the
beacon by correlation against the hard-coded waveform of the rendezvous
contract (ADR-0028). The beacon's **direction is measured, not found**:
the arriving wavefront reaches one edge of the 0.5 m face a fraction of a
wavelength before the other, so the elements see the same signal at
different carrier phases, and fitting the tilt of that phase plane fixes
the direction of arrival (`acquisition::doa_rms`). The reply is
transmitted along the measured wavefront — each element sending the
conjugate of the phase it received, retrodirective-style — at full array
gain. The hardware this asks of the panel is the ability to read
per-element (or per-quadrant, monopulse-style) phase on receive; both are
decades-old practice.

## Why

ADR-0007 settled the transmit side (the satellite lanterns its footprint)
and left "a broad-beam X receive mode" as an assumption. A directional
receive antenna would have made acquisition a *two-sided* spatial search —
the link only closes when both rasters point at each other — and both
ways of paying that price lose:

- **Unsynchronized receive raster:** the 0.5 m panel throws a 5.0° beam
  at X, and the sky above the 25° elevation floor tiles into ≈ 607 beam
  positions (`sky_positions`). To guarantee intersection with the
  lantern's walk, each position must be held for one full 13.3 s round:
  ≈ 135 minutes, **9× over** TER-REQ-008's 15-minute bound.
- **Nested fast raster** (sweep all 607 positions electronically inside
  each 10 ms dwell): the pointed beam buys +25.7 dB of array-over-element
  gain but splitting the dwell 607 ways costs 27.8 dB of integration
  time — a net **−2.2 dB, worse than not scanning at all**.
- **Wide-listen closes with margin.** Worst case twice over — edge slant
  (3,642 km) and the element's own pattern leaned 65° off boresight — a
  10 W lantern behind the satellite's 0.7 m X aperture still delivers
  **18.9 dB SNR** in the 50 kHz beacon channel (`thermal_noise_dbw`):
  detection inside a single dwell. The narrow channel is what makes the
  humble antenna sufficient — kTB physics, the same reason a slow, narrow
  beacon can always be heard by hardware too simple to carry traffic.
- **The compass is finer than anything it must seed.** At that same worst
  case the phase-tilt fit reads the direction to **0.59° rms**
  (`doa_rms`, monopulse rule of thumb θ_bw/(1.6·√(2·SNR))) — 5.6× finer
  than the 3.31° Ka pencil (broadened at the 65° scan) the box must
  eventually point. The reply returns at 26.2 dBi, +25.7 dB over the
  bare element, without the box ever computing where *it* is.
- **TER-REQ-009's philosophy, completed.** The requirement forbids blind
  timing and Doppler search; precompensation (ADR-0006) killed the
  frequency dial and the spot geometry killed the timing sweep. This
  decision kills the last conceivable dial — the spatial one. The budget
  line reads: frequency search 0 s, spatial search 0 s.

## Consequences

- The panel specification (ADR-0013) gains a receive requirement:
  per-element or per-quadrant phase readout in the acquisition band. Full
  digital beamforming satisfies it trivially; a four-quadrant monopulse
  arrangement is the floor.
- The beacon budget now has stated absolute anchors (10 W, 50 kHz, 290 K,
  5 dBi element) recorded in ADR-0028's contract; Series 2 waveform work
  inherits them.
- The first-contact ceremony is unchanged in time — the compass is read
  within the same 10 ms dwell that delivers the spot identity — so the
  43 s worst-case budget and its 21× margin stand.
- "The terminal never searches once" is now a decision with priced
  alternatives, not a slogan.
