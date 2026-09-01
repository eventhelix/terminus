// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Multi-satellite coverage of a fixed ground band by polar-orbit
//! constellations, evaluated in the planet-fixed frame.
//!
//! Frame convention (planet-fixed, for a synchronously rotating planet):
//! `z` is the polar axis, the star lies along `-x` forever, so the
//! terminator is the great circle in the `x = 0` plane. The band of ground
//! points to serve straddles that circle. Orbits are circular and polar;
//! in this frame their planes regress at the planet's rotation rate.

use crate::CentralBody;

/// A symmetric fleet of circular polar orbits: `planes` orbital planes with
/// ascending nodes spread evenly over 180°, `sats_per_plane` satellites
/// evenly phased in each, all at the same `altitude` (m).
/// `interplane_phase` (rad) staggers the phasing from one plane to the next.
#[derive(Debug, Clone, Copy)]
pub struct PolarConstellation {
    pub altitude: f64,
    pub planes: usize,
    pub sats_per_plane: usize,
    pub interplane_phase: f64,
}

/// Position (m, planet-fixed frame) of a satellite in a circular polar
/// orbit with ascending node `raan` (rad, at t = 0) and along-orbit phase
/// `theta0` (rad, at t = 0), evaluated at time `t` (s).
pub fn polar_sat_position(
    body: &CentralBody,
    altitude: f64,
    raan: f64,
    theta0: f64,
    t: f64,
) -> [f64; 3] {
    let r = body.radius + altitude;
    let n = 2.0 * std::f64::consts::PI / crate::circular::orbital_period(body, altitude);
    let spin = 2.0 * std::f64::consts::PI / body.rotation_period;
    let theta = theta0 + n * t;
    let node = raan - spin * t;
    [
        r * theta.cos() * node.cos(),
        r * theta.cos() * node.sin(),
        r * theta.sin(),
    ]
}

/// Elevation angle (rad) of a satellite at `sat` (m, planet-fixed) as seen
/// from the ground point in unit direction `ground_unit`.
pub fn elevation(body: &CentralBody, ground_unit: [f64; 3], sat: [f64; 3]) -> f64 {
    let g = ground_unit;
    let rho = [
        sat[0] - body.radius * g[0],
        sat[1] - body.radius * g[1],
        sat[2] - body.radius * g[2],
    ];
    let norm = (rho[0] * rho[0] + rho[1] * rho[1] + rho[2] * rho[2]).sqrt();
    ((rho[0] * g[0] + rho[1] * g[1] + rho[2] * g[2]) / norm).asin()
}

/// Unit direction of a band ground point: `azimuth` (rad) runs around the
/// terminator great circle, `offset` (rad) displaces the point toward the
/// night side (positive `x`).
pub fn band_point(azimuth: f64, offset: f64) -> [f64; 3] {
    [
        offset.sin(),
        offset.cos() * azimuth.cos(),
        offset.cos() * azimuth.sin(),
    ]
}

/// Number of the constellation's satellites at or above `min_elevation`
/// (rad) as seen from `ground_unit` at time `t`.
pub fn visible_count(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    min_elevation: f64,
    t: f64,
) -> usize {
    let mut count = 0;
    for k in 0..c.planes {
        let raan = k as f64 * std::f64::consts::PI / c.planes as f64;
        for j in 0..c.sats_per_plane {
            let theta0 = j as f64 * 2.0 * std::f64::consts::PI / c.sats_per_plane as f64
                + k as f64 * c.interplane_phase;
            let sat = polar_sat_position(body, c.altitude, raan, theta0, t);
            if elevation(body, ground_unit, sat) >= min_elevation {
                count += 1;
            }
        }
    }
    count
}

impl PolarConstellation {
    /// Along-orbit phase offset (rad) applied to every satellite of plane `k`
    /// under the uniform [`Self::interplane_phase`] stagger.
    pub fn plane_phase(&self, k: usize) -> f64 {
        k as f64 * self.interplane_phase
    }

    /// The uniform stagger written out as one phase per plane, for the
    /// functions that take an explicit `phases` slice.
    pub fn uniform_phases(&self) -> Vec<f64> {
        (0..self.planes).map(|k| self.plane_phase(k)).collect()
    }

    /// Ascending node (rad) of plane `k` at time `t`, in the planet-fixed
    /// frame where it regresses at the planet's spin rate.
    pub fn node(&self, body: &CentralBody, k: usize, t: f64) -> f64 {
        k as f64 * std::f64::consts::PI / self.planes as f64
            - 2.0 * std::f64::consts::PI / body.rotation_period * t
    }

    /// Planet-central half-angle (rad) of one satellite's footprint.
    pub fn footprint_half_angle(&self, body: &CentralBody, min_elevation: f64) -> f64 {
        crate::coverage::footprint_radius(body, self.altitude, min_elevation) / body.radius
    }
}

/// Satellites of plane `k` visible from `ground_unit` at time `t`, given that
/// plane's along-orbit `phase` (rad).
///
/// Solved rather than enumerated. A plane's satellites all ride one great
/// circle, so the along-orbit angles from which the ground point lies within
/// the footprint half-angle `lambda` form a single arc: the plane passes the
/// ground point at closest approach `d` (the cross-track distance, where
/// `cos d` is the length of the ground point's projection into the plane),
/// and coverage runs `+/- acos(cos lambda / cos d)` about that. Counting the
/// evenly spaced satellites inside that arc is then arithmetic rather than a
/// loop over spacecraft, which is what makes randomized phase sweeps
/// affordable.
///
/// Returns 0 when the plane never comes within `lambda` of the ground point.
/// No number of satellites in that ring can help, which is the wall the
/// strict duty-ring design runs into; see [`crate::duty`].
pub fn plane_visible_count(
    body: &CentralBody,
    c: &PolarConstellation,
    k: usize,
    ground_unit: [f64; 3],
    phase: f64,
    min_elevation: f64,
    t: f64,
) -> usize {
    let lambda = c.footprint_half_angle(body, min_elevation);
    let node = c.node(body, k, t);
    let a = ground_unit[0] * node.cos() + ground_unit[1] * node.sin();
    let b = ground_unit[2];
    let cos_d = (a * a + b * b).sqrt();
    // Pad the reach test before using it to reject a plane or to bound the
    // arc. The padded angle is far below any real geometry (1e-9 rad is
    // millimetres of ground track) but comfortably above the float error
    // between the central-angle and elevation forms of the same test, so a
    // satellite sitting on the mask is never screened out before it is
    // measured.
    let cos_reach = (lambda + 1e-9).cos();
    if cos_d <= cos_reach {
        return 0;
    }
    let w = (cos_reach / cos_d).clamp(-1.0, 1.0).acos();
    let psi = b.atan2(a);
    let n = 2.0 * std::f64::consts::PI / crate::circular::orbital_period(body, c.altitude);
    let step = 2.0 * std::f64::consts::PI / c.sats_per_plane as f64;
    // A satellite's current along-orbit angle is `j*step + phase + n*t`, so it
    // is visible when `j*step - U` falls in the arc, with `U = psi - phase -
    // n*t`. Splitting `U = m*step + u` puts the arc in the shifted index
    // `i = j - m`; `m` must be carried back when naming the satellite, since it
    // decides *which* ones are candidates even though it cannot change how
    // many.
    let big_u = psi - phase - n * t;
    let m = (big_u / step).floor();
    let u = big_u - m * step;
    let m = m as i64;

    // The arc narrows the field to a couple of candidates; it does not decide
    // them. Deciding by arithmetic alone disagrees with `elevation` for a
    // satellite sitting exactly on the mask, and a coverage *minimum* is
    // precisely where those cases fall - an over-count of one there turns a
    // real gap into apparent coverage. So test each candidate with the same
    // elevation comparison the brute-force count uses, and widen the candidate
    // range by one slot on each side so no boundary case is missed.
    let lo = ((u - w) / step).ceil() as i64 - 1;
    let hi = ((u + w) / step).floor() as i64 + 1;
    // Each candidate must be a distinct satellite: if the widened range wraps
    // past a full ring, fall back to testing every one of them.
    let span = hi - lo + 1;
    let (lo, hi) = if span >= c.sats_per_plane as i64 {
        (0, c.sats_per_plane as i64 - 1)
    } else {
        (lo, hi)
    };
    let raan = k as f64 * std::f64::consts::PI / c.planes as f64;
    let mut count = 0;
    for i in lo..=hi {
        let j = (i + m).rem_euclid(c.sats_per_plane as i64);
        // Compute theta0 exactly as `visible_count` does. `j * step` and
        // `j * 2pi / sats` differ in the last bit, which is enough to flip a
        // satellite sitting precisely on the mask and turn a real gap into
        // apparent coverage.
        let theta0 = j as f64 * 2.0 * std::f64::consts::PI / c.sats_per_plane as f64 + phase;
        let sat = polar_sat_position(body, c.altitude, raan, theta0, t);
        if elevation(body, ground_unit, sat) >= min_elevation {
            count += 1;
        }
    }
    count
}

/// How the rings' along-orbit phases relate to one another.
///
/// A ring's phase is a single angle added to every satellite in it, so these
/// modes do not change how satellites are spaced *within* a ring — only where
/// each ring sits relative to its neighbours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseMode {
    /// Every ring in step. The published baseline, and what
    /// `interplane_phase = 0` produces.
    Aligned,
    /// Each ring offset half an in-plane slot from the last, so neighbouring
    /// rings' satellites interleave. This is the phasing a cellular instinct
    /// reaches for — the triangular lattice that covers a plane with the
    /// fewest cells — and a Walker phasing factor. The
    /// `phasing_options` example is where it fails to buy satellites.
    HalfSlot,
    /// An independent, arbitrary offset per ring, which is what an
    /// uncoordinated launch campaign actually produces. See
    /// [`crate::station_keeping`] for why a campaign cannot target a ring's
    /// phase relative to its neighbours.
    Random,
}

/// The 64-bit LCG behind [`PhaseMode::Random`].
///
/// Deliberately the plainest possible generator, and deliberately part of the
/// library rather than a detail of one example: the constellation explorer on
/// the web draws its phase vector by reimplementing exactly this recurrence,
/// so a phasing anyone can see in the browser is one `cargo run` away from
/// being reproduced here. Any change to the constants or to the 53-bit
/// extraction changes what that scene shows.
struct PhaseRng(u64);

impl PhaseRng {
    /// Next draw in `[0, 1)`, as the top 53 bits of the new state.
    fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The seed the constellation explorer draws its random phasing with.
///
/// Fixed so that the scene is the same for every reader and every reload, and
/// so the vector it shows can be printed here rather than described.
pub const EXPLORER_PHASE_SEED: u64 = 0x51E7_2026;

/// One along-orbit phase (rad) per ring under `mode`.
///
/// `seed` is used only by [`PhaseMode::Random`], and the generator is
/// constructed fresh per call — so a given `(mode, planes, sats_per_plane,
/// seed)` always yields the same vector, whatever else has drawn before it.
/// Feed the result to [`visible_count_with_phases`],
/// [`crate::activation::satellite_units`], or [`crate::handover::best_visible`].
pub fn plane_phases(mode: PhaseMode, planes: usize, sats_per_plane: usize, seed: u64) -> Vec<f64> {
    let slot = 2.0 * std::f64::consts::PI / sats_per_plane as f64;
    let mut rng = PhaseRng(seed);
    (0..planes)
        .map(|k| match mode {
            PhaseMode::Aligned => 0.0,
            PhaseMode::HalfSlot => k as f64 * slot / 2.0,
            PhaseMode::Random => rng.next_f64() * slot,
        })
        .collect()
}

/// Total satellites visible at `t`, with an independent along-orbit phase for
/// each plane. `phases` must have one entry per plane.
///
/// Uncoordinated phasing is the honest model: a launch campaign that cannot
/// target a plane's phase relative to its neighbours (see
/// [`crate::station_keeping`]) produces arbitrary offsets, and a coverage
/// claim has to hold for all of them.
pub fn visible_count_with_phases(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    phases: &[f64],
    min_elevation: f64,
    t: f64,
) -> usize {
    (0..c.planes)
        .map(|k| plane_visible_count(body, c, k, ground_unit, phases[k], min_elevation, t))
        .sum()
}

/// Coverage of the band over time: fewest and average satellites usable
/// from any sampled band point at any sampled instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverageStats {
    pub min_visible: usize,
    pub mean_visible: f64,
}

/// Sweep one full band (`band_halfwidth` rad to either side of the
/// terminator, sampled at `azimuth_samples` points around the circle and
/// the two band edges plus the center line) over `duration` (s) in `step`
/// (s) increments.
pub fn band_coverage(
    body: &CentralBody,
    c: &PolarConstellation,
    band_halfwidth: f64,
    min_elevation: f64,
    duration: f64,
    step: f64,
    azimuth_samples: usize,
) -> CoverageStats {
    let mut min_visible = usize::MAX;
    let mut total: f64 = 0.0;
    let mut samples: u64 = 0;
    let offsets = [-band_halfwidth, 0.0, band_halfwidth];
    let mut t = 0.0;
    while t < duration {
        for i in 0..azimuth_samples {
            let azimuth = i as f64 * 2.0 * std::f64::consts::PI / azimuth_samples as f64;
            for offset in offsets {
                let n = visible_count(body, c, band_point(azimuth, offset), min_elevation, t);
                min_visible = min_visible.min(n);
                total += n as f64;
                samples += 1;
            }
        }
        t += step;
    }
    CoverageStats {
        min_visible,
        mean_visible: total / samples as f64,
    }
}

#[cfg(test)]
mod tests {
    /// The web constellation explorer draws its phasing by reimplementing
    /// `PhaseRng` in JavaScript, where there is no u64 and the recurrence has
    /// to be run in `BigInt`. Pinning the vector here rather than describing
    /// it gives that port something to be checked against: if these six
    /// numbers ever change, the scene readers see changes with them.
    #[test]
    fn the_explorer_phase_vector_is_fixed() {
        let ph = plane_phases(PhaseMode::Random, 6, 12, EXPLORER_PHASE_SEED);
        let expected = [
            0.05075556215960516,
            0.2781275504907797,
            0.3103694398582766,
            0.2622527495817053,
            0.2760803509942688,
            0.014762893105412532,
        ];
        for (k, (got, want)) in ph.iter().zip(expected).enumerate() {
            assert!(
                (got - want).abs() < 1e-15,
                "ring {k}: {got} is not the pinned {want}"
            );
        }
    }

    /// A fresh generator per call, so a vector does not depend on what drew
    /// before it -- the property that lets one row of `phasing_options` be
    /// re-run on its own.
    #[test]
    fn a_phase_vector_depends_only_on_its_arguments() {
        let once = plane_phases(PhaseMode::Random, 6, 12, EXPLORER_PHASE_SEED);
        let _ = plane_phases(PhaseMode::Random, 8, 9, 12345);
        let again = plane_phases(PhaseMode::Random, 6, 12, EXPLORER_PHASE_SEED);
        assert_eq!(once, again);
    }

    /// Aligned is the `interplane_phase = 0` baseline; half-slot is the
    /// uniform stagger written out. Both must agree with the struct field, or
    /// the explorer and the sizing examples are modelling different fleets.
    #[test]
    fn the_uniform_modes_agree_with_interplane_phase() {
        let slot = 2.0 * std::f64::consts::PI / 12.0;
        assert_eq!(plane_phases(PhaseMode::Aligned, 6, 12, 0), vec![0.0; 6]);
        let c = PolarConstellation {
            altitude: 2_200e3,
            planes: 6,
            sats_per_plane: 12,
            interplane_phase: slot / 2.0,
        };
        assert_eq!(
            plane_phases(PhaseMode::HalfSlot, 6, 12, 0),
            c.uniform_phases()
        );
    }

    use super::*;
    use crate::coverage::footprint_radius;
    use std::f64::consts::PI;

    const MIN_ELEVATION: f64 = 25.0 * std::f64::consts::PI / 180.0;

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
    fn overhead_satellite_is_at_zenith() {
        let p = reference_planet();
        let sat = [0.0, p.radius + 1_800e3, 0.0];
        let e = elevation(&p, [0.0, 1.0, 0.0], sat);
        assert_close(e, std::f64::consts::FRAC_PI_2, 1e-9);
    }

    #[test]
    fn satellite_at_coverage_edge_sits_at_min_elevation() {
        // A satellite displaced from the ground point by exactly the
        // coverage half-angle from coverage.rs must appear at exactly the
        // minimum elevation — the two modules must agree.
        let p = reference_planet();
        let lambda = footprint_radius(&p, 1_800e3, MIN_ELEVATION) / p.radius;
        let r = p.radius + 1_800e3;
        let sat = [0.0, r * lambda.cos(), r * lambda.sin()];
        let e = elevation(&p, [0.0, 1.0, 0.0], sat);
        assert_close(e, MIN_ELEVATION, 1e-9);
    }

    #[test]
    fn polar_position_starts_at_node_and_regresses_with_spin() {
        let p = reference_planet();
        let r = p.radius + 1_800e3;
        let pos = polar_sat_position(&p, 1_800e3, 0.0, 0.0, 0.0);
        assert_close(pos[0], r, 1e-12);

        // A quarter orbit later the satellite is over the pole regardless
        // of node drift.
        let quarter = crate::circular::orbital_period(&p, 1_800e3) / 4.0;
        let pos = polar_sat_position(&p, 1_800e3, 0.0, 0.0, quarter);
        assert_close(pos[2], r, 1e-9);
    }

    #[test]
    fn seed_at_1800_km_gaps_but_2200_km_covers() {
        // The 6×12 seed at 1,800 km leaves band points with zero usable
        // satellites; the same 72 satellites at 2,200 km cover continuously.
        // Sampling here (120 s, 36 azimuths) is the coarsest that exhibits
        // the 1,800 km gap; finer sampling (30 s, 72 azimuths, see the
        // access_constellation example) agrees on both verdicts.
        let p = reference_planet();
        let band = 20.0_f64.to_radians();
        let duration = 11.2 * 86_400.0;
        let seed = PolarConstellation {
            altitude: 1_800e3,
            planes: 6,
            sats_per_plane: 12,
            interplane_phase: 0.0,
        };
        let stats = band_coverage(&p, &seed, band, MIN_ELEVATION, duration, 120.0, 36);
        assert_eq!(stats.min_visible, 0);

        let baseline = PolarConstellation {
            altitude: 2_200e3,
            ..seed
        };
        let stats = band_coverage(&p, &baseline, band, MIN_ELEVATION, duration, 120.0, 36);
        assert!(stats.min_visible >= 1);
    }

    #[test]
    fn solved_arc_count_matches_brute_force_enumeration() {
        // plane_visible_count solves for the visibility arc instead of
        // testing every spacecraft; the two must agree exactly, or every
        // randomized phase sweep built on the fast path is worthless.
        let p = reference_planet();
        // The half-slot stagger at 1,800 km is in this list deliberately: an
        // earlier arithmetic-only solver over-counted by one there, exactly at
        // the instant the band's coverage minimum is decided, and reported a
        // real gap as covered. Sampling is fine enough to land on it.
        for (planes, sats, stagger) in [
            (6, 12, 0.0),
            (6, 12, 0.5 * 2.0 * PI / 12.0),
            (6, 14, 0.13),
            (8, 12, 0.4),
        ] {
            for altitude in [1_800e3, 2_200e3] {
                let c = PolarConstellation {
                    altitude,
                    planes,
                    sats_per_plane: sats,
                    interplane_phase: stagger,
                };
                let phases: Vec<f64> = (0..planes).map(|k| c.plane_phase(k)).collect();
                let mut t = 0.0;
                while t < 2.0 * 86_400.0 {
                    for i in 0..72 {
                        let az = i as f64 * 2.0 * PI / 72.0;
                        for off in [-0.349_066, -0.1, 0.0, 0.2, 0.349_066] {
                            let g = band_point(az, off);
                            assert_eq!(
                            visible_count_with_phases(&p, &c, g, &phases, MIN_ELEVATION, t),
                            visible_count(&p, &c, g, MIN_ELEVATION, t),
                            "planes {planes} sats {sats} alt {altitude} az {az} off {off} t {t}"
                        );
                        }
                    }
                    t += 30.0;
                }
            }
        }
    }

    #[test]
    fn the_half_slot_alternation_breaks_the_baseline() {
        // Coverage survives *uncoordinated* phasing, but not every phasing.
        // Offsetting each ring half a slot from the last -- the triangular
        // interleave a cellular network would use -- is a structured pattern,
        // and at the 12-satellite baseline it opens a gap that the aligned
        // wheel does not have. Robust to random is not the same as immune.
        let p = reference_planet();
        let slot = 2.0 * PI / 12.0;
        let aligned = PolarConstellation {
            altitude: 2_200e3,
            planes: 6,
            sats_per_plane: 12,
            interplane_phase: 0.0,
        };
        let alternating = PolarConstellation {
            interplane_phase: slot / 2.0,
            ..aligned
        };
        let band = 20.0_f64.to_radians();
        let dur = 11.2 * 86_400.0;
        assert_eq!(
            band_coverage(&p, &aligned, band, MIN_ELEVATION, dur, 120.0, 36).min_visible,
            1
        );
        assert_eq!(
            band_coverage(&p, &alternating, band, MIN_ELEVATION, dur, 30.0, 72).min_visible,
            0,
            "the half-slot alternation should open a gap at 12 satellites per ring"
        );
    }

    #[test]
    fn a_plane_beyond_its_footprint_sees_nothing_at_any_satellite_count() {
        // Past the footprint half-angle, packing more satellites into a ring
        // changes nothing: the ring simply never reaches the ground point.
        let p = reference_planet();
        let g = band_point(0.0, 0.0);
        for sats in [12, 24, 48, 96] {
            let c = PolarConstellation {
                altitude: 2_200e3,
                planes: 6,
                sats_per_plane: sats,
                interplane_phase: 0.0,
            };
            // Plane 3's node lies along this ground point, so it serves it...
            assert!(plane_visible_count(&p, &c, 3, g, 0.0, MIN_ELEVATION, 0.0) > 0);
            // ...while plane 0's node is 90 deg away: hopeless at any count.
            assert_eq!(
                plane_visible_count(&p, &c, 0, g, 0.0, MIN_ELEVATION, 0.0),
                0
            );
        }
    }

    #[test]
    fn band_points_lie_on_the_unit_sphere() {
        for (az, off) in [(0.3, 0.2), (2.0, -0.35), (4.5, 0.0)] {
            let p = band_point(az, off);
            let norm = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            assert_close(norm, 1.0, 1e-12);
        }
    }
}
