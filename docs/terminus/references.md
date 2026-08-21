# External Crate and Library Watchlist

Candidate dependencies and design references, with the verdict current as of
the noted date. Re-evaluate before taking a dependency.

## Orbital mechanics

### satkit — https://github.com/ssmichael1/satkit (crates.io: `satkit`)

Evaluated 2026-08-21. Rust astrodynamics: SGP4, adaptive RK and Gauss-Jackson
propagators, IERS frames, JPL ephemerides, drag/SRP force models, maneuver
support. Apache-2.0/MIT.

Verdict: **reference, not a dependency.** It is Earth-specific (Earth gravity
models, Earth frames, TLE inputs) and requires external data downloads, which
conflicts with configurable-planet design and deterministic CI. Use it to
cross-validate helixsim-orbits results for Earth-parameter cases, and as a
design reference for numerical propagation and stability work (post 4's
Hill-sphere / three-body checks).

### Lox — https://github.com/lox-space/lox (crates.io: `lox-space`)

Evaluated 2026-08-21. Rust astrodynamics with Python bindings: Keplerian
propagation, J2, SGP4, time scales, frames — plus, notably, constellation
design (Walker Delta/Star, Street-of-Coverage, Flower), ground-station
visibility and pass prediction, and RF link budgets with antenna patterns.
MPL-2.0. Pre-1.0, API unstable.

Verdict: **evaluate hands-on before building coverage/visibility code for
post 5.** Its constellation and visibility features overlap what
helixsim-orbits would otherwise grow; the open question is whether an
arbitrary tidally locked body can be configured (examples are Earth-centric —
inspect `lox-bodies`). MPL-2.0 is compatible as a dependency but must be
noted. The first-order screening spike stays in helixsim-orbits regardless:
it is closed-form math over a fully generic body.

## Ground-coverage discretization

### H3 — https://github.com/uber/h3 (Rust: `h3o`, a pure-Rust reimplementation)

Evaluated 2026-08-21. Hexagonal hierarchical geospatial index, resolutions
0–15: lat/lng → cell, boundaries, traversal, polygon fill. Apache-2.0;
production-grade.

Verdict: **strong candidate for coverage and beam-layout analysis** (post 5's
terminator-band coverage sweeps, post 8's spot-beam layout — hex packing is
the classic cellular beam pattern). Prefer the pure-Rust `h3o` crate over the
C library for determinism and build simplicity. Caveat: H3's metric helpers
(cell area, edge length) assume Earth's radius; on the reference planet,
index by angular coordinates and scale metric quantities by the planet's
radius explicitly.

## Transport and FEC (Series 2)

### QuicFuscate — https://github.com/Christopher-Schulze/QuicFuscate

Evaluated 2026-08-21. Rust VPN runtime; the relevant core is a QUIC transport
with integrated hybrid RLNC + Tetrys-like sliding-window FEC with adaptive
mode switching, SIMD GF arithmetic, and BBR congestion control. MIT.

Verdict: **design reference and benchmark candidate** for the sliding-window
and RLNC implementations behind the project's `FecCodec` trait. Its
obfuscation/stealth features are out of scope. Modest maturity — do not
build on it directly.

### Issue #2 codec candidates (for the `FecCodec` trait)

- `cberner/raptorq` — RFC 6330 RaptorQ; baseline for generation-based FEC.
- `reed-solomon-simd` — small fixed generations.
- `tambur-rs` — sliding-window FEC for real-time traffic; maturity concern,
  keep behind the trait.
