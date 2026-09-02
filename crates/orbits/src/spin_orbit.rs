// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

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
    // Exact equality, deliberately - and note what it tests: the two *inputs*,
    // not a computed quantity. A 1:1 lock is a modelling statement ("these two
    // periods are the same number"), not a measurement that might land near
    // zero, so there is nothing for a tolerance to absorb. A body a nanosecond
    // off lock really does have a finite, if astronomical, solar day, and
    // reporting it is more honest than snapping it to infinity.
    if orbital_period == rotation_period {
        return None;
    }
    // Algebraically 1/(1/P_rot - 1/P_orb), but never computed that way. For
    // nearly equal periods that subtraction cancels away the significant
    // digits, and for periods within an ulp of each other it underflows to
    // exactly zero - which would report an unlocked body as locked. Multiplying
    // first keeps the precision and leaves the only equality test on values the
    // caller supplied.
    Some(rotation_period * orbital_period / (orbital_period - rotation_period))
}

/// Speed at which the terminator sweeps across the equator, m/s, for a body
/// of the given radius (m) and solar day (s): one equatorial circumference
/// per solar day.
pub fn terminator_drift_speed(radius: f64, solar_day: f64) -> f64 {
    2.0 * PI * radius / solar_day
}

/// Sunrises seen per orbit from a fixed point on the surface of a body that
/// spins `rotations_per_orbit` times per orbit (prograde, spin axis normal to
/// the orbit). Seen from the surface, the star circles the local sky
/// `|k - 1|` times per orbit — the same subtraction as [`solar_day`], recast
/// as a dimensionless ratio so it also covers the two cases a finite rotation
/// period cannot express: a 1:1 lock scores exactly zero (the star never
/// rises or sets), and a body with no spin at all (`k = 0`, infinite rotation
/// period) still scores one sunrise per orbit. Mercury's 3:2 scores one every
/// second orbit.
pub fn sunrises_per_orbit(rotations_per_orbit: f64) -> f64 {
    (rotations_per_orbit - 1.0).abs()
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
    fn a_body_one_ulp_off_lock_is_not_reported_as_locked() {
        // The reciprocal-difference form fails here: 1/P_rot - 1/P_orb
        // underflows to exactly zero for periods this close, so a body that is
        // merely very nearly locked comes back as locked. Only exact equality
        // of the inputs may return None.
        let p = 11.2 * DAY;
        let just_off = f64::from_bits(p.to_bits() + 1);
        assert!(just_off > p);
        assert_eq!(
            1.0 / p - 1.0 / just_off,
            0.0,
            "premise: the old form cancels"
        );

        let s = solar_day(p, just_off).expect("not locked, so it has a solar day");
        assert!(s.is_finite() && s > 0.0, "solar day {s}");
        // A period difference of one part in 2^52 gives a solar day of that
        // same order times the period: astronomically long, but a number.
        assert_close(s, p * p / (just_off - p), 1e-9);
    }

    #[test]
    fn nearly_locked_periods_keep_their_precision() {
        // A body a part-per-billion off lock. The subtraction form loses most
        // of its digits here; the product form does not.
        let p = 11.2 * DAY;
        let orb = p * (1.0 + 1e-9);
        let s = solar_day(p, orb).unwrap();
        assert_close(s, p * orb / (orb - p), 1e-12);
        // Sanity: roughly the period divided by the fractional offset.
        assert_close(s, p / 1e-9, 1e-6);
    }

    #[test]
    fn lock_plate_spin_rules() {
        // The three modes of the lock plate on "know your planet". A 1:1 lock
        // is the exact zero — an assertion about the model, so no tolerance —
        // while no spin still delivers a sunrise every orbit, and a 3:2
        // resonance one every second orbit.
        assert_eq!(sunrises_per_orbit(1.0), 0.0);
        assert_eq!(sunrises_per_orbit(0.0), 1.0);
        assert_eq!(sunrises_per_orbit(1.5), 0.5);
    }

    #[test]
    fn three_two_resonance_on_reference_planet_has_a_two_orbit_solar_day() {
        // Impose Mercury's 3:2 on the reference planet's 11.2-day orbit:
        // rotation period 2/3 of the orbit. Both routes must agree — the
        // period form and the ratio form are the same subtraction.
        let orb = 11.2 * DAY;
        let s = solar_day(orb * 2.0 / 3.0, orb).unwrap();
        assert_close(s, 22.4 * DAY, 1e-12); // exactly two orbits
        assert_close(s, orb / sunrises_per_orbit(1.5), 1e-12);
    }

    #[test]
    fn mercury_terminator_drifts_at_walking_pace() {
        // Radius 2,439.7 km; ~1 m/s at the equator.
        let s = solar_day(58.646 * DAY, 87.969 * DAY).unwrap();
        assert_close(terminator_drift_speed(2.4397e6, s), 1.0085, 1e-3);
    }
}
