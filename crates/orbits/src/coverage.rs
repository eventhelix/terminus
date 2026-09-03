// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

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

/// Ceiling (m) on `footprint_radius`, approached as altitude grows without
/// bound. The planet-central half-angle is
/// `acos(ratio * cos(min_elevation)) - min_elevation`; as `ratio` falls to
/// zero the arccos saturates at a right angle, so no altitude can serve more
/// ground than `90 deg - min_elevation` of arc.
pub fn footprint_radius_limit(body: &CentralBody, min_elevation: f64) -> f64 {
    body.radius * (std::f64::consts::FRAC_PI_2 - min_elevation)
}

/// Slant range (m) from a ground user at exactly the minimum elevation angle
/// to the satellite — the longest, highest-loss path in the footprint.
///
/// From the law of cosines on the same O-U-S triangle `coverage_half_angle`
/// uses. Writing the satellite's position as the user's position plus `d` at
/// `min_elevation` above the local horizon gives
/// `(R + d·sin ε)² + (d·cos ε)² = (R + h)²`, a quadratic in `d` whose
/// positive root is the expression below.
pub fn edge_slant_range(body: &CentralBody, altitude: f64, min_elevation: f64) -> f64 {
    let ratio = (body.radius + altitude) / body.radius;
    body.radius * ((ratio * ratio - min_elevation.cos().powi(2)).sqrt() - min_elevation.sin())
}

/// Duration (s) of the best-case pass, straight through the zenith, for a
/// ground user on a slowly rotating body: the satellite is visible while it
/// crosses `2 × coverage_half_angle` of its orbit.
///
/// That arc is `2λ` out of `2π`, so the pass is the fraction `λ/π` of one
/// lap. Both factors grow with altitude — a longer period and a wider λ —
/// which is why dwell climbs far faster than period alone: across the
/// reference survey it grows 287x, against 24.6x for the period.
///
/// Idealised: it assumes the pass crosses the user's zenith and ignores the
/// body's rotation during the pass. Fair for a slow rotator, but not free —
/// on the reference planet a 37 h pass at 50,000 km spans ~14% of an
/// 11.2-day day.
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
    fn footprint_radius_approaches_its_ceiling() {
        let p = reference_planet();
        let ceiling = footprint_radius_limit(&p, MIN_ELEVATION);
        // 6371 km x 65 deg of arc.
        assert_close(ceiling, 7.2276e6, 1e-3);
        // The highest surveyed shelf already buys most of it, and nothing
        // above it ever crosses the line.
        assert!(footprint_radius(&p, 50_000e3, MIN_ELEVATION) / ceiling > 0.90);
        assert!(footprint_radius(&p, 1e12, MIN_ELEVATION) < ceiling);
    }

    #[test]
    fn dropping_the_mask_costs_low_shelves_most() {
        let p = reference_planet();
        let cost =
            |alt: f64| footprint_radius(&p, alt, 0.0) / footprint_radius(&p, alt, MIN_ELEVATION);
        assert_close(cost(300e3), 3.40, 1e-2);
        assert_close(cost(20_000e3), 1.45, 1e-2);
    }

    #[test]
    fn edge_slant_range_matches_the_law_of_cosines() {
        // The closed form is the positive root of a quadratic; the law of
        // cosines on the same triangle must agree at every surveyed shelf.
        let p = reference_planet();
        for altitude_km in [300.0, 1_200.0, 1_800.0, 10_000.0, 20_000.0, 50_000.0] {
            let h = altitude_km * 1e3;
            let lambda = coverage_half_angle(&p, h, MIN_ELEVATION);
            let r = p.radius + h;
            let by_cosines =
                (p.radius * p.radius + r * r - 2.0 * p.radius * r * lambda.cos()).sqrt();
            assert_close(edge_slant_range(&p, h, MIN_ELEVATION), by_cosines, 1e-9);
        }
    }

    #[test]
    fn edge_slant_range_limits() {
        let p = reference_planet();
        let h = 1_800e3;
        // Straight overhead there is no slant at all: the path is the altitude.
        assert_close(
            edge_slant_range(&p, h, std::f64::consts::FRAC_PI_2),
            h,
            1e-9,
        );
        // Dropping the mask to the horizon gives the tangent line, sqrt((R+h)^2 - R^2).
        let horizon = ((p.radius + h).powi(2) - p.radius.powi(2)).sqrt();
        assert_close(edge_slant_range(&p, h, 0.0), horizon, 1e-9);
    }

    #[test]
    fn dwell_growth_splits_into_period_and_angle() {
        // Dwell is (lambda/pi) * period, so its growth across the survey is
        // the product of the two growths: 24.56x from period and 11.70x from
        // the wider half-angle make 287x, not the 24.56x period alone.
        let p = reference_planet();
        let (lo, hi) = (300e3, 50_000e3);
        let period_ratio = orbital_period(&p, hi) / orbital_period(&p, lo);
        let angle_ratio =
            coverage_half_angle(&p, hi, MIN_ELEVATION) / coverage_half_angle(&p, lo, MIN_ELEVATION);
        let dwell_ratio =
            max_pass_duration(&p, hi, MIN_ELEVATION) / max_pass_duration(&p, lo, MIN_ELEVATION);
        assert_close(period_ratio, 24.564, 1e-4);
        assert_close(angle_ratio, 11.697, 1e-4);
        assert_close(dwell_ratio, 287.32, 1e-4);
        assert_close(dwell_ratio, period_ratio * angle_ratio, 1e-9);
    }

    #[test]
    fn edge_slant_range_at_reference_altitudes() {
        let p = reference_planet();
        // 3,089 km at 1,800 km (≈10.3 ms one way); 23,039 km at 20,000 km
        // (≈77 ms one way).
        assert_close(edge_slant_range(&p, 1_800e3, MIN_ELEVATION), 3.089e6, 1e-3);
        assert_close(
            edge_slant_range(&p, 20_000e3, MIN_ELEVATION),
            2.3039e7,
            1e-3,
        );
    }

    #[test]
    fn max_pass_duration_at_reference_altitudes() {
        let p = reference_planet();
        // ≈13.6 min at 1,800 km; ≈3.4 h at 20,000 km.
        assert_close(max_pass_duration(&p, 1_800e3, MIN_ELEVATION), 818.0, 2e-3);
        assert_close(
            max_pass_duration(&p, 20_000e3, MIN_ELEVATION),
            1.2395e4,
            2e-3,
        );
    }
}
