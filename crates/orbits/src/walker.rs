//! Inclined circular shells, evaluated in the same planet-fixed frame as
//! [`crate::constellation`].
//!
//! A Walker-style shell spreads `planes` ascending nodes evenly over the
//! full 360° — inclined planes, unlike polar ones, are not their own 180°
//! opposites — and phases the satellites within each plane by the Walker
//! factor. The polar constellation is the inclination = 90° special case;
//! `inclined_sat_position` reduces exactly to
//! [`crate::constellation::polar_sat_position`] there, and a test pins that.
//!
//! This is the shape a navigation shell wants: a ground point needs four
//! satellites in view simultaneously, spread in azimuth and elevation, which
//! a small number of polar planes cannot offer at every latitude.

use std::f64::consts::PI;

use crate::circular::orbital_period;
use crate::constellation::{band_point, elevation, CoverageStats};
use crate::CentralBody;

/// A symmetric shell of circular orbits at a common inclination: `planes`
/// planes with ascending nodes spread evenly over 360°, `sats_per_plane`
/// satellites evenly phased in each, all at the same `altitude` (m).
///
/// `phase_factor` is the Walker Delta phasing parameter F: satellites in
/// plane k are advanced by `F · 2π / (planes · sats_per_plane)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkerShell {
    pub altitude: f64,
    pub planes: usize,
    pub sats_per_plane: usize,
    pub inclination: f64,
    pub phase_factor: f64,
}

impl WalkerShell {
    /// Total satellites in the shell.
    pub fn total(&self) -> usize {
        self.planes * self.sats_per_plane
    }

    /// Ascending node (rad, at t = 0) of plane `k`.
    pub fn raan(&self, k: usize) -> f64 {
        k as f64 * 2.0 * PI / self.planes as f64
    }

    /// Along-orbit phase (rad, at t = 0) of satellite `j` in plane `k`.
    pub fn theta0(&self, k: usize, j: usize) -> f64 {
        j as f64 * 2.0 * PI / self.sats_per_plane as f64
            + k as f64 * self.phase_factor * 2.0 * PI / self.total() as f64
    }
}

/// Position (m, planet-fixed frame) of a satellite in a circular orbit of
/// arbitrary `inclination` (rad) with ascending node `raan` (rad, at t = 0)
/// and along-orbit phase `theta0` (rad, at t = 0), evaluated at time `t` (s).
///
/// As in the polar case, the plane is inertially fixed, so in the rotating
/// planet-fixed frame its node regresses at the body's spin rate.
pub fn inclined_sat_position(
    body: &CentralBody,
    altitude: f64,
    raan: f64,
    theta0: f64,
    inclination: f64,
    t: f64,
) -> [f64; 3] {
    let r = body.radius + altitude;
    let n = 2.0 * PI / orbital_period(body, altitude);
    let spin = 2.0 * PI / body.rotation_period;
    let theta = theta0 + n * t;
    let node = raan - spin * t;
    let (st, ct) = theta.sin_cos();
    let (sn, cn) = node.sin_cos();
    let (si, ci) = inclination.sin_cos();
    [
        r * (ct * cn - st * ci * sn),
        r * (ct * sn + st * ci * cn),
        r * (st * si),
    ]
}

/// Position of the satellite in plane `k`, slot `j` of `shell` at time `t`.
pub fn shell_sat_position(
    body: &CentralBody,
    shell: &WalkerShell,
    k: usize,
    j: usize,
    t: f64,
) -> [f64; 3] {
    inclined_sat_position(
        body,
        shell.altitude,
        shell.raan(k),
        shell.theta0(k, j),
        shell.inclination,
        t,
    )
}

/// Number of the shell's satellites at or above `min_elevation` (rad) as
/// seen from `ground_unit` at time `t`.
pub fn visible_count(
    body: &CentralBody,
    shell: &WalkerShell,
    ground_unit: [f64; 3],
    min_elevation: f64,
    t: f64,
) -> usize {
    let mut count = 0;
    for k in 0..shell.planes {
        for j in 0..shell.sats_per_plane {
            if elevation(body, ground_unit, shell_sat_position(body, shell, k, j, t))
                >= min_elevation
            {
                count += 1;
            }
        }
    }
    count
}

/// Fewest and average satellites in view from any sampled band point at any
/// sampled instant — the navigation counterpart of
/// [`crate::constellation::band_coverage`], with the same sampling scheme.
pub fn band_coverage(
    body: &CentralBody,
    shell: &WalkerShell,
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
            let azimuth = i as f64 * 2.0 * PI / azimuth_samples as f64;
            for offset in offsets {
                let n = visible_count(body, shell, band_point(azimuth, offset), min_elevation, t);
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
    use crate::constellation::polar_sat_position;

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

    /// A navigation shell of 6 planes × 4 satellites at 55°, the arrangement
    /// a GPS-like service uses, at the service (MEO) altitude.
    fn nav_shell() -> WalkerShell {
        WalkerShell {
            altitude: 20_000e3,
            planes: 6,
            sats_per_plane: 4,
            inclination: 55.0_f64.to_radians(),
            phase_factor: 1.0,
        }
    }

    #[test]
    fn inclined_position_reduces_to_the_polar_case() {
        let p = reference_planet();
        for &(raan, theta0, t) in &[(0.0, 0.0, 0.0), (0.4, 1.1, 500.0), (2.9, -0.7, 40_000.0)] {
            let polar = polar_sat_position(&p, 2_200e3, raan, theta0, t);
            let inc = inclined_sat_position(
                &p,
                2_200e3,
                raan,
                theta0,
                std::f64::consts::FRAC_PI_2,
                t,
            );
            for axis in 0..3 {
                assert!(
                    (polar[axis] - inc[axis]).abs() < 1e-6,
                    "axis {axis}: {} vs {}",
                    polar[axis],
                    inc[axis]
                );
            }
        }
    }

    #[test]
    fn inclined_orbit_is_circular_and_bounded_by_its_inclination() {
        let p = reference_planet();
        let inclination = 55.0_f64.to_radians();
        let r = p.radius + 20_000e3;
        let mut max_z: f64 = 0.0;
        for step in 0..360 {
            let theta = (step as f64).to_radians();
            let pos = inclined_sat_position(&p, 20_000e3, 0.3, theta, inclination, 0.0);
            let norm = (pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt();
            assert_close(norm, r, 1e-12);
            max_z = max_z.max(pos[2].abs());
        }
        // The orbit reaches exactly its inclination in latitude, no further.
        assert_close(max_z, r * inclination.sin(), 1e-3);
    }

    #[test]
    fn nav_shell_keeps_four_in_view_but_one_per_plane_does_not() {
        // TER-REQ-015 asks for four navigation satellites in view at all
        // times. Six planes of four at 55° hold that everywhere in the band;
        // the same six planes with one satellite each — enough to anchor
        // sessions — leave band points with no fix at all.
        let p = reference_planet();
        let band = 20.0_f64.to_radians();
        let mask = 10.0_f64.to_radians();
        let duration = 86_400.0;

        let stats = band_coverage(&p, &nav_shell(), band, mask, duration, 300.0, 36);
        assert!(
            stats.min_visible >= 4,
            "min visible {} < 4",
            stats.min_visible
        );

        let thin = WalkerShell {
            sats_per_plane: 1,
            ..nav_shell()
        };
        let stats = band_coverage(&p, &thin, band, mask, duration, 300.0, 36);
        assert_eq!(stats.min_visible, 0);
    }

    #[test]
    fn shell_phasing_places_one_satellite_per_plane_per_slot() {
        let shell = nav_shell();
        assert_eq!(shell.total(), 24);
        assert_close(shell.raan(1), 2.0 * PI / 6.0, 1e-12);
        // Walker F = 1: plane k is advanced by one 24th of an orbit.
        assert_close(
            shell.theta0(1, 0) - shell.theta0(0, 0),
            2.0 * PI / 24.0,
            1e-12,
        );
    }
}
