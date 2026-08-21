//! Single-satellite coverage geometry: how much ground one satellite sees
//! above a minimum elevation angle, how far the edge of that view is, and
//! how long an overhead pass lasts.

use crate::circular::orbital_period;
use crate::CentralBody;

/// Planet-central angle (rad) from the sub-satellite point to the edge of
/// coverage, for a satellite at `altitude` seen at `min_elevation` (rad).
fn coverage_half_angle(body: &CentralBody, altitude: f64, min_elevation: f64) -> f64 {
    let ratio = body.radius / (body.radius + altitude);
    (ratio * min_elevation.cos()).acos() - min_elevation
}

/// Great-circle radius (m) of the ground area a satellite serves above the
/// given minimum elevation angle (rad).
pub fn footprint_radius(body: &CentralBody, altitude: f64, min_elevation: f64) -> f64 {
    body.radius * coverage_half_angle(body, altitude, min_elevation)
}

/// Slant range (m) from a ground user at exactly the minimum elevation angle
/// to the satellite — the longest, highest-loss path in the footprint.
pub fn edge_slant_range(body: &CentralBody, altitude: f64, min_elevation: f64) -> f64 {
    let ratio = (body.radius + altitude) / body.radius;
    body.radius * ((ratio * ratio - min_elevation.cos().powi(2)).sqrt() - min_elevation.sin())
}

/// Duration (s) of the best-case pass, straight through the zenith, for a
/// ground user on a slowly rotating body: the satellite is visible while it
/// crosses `2 × coverage_half_angle` of its orbit.
pub fn max_pass_duration(body: &CentralBody, altitude: f64, min_elevation: f64) -> f64 {
    let lambda = coverage_half_angle(body, altitude, min_elevation);
    lambda / std::f64::consts::PI * orbital_period(body, altitude)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn footprint_radius_at_reference_altitudes() {
        let p = reference_planet();
        assert_close(footprint_radius(&p, 300e3, MIN_ELEVATION), 5.621e5, 1e-3);
        assert_close(footprint_radius(&p, 1_800e3, MIN_ELEVATION), 2.228e6, 1e-3);
        assert_close(footprint_radius(&p, 20_000e3, MIN_ELEVATION), 5.821e6, 1e-3);
    }

    #[test]
    fn edge_slant_range_at_reference_altitudes() {
        let p = reference_planet();
        // 3,089 km at 1,800 km (≈10.3 ms one way); 23,039 km at 20,000 km
        // (≈77 ms one way).
        assert_close(edge_slant_range(&p, 1_800e3, MIN_ELEVATION), 3.089e6, 1e-3);
        assert_close(edge_slant_range(&p, 20_000e3, MIN_ELEVATION), 2.3039e7, 1e-3);
    }

    #[test]
    fn max_pass_duration_at_reference_altitudes() {
        let p = reference_planet();
        // ≈13.6 min at 1,800 km; ≈3.4 h at 20,000 km.
        assert_close(max_pass_duration(&p, 1_800e3, MIN_ELEVATION), 818.0, 2e-3);
        assert_close(max_pass_duration(&p, 20_000e3, MIN_ELEVATION), 1.2395e4, 2e-3);
    }
}
