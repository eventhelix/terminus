# ADR-0005: Frequency plan — Ka primary, X diversity, 1–3 GHz excluded

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-003, TER-REQ-011, TER-REQ-012
Evidence: `cargo run -p terminus-orbits --example frequency_plan` (tag: terminus-post-7)

## Decision

Access links use Ka-band (reference 30 GHz) as the primary band and X-band
(reference 8.4 GHz) as the weather/flare diversity band. L- and S-band
(1–3 GHz) are excluded from all service links. Cross-band techniques
(source packets on Ka, repair packets on X) are deferred to the Series 2
reliability posts.

## Why

- **The star owns 1–3 GHz.** The red dwarf emits coherent radio bursts
  across roughly 1–3 GHz (strongest near 1.6 GHz), and it sits permanently
  on the horizon of every terminal in the band. In-band stellar
  interference cannot be filtered away; the band is conceded (TER-REQ-011).
- **Higher bands win for fixed apertures.** Path loss grows 20·log₁₀(f),
  but two fixed-size dishes' combined gain grows 40·log₁₀(f): net
  +25.5 dB for Ka over L at the worst-case access slant (3,642 km). A
  0.5 m terminal dish that floods 26° of sky at L-band focuses to 1.4° at
  Ka — and with the 25° minimum elevation rule, that pencil never contains
  the horizon-sitting star.
- **Ka's weakness prices the diversity band.** Ka is weather-sensitive in
  an Earth-like atmosphere; X-band trades 11 dB of aperture advantage for
  robustness in rain and elevated stellar-noise conditions, and carries
  degraded-rate service through major flares (TER-REQ-012) so sessions
  never drop.

## Consequences

- Link budgets in later posts (spot beams, terminal design) use Ka 30 GHz
  / X 8.4 GHz reference frequencies and the 0.5 m / 0.6-efficiency
  terminal aperture.
- PNT signal design (Series 3) inherits the same exclusion and evaluates
  X/Ku/Ka navigation waveforms.
- Flare response is rate adaptation and band fallback, never session drop
  — a transport-layer obligation recorded for Series 2.
