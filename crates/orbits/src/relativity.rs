// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! First-order relativistic clock rates for navigation-grade timing:
//! velocity time dilation, gravitational blueshift, and the stellar tidal
//! modulation that distinguishes this system from Earth GNSS.

use crate::placement::SPEED_OF_LIGHT;
use crate::CentralBody;

/// Fractional rate at which a circular-orbit satellite clock runs slow due
/// to its orbital velocity (special relativity): v²/2c², with v² = μ/r.
pub fn fractional_velocity_dilation(body: &CentralBody, altitude: f64) -> f64 {
    let c2 = SPEED_OF_LIGHT * SPEED_OF_LIGHT;
    body.mu / (body.radius + altitude) / (2.0 * c2)
}

/// Fractional rate at which a satellite clock runs fast relative to a
/// surface clock because it sits higher in the planet's gravity well
/// (general relativity): (μ/R − μ/r)/c².
pub fn fractional_gravitational_blueshift(body: &CentralBody, altitude: f64) -> f64 {
    let c2 = SPEED_OF_LIGHT * SPEED_OF_LIGHT;
    (body.mu / body.radius - body.mu / (body.radius + altitude)) / c2
}

/// Net satellite-clock rate versus a surface clock, in seconds per Earth
/// day (positive: satellite clock runs fast).
pub fn net_clock_rate_per_day(body: &CentralBody, altitude: f64) -> f64 {
    (fractional_gravitational_blueshift(body, altitude)
        - fractional_velocity_dilation(body, altitude))
        * 86_400.0
}

/// Scale of the stellar tidal clock modulation across a satellite orbit of
/// radius `orbit_radius` (m) around a planet at `orbital_distance` (m)
/// from a star of gravitational parameter `star_mu`: μ★·r²/(a³·c²), in
/// seconds per Earth day. (The star's direct potential cancels for the
/// freely falling planet system; the tidal term does not.)
pub fn stellar_tidal_rate_per_day(star_mu: f64, orbital_distance: f64, orbit_radius: f64) -> f64 {
    let c2 = SPEED_OF_LIGHT * SPEED_OF_LIGHT;
    star_mu * orbit_radius * orbit_radius / (orbital_distance.powi(3) * c2) * 86_400.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hill::SUN_MU;

    const MEO_ALT: f64 = 20_000e3;

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
    fn meo_clocks_echo_the_gps_numbers() {
        // Earth-mass planet, ~GPS-altitude shell: slow 7.27 µs/day from
        // velocity, fast 45.6 µs/day from altitude, net +38.3 µs/day —
        // within a hair of Earth GPS's +38.6.
        let p = reference_planet();
        assert_close(
            fractional_velocity_dilation(&p, MEO_ALT) * 86_400.0,
            7.265e-6,
            1e-3,
        );
        assert_close(
            fractional_gravitational_blueshift(&p, MEO_ALT) * 86_400.0,
            4.561e-5,
            1e-3,
        );
        assert_close(net_clock_rate_per_day(&p, MEO_ALT), 3.835e-5, 1e-3);
    }

    #[test]
    fn stellar_tide_is_a_thousand_times_earths() {
        // An M-dwarf-mass star at 0.0485 AU vs the Sun at 1 AU (GPS radius):
        // ~28 ns/day of periodic modulation vs ~0.027 ns/day.
        let p = reference_planet();
        let local = stellar_tidal_rate_per_day(0.122 * SUN_MU, 7.2555e9, p.radius + MEO_ALT);
        let earth = stellar_tidal_rate_per_day(SUN_MU, 1.495979e11, 2.656e7);
        assert_close(local, 2.834e-8, 1e-3);
        assert_close(local / earth, 1_054.0, 5e-3);
    }
}
