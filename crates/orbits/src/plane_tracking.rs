use std::f64::consts::PI;

use crate::circular::orbital_velocity;
use crate::CentralBody;

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

/// Fraction of the spacecraft's initial mass remaining after sustaining the
/// ideal plane-tracking Δv for the given number of Earth days (rocket
/// equation, compounding).
pub fn remaining_mass_fraction(body: &CentralBody, altitude: f64, isp: f64, days: f64) -> f64 {
    let dv = ideal_plane_change_dv_per_day(body, altitude) * days;
    (-dv / (G0 * isp)).exp()
}

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

    #[test]
    fn compounding_consumes_the_spacecraft() {
        let p = reference_planet();
        // 0.8753^11.2 ≈ 0.2250 after one local year (11.2 Earth days);
        // 0.8753^30 ≈ 0.0184 after 30 Earth days.
        assert_close(remaining_mass_fraction(&p, 1_800e3, 3_000.0, 11.2), 0.2250, 2e-3);
        assert_close(remaining_mass_fraction(&p, 1_800e3, 3_000.0, 30.0), 0.0184, 5e-3);
    }
}
