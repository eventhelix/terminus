// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Hill-sphere estimates: how far from a planet a satellite can orbit before
//! the star's gravity takes over.

use crate::CentralBody;

/// Standard gravitational parameter of the Sun, m³/s².
pub const SUN_MU: f64 = 1.32712440018e20;

/// Hill radius (m): the scale of the region where the planet's gravity
/// dominates the star's, for a planet with gravitational parameter
/// `body.mu` orbiting a star of gravitational parameter `star_mu` at
/// distance `orbital_distance` (m).
pub fn hill_radius(body: &CentralBody, star_mu: f64, orbital_distance: f64) -> f64 {
    orbital_distance * (body.mu / (3.0 * star_mu)).cbrt()
}

/// Conservative outer limit (m) for long-lived prograde satellite orbits:
/// roughly half the Hill radius.
pub fn prograde_stability_limit(body: &CentralBody, star_mu: f64, orbital_distance: f64) -> f64 {
    0.5 * hill_radius(body, star_mu, orbital_distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn hill_radius_for_close_in_earth_mass_planet() {
        // Earth-mass planet, 0.122-solar-mass star, 0.0485 AU (7.2555e9 m):
        // Hill radius ≈ 146,400 km; prograde stability limit ≈ 73,200 km.
        let p = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        let star_mu = 0.122 * SUN_MU;
        let a = 7.2555e9;
        assert_close(hill_radius(&p, star_mu, a), 1.4645e8, 1e-3);
        assert_close(prograde_stability_limit(&p, star_mu, a), 7.322e7, 1e-3);
    }
}
