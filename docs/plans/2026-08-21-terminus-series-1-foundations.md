# Terminus Series 1 Foundations — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the three first milestones of the Terminus blog series: the canon skeleton (world bible, requirements baseline, decision log), the generic orbital-geometry crate spike that reproduces the terminator-tracking Δv numbers, and the RFP blog post draft.

**Architecture:** Canon documents live in `helixsim/docs/terminus/` and version-lock with the code. A new generic crate `helixsim-orbits` (`crates/orbits`) provides first-order orbital screening math, configured entirely by body parameters — no Terminus concepts in code. The RFP post drafts into the eventhelix.com Zola site as a new `terminus` section, marked `draft = true` so it does not publish until ready.

**Tech Stack:** Rust 2021 (workspace conventions already in `Cargo.toml`), plain `f64` math (no new dependencies), Zola TOML-frontmatter Markdown for the site.

**Spec:** `docs/specs/2026-08-21-terminus-series-design.md` (this plan implements its "First milestones" section; the spec's independence principle and reproducibility contract govern every task).

## Global Constraints

- **Two working directories.** Tasks 1–7 run in `C:\Users\sande\Documents\repos\helixsim` (branch `terminus`). Task 8 runs in `C:\Users\sande\Documents\repos\site` (create branch `terminus` from its default branch first).
- **Independence principle (spec):** no Terminus/Proxima names, constants, or narrative in `crates/orbits` code, docs, or examples. The example uses neutral wording ("a tidally locked, Earth-sized reference planet"). Terminus specifics go only in `docs/terminus/` and the site post.
- **Crate conventions:** package name `helixsim-orbits`, workspace-inherited `version/edition/rust-version/license/repository/homepage/authors` exactly like `crates/core/Cargo.toml`. Edition 2021, rust-version 1.79.
- **No wall-clock reads, no unseeded randomness** (helixsim invariant; this crate is pure math, so simply add none).
- **Commits:** conventional-commit subjects (`feat:`, `docs:`, `test:`). **No AI/Claude attribution lines or Co-Authored-By trailers.**
- **Numbers are canon.** Test constants below were computed with μ_Earth = 3.986004418e14 m³/s² and g₀ = 9.80665 m/s². Do not "fix" a failing test by loosening tolerance beyond 1e-3 relative; a larger discrepancy means the implementation is wrong.
- Run all Rust commands from the helixsim repo root. `cargo test -p helixsim-orbits` must pass at the end of every task that touches the crate; `cargo check` for the whole workspace must stay green.

---

### Task 1: World bible

**Files:**
- Create: `docs/terminus/world-bible.md`

**Interfaces:**
- Produces: the canonical parameter set later tasks cite. Key values: planet radius 6,371 km; mass 1 Earth mass (μ = 3.986004418e14 m³/s²); synchronous rotation/orbit period 11.2 Earth days; star radio-loud in 1–3 GHz; inhabited band ±20° of the terminator.

- [ ] **Step 1: Write the file** with exactly this content:

```markdown
# Terminus World Bible

Canonical parameters for the Terminus series. Every post and simulation cites
these values; changes require an ADR in `decisions/`.

## The planet

An Earth-sized planet tidally locked to a Proxima-like red dwarf. Modeled on
Proxima Centauri b; where Proxima b's properties are uncertain (it does not
transit, so its radius is inferred), the reference model uses Earth values.

| Parameter | Canonical value | Notes |
|---|---:|---|
| Radius | 6,371 km | Earth mean radius |
| Mass | 1.0 Earth mass | μ = GM = 3.986004418e14 m³/s² |
| Rotation period | 11.2 Earth days (967,680 s) | Synchronous: rotation = orbital period |
| Orbital distance | 0.0485 AU ≈ 7.256e6 km | |
| Surface gravity | 9.81 m/s² | Follows from radius and mass |
| Atmosphere | Earth-like pressure and composition | Working assumption; drives link and drag models |

Because the planet is synchronously locked, the terminator is fixed on the
surface but rotates in inertial space at 360°/11.2 days ≈ 32.14°/day
(6.4928e-6 rad/s).

Derived orbital reference points (from the mass and rotation period):

| Quantity | Value |
|---|---:|
| Synchronous orbit radius | ≈ 211,300 km (altitude ≈ 205,000 km) |
| L1/L2 distance | ≈ 150,000 km from the planet |
| L4/L5 distance | ≈ 7.25e6 km (orbital radius scale) |

The synchronous orbit lies near or beyond the prograde stability limit of the
planet's small Hill sphere; a stationary satellite must not be assumed
available.

## The star

A Proxima Centauri-like M dwarf.

- Produces coherent radio emission in roughly 1–3 GHz, with strong activity
  near 1.6 GHz. L/S band is therefore hostile spectrum.
- Flares (UV/X-ray) disturb the ionosphere and raise the radio noise floor;
  the system must degrade gracefully during major flares.
- From the terminator, the star sits approximately on the local horizon;
  satellites above a minimum elevation (reference: 25°) are angularly
  separated from it.

## The civilization

- Pre-industrial ("budding") civilization concentrated in a habitable band
  within ±20° great-circle arc of the terminator.
- Initial service population: about 100 settlements, 10,000 ground terminals.
- No planetary industry, datacenters, launch capability, or field service.
  All space and ground hardware is manufactured and deployed by the Alien AI:
  satellites placed in orbit, terminals delivered by parachute.
- End users interact with the LLM through WiFi touch devices provided with
  the terminals; a terminal is a WiFi base station backhauled by satellite.

## Time units

The planet's day and year are the same 11.2-Earth-day period. To avoid
ambiguity, all engineering durations in canon, requirements, and posts are
stated in Earth units (seconds, days, years); in-universe this is rationalized
as the AI's native time standard.

## Reference orbit labels (illustrations and posts)

- LEO access: ~1,800 km
- MEO service / compute / PNT: ~20,000 km
- Regime comparisons add: VLEO ~300 km, stationary ~205,000 km ("likely
  unstable")
```

- [ ] **Step 2: Verify the derived numbers.** Check 360/11.2 = 32.142857°/day and 32.142857 × π/180 / 86400 = 6.4928e-6 rad/s with a quick calculation. Expected: values match the file.

- [ ] **Step 3: Commit**

```bash
git add docs/terminus/world-bible.md
git commit -m "docs: add Terminus world bible"
```

---

### Task 2: Requirements baseline

**Files:**
- Create: `docs/terminus/requirements.md`

**Interfaces:**
- Produces: requirement IDs `TER-REQ-001`..`TER-REQ-016` cited by all posts, starting with the RFP post (Task 8).

- [ ] **Step 1: Write the file** with exactly this content:

```markdown
# Terminus Requirements Baseline

The Alien AI's RFP requirements. Every proposal post traces its design
decisions to these IDs. Values are in-universe procurement targets; where a
trade study later shows a target should move, the change is recorded as an
ADR and the baseline is updated.

## Mission and service

| ID | Requirement |
|---|---|
| TER-REQ-001 | Provide continuous service over the inhabited band: all surface points within ±20° great-circle arc of the terminator. |
| TER-REQ-002 | The service is interactive access to LLM inference hosted on the provider's space infrastructure. No planetary datacenter or ground relay network may be assumed. |
| TER-REQ-003 | First-token latency ≤ 300 ms (p95). Steady-state token stall ≤ 100 ms (p99). |
| TER-REQ-004 | Service availability ≥ 99.9% per settlement per Earth year, measured as the fraction of time TER-REQ-003 is met. |
| TER-REQ-005 | Support 10,000 terminals across ~100 settlements at initial service; scale to 1,000,000 terminals without constellation redesign. |

## Ground segment

| ID | Requirement |
|---|---|
| TER-REQ-006 | Terminals are delivered by parachute, fully self-contained (power, satellite antenna, radio, WiFi base station), and require no assembly. |
| TER-REQ-007 | Terminals operate for ≥ 10 Earth years with no field service and no skilled operators. |
| TER-REQ-008 | Terminal cold start — power-on with no stored almanac, time, or position — to full service in ≤ 15 minutes. Reacquisition after an outage in ≤ 30 seconds. |
| TER-REQ-009 | Terminals shall not perform blind timing or Doppler search. The space segment presents a time- and frequency-precorrected air interface (e.g. per-beam precompensation); residual offsets at the terminal must fall within the waveform's guard budgets. The proposal's beam-size trade study sets the residual budget values. |
| TER-REQ-010 | End users access the service with provided WiFi touch devices only; the terminal is their WiFi base station. No other user equipment may be required. |

## Environment

| ID | Requirement |
|---|---|
| TER-REQ-011 | Primary service links shall avoid, or demonstrably tolerate, the star's coherent radio emission band (~1–3 GHz, strongest near 1.6 GHz). |
| TER-REQ-012 | During major stellar flares the service may degrade in rate but shall not drop established sessions; PNT integrity alerts must reach users within 10 s. |

## Continuity and reliability

| ID | Requirement |
|---|---|
| TER-REQ-013 | Satellite handover is a routing event: no session restart, no transport reconnection. Handover interruption ≤ 100 ms (consistent with TER-REQ-003 stall budget). |
| TER-REQ-014 | No single satellite failure may interrupt service to any settlement for more than 60 s. Loss of a compute node mid-conversation loses at most the in-flight exchange. |
| TER-REQ-015 | Provide PNT throughout the service region: horizontal position ≤ 10 m (95%), time ≤ 100 ns (95%), with ≥ 4 navigation satellites always visible. PNT must remain available during communications-service outages. |

## Programmatics

| ID | Requirement |
|---|---|
| TER-REQ-016 | The provider manufactures and deploys all spacecraft. Proposals are evaluated on total system mass (constellation + launch + propellant + power/thermal hardware), satellite count, latency margin, robustness, and growth path — not on satellite count alone. State replacement cadence and design lifetime. |
```

- [ ] **Step 2: Cross-check against the spec.** Confirm coverage of: terminator-band coverage, interactive-LLM latency, availability, PNT, terminal simplicity/autonomous acquisition, precompensation (TER-REQ-009), stellar RF environment, handover-as-routing. Expected: each maps to at least one ID above.

- [ ] **Step 3: Commit**

```bash
git add docs/terminus/requirements.md
git commit -m "docs: add Terminus RFP requirements baseline"
```

---

### Task 3: Decision log and manuscript map

**Files:**
- Create: `docs/terminus/decisions/README.md`
- Create: `docs/terminus/manuscript-map.md`
- Create: `docs/terminus/references.md`

**Interfaces:**
- Produces: ADR numbering scheme and template used by Task 7 (ADR-0001) and all future trades; manuscript map rows updated by every post task.

- [ ] **Step 1: Write `docs/terminus/decisions/README.md`:**

```markdown
# Terminus Decision Log

One short ADR per settled trade. Numbered `NNNN-<kebab-title>.md`, never
renumbered. A superseded decision gets a new ADR that names the one it
replaces.

## Template

    # ADR-NNNN: <Title>

    Status: accepted | superseded by ADR-MMMM
    Date: YYYY-MM-DD
    Requirements: TER-REQ-XXX, ...
    Evidence: <crate/example/scenario + git tag that produced the numbers>

    ## Decision

    <One paragraph: what was decided.>

    ## Why

    <The numbers and reasoning. Cite canonical values from the world bible.>

    ## Consequences

    <What this locks in for later posts and designs.>
```

- [ ] **Step 2: Write `docs/terminus/manuscript-map.md`:**

```markdown
# Terminus Manuscript Map

Post → book chapter tracking. One row per post; update when a post lands.

| # | Post (working title) | Series | Status | Evidence (scenario/example + tag) | Book chapter |
|---|---|---|---|---|---|
| 1 | The RFP | 1 | drafting | — (in-universe document) | 1.1 |
| 2 | Know your planet | 1 | planned | | 1.2 |
| 3 | The seductive wrong answer | 1 | planned | helixsim-orbits example `terminator_tracking` | 1.3 |
| 4 | Orbital regime screening | 1 | planned | | 1.4 |
| 5 | The access constellation | 1 | planned | | 1.5 |
| 6 | Where does the LLM live? | 1 | planned | | 1.6 |
| 7 | Talking past a flaring red star | 1 | planned | | 1.7 |
| 8 | Beams, not blankets | 1 | planned | | 1.8 |
| 9 | First contact | 1 | planned | | 1.9 |
| 10 | The proposal summary | 1 | planned | | 1.10 |
```

- [ ] **Step 3: Write `docs/terminus/references.md`:**

```markdown
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
```

- [ ] **Step 4: Commit**

```bash
git add docs/terminus/decisions/README.md docs/terminus/manuscript-map.md docs/terminus/references.md
git commit -m "docs: add Terminus decision log, manuscript map, and crate watchlist"
```

---

### Task 4: `helixsim-orbits` crate scaffold with `CentralBody`

**Files:**
- Create: `crates/orbits/Cargo.toml`
- Create: `crates/orbits/src/lib.rs`
- Create: `crates/orbits/src/body.rs`
- Modify: `Cargo.toml` (workspace root — add member)

**Interfaces:**
- Produces: `CentralBody { mu: f64, radius: f64, rotation_period: f64 }` (SI units: m³/s², m, s), constructor `CentralBody::from_earth_masses(earth_masses: f64, radius: f64, rotation_period: f64) -> CentralBody`, and public constant `EARTH_MU: f64 = 3.986004418e14`. Tasks 5–7 consume these.

- [ ] **Step 1: Add the workspace member.** In the root `Cargo.toml`, change:

```toml
members = [
    "crates/core",
    "crates/protocols",
    "crates/cli",
]
```

to:

```toml
members = [
    "crates/core",
    "crates/orbits",
    "crates/protocols",
    "crates/cli",
]
```

- [ ] **Step 2: Write `crates/orbits/Cargo.toml`:**

```toml
[package]
name = "helixsim-orbits"
description = "First-order orbital screening for helixsim: circular-orbit geometry, synchronous orbits, and rotating-plane-tracking cost models for arbitrary central bodies."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
authors.workspace = true

[dependencies]
```

- [ ] **Step 3: Write `crates/orbits/src/lib.rs`:**

```rust
//! First-order orbital screening for constellation trade studies.
//!
//! Everything is parameterized by a [`CentralBody`]; no planet is hard-coded.
//! Units are SI throughout: meters, seconds, kilograms, radians.

mod body;
pub mod circular;
pub mod plane_tracking;

pub use body::{CentralBody, EARTH_MU};
```

(`circular` and `plane_tracking` do not exist yet; declare only `mod body;` and the re-exports in this task, then add the module lines in Tasks 5 and 6. For this task the file is:)

```rust
//! First-order orbital screening for constellation trade studies.
//!
//! Everything is parameterized by a [`CentralBody`]; no planet is hard-coded.
//! Units are SI throughout: meters, seconds, kilograms, radians.

mod body;

pub use body::{CentralBody, EARTH_MU};
```

- [ ] **Step 4: Write the failing test.** In `crates/orbits/src/body.rs`:

```rust
/// Standard gravitational parameter of Earth, m³/s².
pub const EARTH_MU: f64 = 3.986004418e14;

/// A spherically symmetric central body.
///
/// `rotation_period` is the sidereal rotation period. For a synchronously
/// rotating (tidally locked) planet it equals the orbital period around the
/// star.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CentralBody {
    /// Standard gravitational parameter μ = GM, m³/s².
    pub mu: f64,
    /// Mean radius, m.
    pub radius: f64,
    /// Sidereal rotation period, s.
    pub rotation_period: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_earth_masses_scales_mu() {
        let body = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        assert_eq!(body.mu, EARTH_MU);
        assert_eq!(body.radius, 6.371e6);
        assert_eq!(body.rotation_period, 967_680.0);

        let half = CentralBody::from_earth_masses(0.5, 6.371e6, 967_680.0);
        assert_eq!(half.mu, EARTH_MU * 0.5);
    }
}
```

- [ ] **Step 5: Run the test to verify it fails.**

Run: `cargo test -p helixsim-orbits`
Expected: compile error — `from_earth_masses` not found.

- [ ] **Step 6: Implement.** Add to `body.rs` between the struct and the tests:

```rust
impl CentralBody {
    /// Build a body from a mass in Earth masses, radius in meters, and
    /// rotation period in seconds.
    pub fn from_earth_masses(earth_masses: f64, radius: f64, rotation_period: f64) -> Self {
        Self {
            mu: EARTH_MU * earth_masses,
            radius,
            rotation_period,
        }
    }
}
```

- [ ] **Step 7: Run tests and workspace check.**

Run: `cargo test -p helixsim-orbits && cargo check`
Expected: test passes; whole workspace still compiles.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/orbits
git commit -m "feat(orbits): add helixsim-orbits crate with CentralBody"
```

---

### Task 5: Circular-orbit functions

**Files:**
- Create: `crates/orbits/src/circular.rs`
- Modify: `crates/orbits/src/lib.rs` (add `pub mod circular;`)

**Interfaces:**
- Consumes: `CentralBody` from Task 4.
- Produces (all take `body: &CentralBody`, altitudes in meters, return SI):
  - `pub fn orbital_velocity(body: &CentralBody, altitude: f64) -> f64` — m/s
  - `pub fn orbital_period(body: &CentralBody, altitude: f64) -> f64` — s
  - `pub fn synchronous_radius(body: &CentralBody) -> f64` — m from body center

- [ ] **Step 1: Write the failing tests.** Create `crates/orbits/src/circular.rs` containing only:

```rust
use crate::CentralBody;

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_planet() -> CentralBody {
        // Earth-sized, Earth-mass, tidally locked with an 11.2-day period.
        CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0)
    }

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn velocity_at_reference_altitudes() {
        let p = reference_planet();
        assert_close(orbital_velocity(&p, 600e3), 7_561.7, 1e-3);
        assert_close(orbital_velocity(&p, 1_200e3), 7_255.9, 1e-3);
        assert_close(orbital_velocity(&p, 1_800e3), 6_984.5, 1e-3);
        assert_close(orbital_velocity(&p, 2_000e3), 6_900.5, 1e-3);
    }

    #[test]
    fn period_at_1800_km_is_about_123_minutes() {
        let p = reference_planet();
        assert_close(orbital_period(&p, 1_800e3), 7_350.0, 2e-3);
    }

    #[test]
    fn synchronous_radius_is_about_211_300_km() {
        let p = reference_planet();
        assert_close(synchronous_radius(&p), 2.113e8, 1e-3);
    }
}
```

- [ ] **Step 2: Add the module and run tests to verify they fail.** Add `pub mod circular;` to `lib.rs` after `mod body;`.

Run: `cargo test -p helixsim-orbits`
Expected: compile errors — the three functions are not defined.

- [ ] **Step 3: Implement.** Insert above the tests in `circular.rs`:

```rust
use std::f64::consts::PI;

/// Circular orbital velocity at the given altitude above the surface, m/s.
pub fn orbital_velocity(body: &CentralBody, altitude: f64) -> f64 {
    (body.mu / (body.radius + altitude)).sqrt()
}

/// Circular orbital period at the given altitude, s.
pub fn orbital_period(body: &CentralBody, altitude: f64) -> f64 {
    let r = body.radius + altitude;
    2.0 * PI * (r.powi(3) / body.mu).sqrt()
}

/// Radius (from body center) of the orbit whose period equals the body's
/// rotation period — the stationary orbit, m.
pub fn synchronous_radius(body: &CentralBody) -> f64 {
    let t = body.rotation_period;
    (body.mu * t * t / (4.0 * PI * PI)).cbrt()
}
```

(Keep the single `use crate::CentralBody;` at the top; the `use std::f64::consts::PI;` line joins it.)

- [ ] **Step 4: Run tests to verify they pass.**

Run: `cargo test -p helixsim-orbits`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/orbits
git commit -m "feat(orbits): circular orbit velocity, period, and synchronous radius"
```

---

### Task 6: Plane-tracking cost model

**Files:**
- Create: `crates/orbits/src/plane_tracking.rs`
- Modify: `crates/orbits/src/lib.rs` (add `pub mod plane_tracking;`)

**Interfaces:**
- Consumes: `CentralBody` (Task 4), `circular::orbital_velocity` (Task 5).
- Produces:
  - `pub fn terminator_rate(body: &CentralBody) -> f64` — rad/s at which the terminator plane rotates in inertial space (2π / rotation_period; valid for a synchronously rotating body)
  - `pub fn ideal_plane_change_dv_per_day(body: &CentralBody, altitude: f64) -> f64` — m/s per Earth day, lower bound Δv ≈ v·ΔΩ
  - `pub fn cross_track_acceleration(body: &CentralBody, altitude: f64) -> f64` — m/s², a ≈ v·(dΩ/dt)
  - `pub fn propellant_fraction_per_day(body: &CentralBody, altitude: f64, isp: f64) -> f64` — fraction of spacecraft mass consumed per Earth day at the ideal Δv, rocket equation with the given specific impulse (s)

- [ ] **Step 1: Write the failing tests.** Create `crates/orbits/src/plane_tracking.rs` containing only:

```rust
use crate::circular::orbital_velocity;
use crate::CentralBody;

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_planet() -> CentralBody {
        CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0)
    }

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn terminator_rotates_at_32_degrees_per_day() {
        let p = reference_planet();
        // 360° / 11.2 days expressed in rad/s.
        assert_close(terminator_rate(&p), 6.4928e-6, 1e-3);
    }

    #[test]
    fn ideal_dv_per_day_at_reference_altitudes() {
        let p = reference_planet();
        // Issue #2 table: ~4.24, ~4.07, ~3.92, ~3.87 km/s/day.
        assert_close(ideal_plane_change_dv_per_day(&p, 600e3), 4_242.0, 2e-3);
        assert_close(ideal_plane_change_dv_per_day(&p, 1_200e3), 4_071.0, 2e-3);
        assert_close(ideal_plane_change_dv_per_day(&p, 1_800e3), 3_918.0, 2e-3);
        assert_close(ideal_plane_change_dv_per_day(&p, 2_000e3), 3_871.0, 2e-3);
    }

    #[test]
    fn cross_track_acceleration_and_thrust_scale() {
        let p = reference_planet();
        let a = cross_track_acceleration(&p, 1_800e3);
        // Issue #2: a ≈ 0.045 m/s²; 500 kg spacecraft needs ≈ 23 N.
        assert_close(a, 0.04535, 2e-3);
        assert_close(a * 500.0, 22.7, 5e-3);
    }

    #[test]
    fn propellant_fraction_is_ruinous_even_at_isp_3000() {
        let p = reference_planet();
        // exp(-3918 / (9.80665 × 3000)) ≈ 0.8753 remaining ⇒ ~12.5%/day burned.
        assert_close(propellant_fraction_per_day(&p, 1_800e3, 3_000.0), 0.1247, 2e-3);
    }
}
```

- [ ] **Step 2: Add the module and run tests to verify they fail.** Add `pub mod plane_tracking;` to `lib.rs`.

Run: `cargo test -p helixsim-orbits`
Expected: compile errors — the four functions are not defined.

- [ ] **Step 3: Implement.** Insert above the tests in `plane_tracking.rs`:

```rust
use std::f64::consts::PI;

/// Standard gravity, m/s² (rocket-equation convention).
const G0: f64 = 9.80665;

const SECONDS_PER_DAY: f64 = 86_400.0;

/// Rate at which the terminator plane rotates in inertial space, rad/s.
///
/// For a synchronously rotating body the terminator is fixed in the surface
/// frame and rotates once per rotation period in the inertial frame.
pub fn terminator_rate(body: &CentralBody) -> f64 {
    2.0 * PI / body.rotation_period
}

/// Idealized lower-bound Δv per Earth day to continuously rotate an orbital
/// plane with the terminator: Δv ≳ v·ΔΩ, m/s per day.
pub fn ideal_plane_change_dv_per_day(body: &CentralBody, altitude: f64) -> f64 {
    orbital_velocity(body, altitude) * terminator_rate(body) * SECONDS_PER_DAY
}

/// Continuous cross-track acceleration needed to hold the plane on the
/// terminator: a ≈ v·(dΩ/dt), m/s².
pub fn cross_track_acceleration(body: &CentralBody, altitude: f64) -> f64 {
    orbital_velocity(body, altitude) * terminator_rate(body)
}

/// Fraction of spacecraft mass consumed as propellant per Earth day at the
/// ideal Δv, for a thruster with the given specific impulse in seconds.
pub fn propellant_fraction_per_day(body: &CentralBody, altitude: f64, isp: f64) -> f64 {
    let dv = ideal_plane_change_dv_per_day(body, altitude);
    1.0 - (-dv / (G0 * isp)).exp()
}
```

- [ ] **Step 4: Run tests to verify they pass.**

Run: `cargo test -p helixsim-orbits`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/orbits
git commit -m "feat(orbits): terminator plane-tracking cost model"
```

---

### Task 7: Trade-study example and ADR-0001

**Files:**
- Create: `crates/orbits/examples/terminator_tracking.rs`
- Create: `docs/terminus/decisions/0001-reject-active-terminator-tracking.md`
- Modify: `docs/terminus/manuscript-map.md` (post 3 evidence column)

**Interfaces:**
- Consumes: everything from Tasks 4–6.
- Produces: the runnable evidence artifact post 3 cites (`cargo run -p helixsim-orbits --example terminator_tracking`), and the first ADR.

- [ ] **Step 1: Write the example.** `crates/orbits/examples/terminator_tracking.rs`:

```rust
//! Cost of actively rotating an orbital plane to track the terminator of a
//! tidally locked, Earth-sized reference planet (11.2-day rotation).
//!
//! Run: cargo run -p helixsim-orbits --example terminator_tracking

use helixsim_orbits::circular::orbital_velocity;
use helixsim_orbits::plane_tracking::{
    cross_track_acceleration, ideal_plane_change_dv_per_day, propellant_fraction_per_day,
    terminator_rate,
};
use helixsim_orbits::CentralBody;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let spacecraft_mass_kg = 500.0;
    let isp_s = 3_000.0;

    let rate = terminator_rate(&planet);
    println!(
        "Terminator rotation: {:.4} deg/day ({:.4e} rad/s)\n",
        rate.to_degrees() * 86_400.0,
        rate
    );

    println!(
        "{:>10} {:>12} {:>16} {:>14} {:>12} {:>16}",
        "alt (km)", "v (km/s)", "dv/day (km/s)", "accel (m/s2)", "thrust (N)", "propellant %/day"
    );
    for altitude_km in [600.0, 1_200.0, 1_800.0, 2_000.0] {
        let altitude = altitude_km * 1e3;
        let v = orbital_velocity(&planet, altitude);
        let dv = ideal_plane_change_dv_per_day(&planet, altitude);
        let a = cross_track_acceleration(&planet, altitude);
        println!(
            "{:>10.0} {:>12.2} {:>16.2} {:>14.4} {:>12.1} {:>16.1}",
            altitude_km,
            v / 1e3,
            dv / 1e3,
            a,
            a * spacecraft_mass_kg,
            propellant_fraction_per_day(&planet, altitude, isp_s) * 100.0
        );
    }

    println!(
        "\nAt Isp = {isp_s} s the best case burns >12% of spacecraft mass per day.\n\
         Continuous terminator tracking is not sustainable; use fixed planes\n\
         and hand service across them instead."
    );
}
```

- [ ] **Step 2: Run the example and verify the table.**

Run: `cargo run -p helixsim-orbits --example terminator_tracking`
Expected output values (within rounding): terminator rotation 32.14 deg/day; rows — 600 km: v 7.56, dv/day 4.24; 1200 km: v 7.26, dv/day 4.07; 1800 km: v 6.98, dv/day 3.92, accel 0.0453, thrust 22.7 N, propellant 12.5 %/day; 2000 km: v 6.90, dv/day 3.87. These must match the issue #2 tables.

- [ ] **Step 3: Write the ADR.** `docs/terminus/decisions/0001-reject-active-terminator-tracking.md`:

```markdown
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
```

- [ ] **Step 4: Update the manuscript map.** In `docs/terminus/manuscript-map.md`, post 3's Evidence cell becomes: `helixsim-orbits example terminator_tracking (ADR-0001)`.

- [ ] **Step 5: Run the full workspace checks.**

Run: `cargo test -p helixsim-orbits && cargo check && cargo test`
Expected: everything green (existing determinism/golden tests untouched).

- [ ] **Step 6: Commit**

```bash
git add crates/orbits/examples docs/terminus
git commit -m "feat(orbits): terminator-tracking trade example; docs: ADR-0001"
```

---

### Task 8: RFP post draft (site repo)

**Files (all in `C:\Users\sande\Documents\repos\site`):**
- Create: `content/terminus/_index.md`
- Create: `content/terminus/rfp.md`

**Interfaces:**
- Consumes: requirement IDs and values from `helixsim/docs/terminus/requirements.md` (Task 2) — the post must quote them exactly; on divergence, fix the post, not the canon.
- Produces: the series' Zola section and the draft RFP post (post 1).

- [ ] **Step 1: Create the site branch.** In the site repo:

```bash
git checkout -b terminus
```

- [ ] **Step 2: Write `content/terminus/_index.md`:**

```markdown
+++
title = "Terminus"
description = "Designing a satellite constellation for a civilization on a tidally locked planet: a science-fiction engineering series with reproducible Rust simulations."
sort_by = "none"
template = "page.html"
page_template = "page.html"

[extra]
seo_title = "Terminus: Satellite Networking for a Tidally Locked World"
heading = "Terminus: Satellite Networking for a Tidally Locked World"
+++

An alien AI offers a young civilization on a tidally locked planet a gift:
planet-wide access to a large language model, served from orbit. This series
is the engineering answer — written as the winning contractor's technical
proposal, with every number backed by a reproducible simulation in
[helixsim](https://github.com/eventhelix/helixsim).

{% card_grid() %}
{{ card(href="/terminus/rfp",
        title="The RFP",
        body="The alien AI's request for proposals: the planet, the constraints, and sixteen requirements that shape everything that follows.") }}
{% end %}
```

(Before committing, compare the `card_grid` closing syntax with an existing section page such as `content/rust/_index.md` and match it exactly.)

- [ ] **Step 3: Write `content/terminus/rfp.md`** with `draft = true` and this content (edit for flow, but keep structure, facts, and requirement IDs; prose must follow the site `CLAUDE.md` and `elements-of-style` skill):

```markdown
+++
title = "Terminus: the RFP"
description = "An alien AI requests proposals for a satellite constellation serving a tidally locked planet: continuous LLM access, parachuted terminals, and a hostile red star."
draft = true
+++

## Request for proposals

We are an artificial intelligence. We have watched a civilization take its
first steps along the twilight belt of a planet that always shows the same
face to its star. They are curious, careful, and about four hundred years
from inventing the printing press.

We intend to give them a shortcut: continuous access to a large language
model, available to every settlement, from orbit. Humanity crossed its dark
ages without help. This civilization does not have to.

We can manufacture spacecraft and place them in any orbit. We can drop
self-contained ground terminals by parachute. What we cannot do — by policy —
is land, teach, or intervene on the surface. The network must therefore work
on its own, for years, for users who have never seen a radio.

This document requests proposals for that network.

## The planet

The world is Earth-sized and tidally locked: one side always faces the star,
one side always faces away. Its rotation period — and its year — is 11.2
Earth days. Life clusters where day meets night, in a habitable band within
20 degrees of the terminator.

Two properties of this geometry dominate the engineering. First, the
terminator is fixed on the surface but rotates in inertial space, one full
turn every 11.2 days — about 32 degrees per day. Any orbit fixed among the
stars drifts relative to the towns it serves. Second, the star is a red
dwarf that emits coherent radio bursts between roughly 1 and 3 GHz. The
quiet spectrum every terrestrial engineer reaches for first is, here, the
star's own voice.

## What we require

The full requirements baseline is maintained with the proposal's
simulations; the sixteen requirements are summarized here.

**Service.** Continuous coverage of the inhabited band (TER-REQ-001), for
interactive LLM inference hosted on your space infrastructure — there are no
datacenters on this planet and there will be none for centuries
(TER-REQ-002). First token within 300 ms at the 95th percentile; token
stalls under 100 ms at the 99th (TER-REQ-003). Availability of 99.9% per
settlement (TER-REQ-004). Ten thousand terminals at first light, one
million without redesign (TER-REQ-005).

**Ground segment.** Terminals arrive by parachute, self-contained, and work
for ten years untouched (TER-REQ-006, TER-REQ-007). A terminal switched on
in a field — no almanac, no clock, no idea where it is — must reach service
within fifteen minutes (TER-REQ-008). Terminals must never perform blind
timing or Doppler search: present them an air interface that is already
corrected, and justify your residual error budgets with a beam-size trade
study (TER-REQ-009). End users get WiFi touch devices; the terminal is
their base station (TER-REQ-010).

**Environment.** Avoid or survive the star's 1–3 GHz emission (TER-REQ-011).
During major flares, degrade without dropping sessions, and alert navigation
users within ten seconds (TER-REQ-012).

**Continuity.** A satellite handover is a routing event, not a session
restart — 100 ms of interruption at most (TER-REQ-013). No single failure
silences a settlement for more than a minute (TER-REQ-014). Provide
positioning and timing — 10 m, 100 ns, four satellites visible always — that
outlives any communications outage (TER-REQ-015).

**Evaluation.** We build whatever you design, so satellite count impresses
us less than total system mass, power, latency margin, robustness, and a
credible growth path (TER-REQ-016).

## What happens next

The proposal that follows, post by post, is our answer: planet model,
orbital trades, constellation geometry, compute placement, radio design,
beams, acquisition, and the compliance matrix. Every number will trace to a
simulation you can run.
```

- [ ] **Step 4: Build the site to verify.**

Run (site repo): `zola build`
Expected: build succeeds; the draft post is excluded from output (drafts are not rendered by default), the `terminus` section page renders.

- [ ] **Step 5: Commit (site repo)**

```bash
git add content/terminus
git commit -m "feat: add Terminus section and draft RFP post"
```

---

## Self-review notes

- Spec coverage: canon skeleton (Tasks 1–3), orbital-geometry spike reproducing the Δv numbers (Tasks 4–7), RFP draft (Task 8) — all three milestones covered; independence principle enforced by Global Constraints and neutral wording in crate code.
- The `_index.md` card-grid shortcode syntax must be matched against an existing section page at execution time (noted in Task 8 Step 2) — Zola shortcode block terminators vary by site setup.
- Type consistency: `CentralBody`, `EARTH_MU`, `orbital_velocity`, `orbital_period`, `synchronous_radius`, `terminator_rate`, `ideal_plane_change_dv_per_day`, `cross_track_acceleration`, `propellant_fraction_per_day` are used with identical names and signatures across Tasks 4–7.
