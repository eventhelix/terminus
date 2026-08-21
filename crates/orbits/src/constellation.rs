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
pub fn polar_sat_position(body: &CentralBody, altitude: f64, raan: f64, theta0: f64, t: f64) -> [f64; 3] {
    let r = body.radius + altitude;
    let n = 2.0 * std::f64::consts::PI
        / crate::circular::orbital_period(body, altitude);
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
    use super::*;
    use crate::coverage::footprint_radius;

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
    fn band_points_lie_on_the_unit_sphere() {
        for (az, off) in [(0.3, 0.2), (2.0, -0.35), (4.5, 0.0)] {
            let p = band_point(az, off);
            let norm = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            assert_close(norm, 1.0, 1e-12);
        }
    }
}
