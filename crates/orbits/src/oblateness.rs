//! Figure of a rotating body: how far from a sphere it is, and what that
//! does to an orbit's node.
//!
//! A spinning fluid body bulges at its equator. The size of the bulge is set
//! by how hard the spin pulls against gravity — the dimensionless ratio
//!
//! ```text
//! q = omega^2 R^3 / mu
//! ```
//!
//! — and by how centrally condensed the body is, captured by the *fluid*
//! (secular) Love number `k2`. For a body that spins freely, the resulting
//! zonal harmonic is `J2 = k2 q / 3`.
//!
//! A **tidally locked** body is a different case. It carries a second,
//! permanent bulge raised by the star it faces, and because the spin is
//! synchronous the tidal and rotational deformations are locked to the same
//! `q`. The equilibrium figure is triaxial, with
//!
//! ```text
//! J2 = 5 k2 q / 6      C22 = k2 q / 4      J2 / C22 = 10 / 3
//! ```
//!
//! so a locked planet is *two and a half times* as oblate as its spin alone
//! would make it — but its spin is so slow that the total is still far below
//! a free-spinning planet's. That combination is the whole reason the
//! Terminus rings can be treated as inertially fixed.

use crate::CentralBody;

/// Fluid (secular) Love number of an Earth-like differentiated rocky planet,
/// calibrated by inverting `J2 = k2 q / 3` on Earth itself: Earth's
/// `q = 3.4498e-3` and `J2 = 1.0826e-3` give `k2 = 0.9414`.
///
/// A homogeneous fluid body would have `k2 = 1.5`; the smaller value is the
/// signature of a dense core. Io, the best-measured synchronous body, sits
/// at 1.30 by the same inversion.
pub const EARTH_FLUID_LOVE_NUMBER: f64 = 0.9414;

/// Ratio of centrifugal to gravitational acceleration at the equator,
/// `q = omega^2 R^3 / mu` — the dimensionless measure of how hard a body's
/// spin fights its own gravity.
pub fn rotational_parameter(body: &CentralBody) -> f64 {
    let omega = 2.0 * std::f64::consts::PI / body.rotation_period;
    omega * omega * body.radius.powi(3) / body.mu
}

/// Second zonal harmonic `J2` of a freely rotating body in hydrostatic
/// equilibrium: `k2 q / 3`.
pub fn free_rotation_j2(body: &CentralBody, fluid_love_number: f64) -> f64 {
    fluid_love_number * rotational_parameter(body) / 3.0
}

/// Second zonal harmonic `J2` of a **synchronously rotating** body, whose
/// permanent tidal bulge adds to its rotational one: `5 k2 q / 6`.
pub fn synchronous_j2(body: &CentralBody, fluid_love_number: f64) -> f64 {
    5.0 * fluid_love_number * rotational_parameter(body) / 6.0
}

/// Sectoral harmonic `C22` of a synchronously rotating body — the
/// star-facing bulge: `k2 q / 4`. Hydrostatic equilibrium fixes
/// `J2 / C22 = 10 / 3`.
pub fn synchronous_c22(body: &CentralBody, fluid_love_number: f64) -> f64 {
    fluid_love_number * rotational_parameter(body) / 4.0
}

/// Geometric flattening `f = (R_eq - R_pol) / R_eq` implied by a given `J2`,
/// from `f = 3 J2 / 2 + q / 2`.
pub fn flattening(body: &CentralBody, j2: f64) -> f64 {
    1.5 * j2 + rotational_parameter(body) / 2.0
}

/// Rate (rad/s, signed) at which `J2` drags an orbit's ascending node around
/// the equator:
///
/// ```text
/// dOmega/dt = -(3/2) J2 (R/a)^2 n cos i
/// ```
///
/// It vanishes at `inclination = 90°`: a perfectly polar orbit has no
/// preferred sense to regress in. That is what lets a polar ring stay fixed
/// against the stars — and what makes injection inclination error, not `J2`
/// itself, the thing that moves a ring.
pub fn nodal_regression_rate(body: &CentralBody, altitude: f64, inclination: f64, j2: f64) -> f64 {
    let a = body.radius + altitude;
    let n = 2.0 * std::f64::consts::PI / crate::circular::orbital_period(body, altitude);
    -1.5 * j2 * (body.radius / a).powi(2) * n * inclination.cos()
}

/// Node drift (rad/s) of a nominally polar ring injected `inclination_error`
/// (rad) away from 90°. The magnitude of [`nodal_regression_rate`] with
/// `cos i = sin(error) ≈ error`.
pub fn polar_node_drift_rate(
    body: &CentralBody,
    altitude: f64,
    inclination_error: f64,
    j2: f64,
) -> f64 {
    nodal_regression_rate(
        body,
        altitude,
        std::f64::consts::FRAC_PI_2 - inclination_error,
        j2,
    )
    .abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(rel < rel_tol, "actual {actual}, expected {expected}, rel err {rel}");
    }

    fn earth() -> CentralBody {
        // sidereal day, not solar
        CentralBody::from_earth_masses(1.0, 6.371e6, 86_164.090_5)
    }

    /// The reference planet: one Earth mass and radius, tidally locked to an
    /// 11.2-day orbit.
    fn locked_planet() -> CentralBody {
        CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0)
    }

    #[test]
    fn earth_rotational_parameter_and_j2_round_trip() {
        let e = earth();
        assert_close(rotational_parameter(&e), 3.4498e-3, 1e-3);
        // The calibration must reproduce Earth's measured J2.
        assert_close(free_rotation_j2(&e, EARTH_FLUID_LOVE_NUMBER), 1.0826e-3, 1e-3);
        // ...and Earth's measured flattening, 1/298.257.
        assert_close(flattening(&e, 1.0826e-3), 1.0 / 298.257, 2e-3);
    }

    #[test]
    fn locked_planet_is_far_rounder_than_earth() {
        let (e, p) = (earth(), locked_planet());
        // q scales as omega^2: 11.2 days vs one sidereal day.
        let spin_ratio = p.rotation_period / e.rotation_period;
        assert_close(
            rotational_parameter(&e) / rotational_parameter(&p),
            spin_ratio * spin_ratio,
            1e-9,
        );

        let j2 = synchronous_j2(&p, EARTH_FLUID_LOVE_NUMBER);
        assert_close(j2, 2.1458e-5, 1e-3);
        // Locked, it is still ~50x rounder than Earth.
        assert!((1.0826e-3 / j2 - 50.0).abs() < 1.0);
        // The star's bulge dominates: 2.5x the spin-only figure.
        assert_close(j2 / free_rotation_j2(&p, EARTH_FLUID_LOVE_NUMBER), 2.5, 1e-9);
    }

    #[test]
    fn hydrostatic_synchronous_figure_has_the_canonical_harmonic_ratio() {
        let p = locked_planet();
        let j2 = synchronous_j2(&p, EARTH_FLUID_LOVE_NUMBER);
        let c22 = synchronous_c22(&p, EARTH_FLUID_LOVE_NUMBER);
        assert_close(j2 / c22, 10.0 / 3.0, 1e-9);
    }

    #[test]
    fn a_polar_orbit_has_no_nodal_regression() {
        let p = locked_planet();
        let j2 = synchronous_j2(&p, EARTH_FLUID_LOVE_NUMBER);
        let rate = nodal_regression_rate(&p, 2_200e3, std::f64::consts::FRAC_PI_2, j2);
        assert!(rate.abs() < 1e-18, "rate {rate}");
    }

    #[test]
    fn inclination_error_moves_a_ring_only_glacially() {
        let p = locked_planet();
        let j2 = synchronous_j2(&p, EARTH_FLUID_LOVE_NUMBER);
        // A tenth of a degree off polar: under half a degree of node drift
        // per decade, against 30 deg of ring spacing.
        let rate = polar_node_drift_rate(&p, 2_200e3, 0.1_f64.to_radians(), j2);
        let per_decade = rate.to_degrees() * 86_400.0 * 3652.5;
        assert!(per_decade < 0.5, "{per_decade} deg/decade");

        // The same error around Earth would move it 50x faster.
        let earth_like = polar_node_drift_rate(&p, 2_200e3, 0.1_f64.to_radians(), 1.0826e-3);
        assert_close(earth_like / rate, 1.0826e-3 / j2, 1e-9);
    }
}
