use std::f64::consts::PI;

use crate::CentralBody;

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
