//! Spin-orbit relations: solar day length and terminator drift.
//!
//! For a prograde rotator, one solar day satisfies
//! `1/S = 1/P_rot - 1/P_orb` (sidereal periods). A 1:1 tidally locked
//! body has an infinite solar day: its terminator is fixed on the surface.

use std::f64::consts::PI;

/// Length of the solar day in seconds for a prograde rotator, given the
/// sidereal rotation period and the orbital period around the star (both in
/// seconds). Returns `None` for a 1:1 locked body (infinite solar day).
pub fn solar_day(rotation_period: f64, orbital_period: f64) -> Option<f64> {
    let rate = 1.0 / rotation_period - 1.0 / orbital_period;
    if rate == 0.0 {
        None
    } else {
        Some(1.0 / rate)
    }
}

/// Speed at which the terminator sweeps across the equator, m/s, for a body
/// of the given radius (m) and solar day (s): one equatorial circumference
/// per solar day.
pub fn terminator_drift_speed(radius: f64, solar_day: f64) -> f64 {
    2.0 * PI * radius / solar_day
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: f64 = 86_400.0;

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn mercury_solar_day_is_about_176_earth_days() {
        // Sidereal rotation 58.646 d, orbital period 87.969 d (3:2 resonance).
        let s = solar_day(58.646 * DAY, 87.969 * DAY).unwrap();
        assert_close(s, 1.5201e7, 1e-3); // ≈ 175.9 Earth days
    }

    #[test]
    fn locked_body_has_no_solar_day() {
        assert_eq!(solar_day(11.2 * DAY, 11.2 * DAY), None);
    }

    #[test]
    fn mercury_terminator_drifts_at_walking_pace() {
        // Radius 2,439.7 km; ~1 m/s at the equator.
        let s = solar_day(58.646 * DAY, 87.969 * DAY).unwrap();
        assert_close(terminator_drift_speed(2.4397e6, s), 1.0085, 1e-3);
    }
}
