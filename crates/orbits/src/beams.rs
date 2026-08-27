//! Spot-beam geometry: range rate and Doppler seen by ground users, and how
//! the timing/Doppler spread collapses when a beam covers only a small spot.
//!
//! In-plane geometry for a circular orbit: the ground user sits at central
//! angle `ground_angle` (rad) from the sub-satellite point, in the orbit
//! plane. Rates use the orbital angular rate; the planet's own rotation
//! (11.2 days vs ~2 h orbits for the reference planet) is neglected.

use crate::circular::orbital_period;
use crate::placement::SPEED_OF_LIGHT;
use crate::CentralBody;

/// Slant range (m) from a ground user at central angle `ground_angle` (rad)
/// to a satellite at `altitude` (m).
pub fn slant_range(body: &CentralBody, altitude: f64, ground_angle: f64) -> f64 {
    let r = body.radius + altitude;
    let big_r = body.radius;
    (big_r * big_r + r * r - 2.0 * big_r * r * ground_angle.cos()).sqrt()
}

/// Rate of change of slant range (m/s, positive receding) for an in-plane
/// user at central angle `ground_angle` (rad).
pub fn range_rate(body: &CentralBody, altitude: f64, ground_angle: f64) -> f64 {
    let r = body.radius + altitude;
    let omega = 2.0 * std::f64::consts::PI / orbital_period(body, altitude);
    body.radius * r * omega * ground_angle.sin() / slant_range(body, altitude, ground_angle)
}

/// Doppler shift magnitude (Hz) at carrier `frequency` for a given
/// `range_rate` (m/s).
pub fn doppler_shift(range_rate: f64, frequency: f64) -> f64 {
    range_rate / SPEED_OF_LIGHT * frequency
}

/// Ground radius (m) of the spot painted by a satellite beam of full width
/// `beamwidth` (rad) pointed at nadir from `altitude`.
pub fn nadir_spot_radius(altitude: f64, beamwidth: f64) -> f64 {
    altitude * (beamwidth / 2.0).tan()
}

/// Doppler spread (Hz) across a spot of `spot_radius` (m) centered at
/// `center_angle` (rad): the difference between the shifts seen at the
/// spot's near and far edges along the orbit track.
pub fn doppler_spread_across_spot(
    body: &CentralBody,
    altitude: f64,
    center_angle: f64,
    spot_radius: f64,
    frequency: f64,
) -> f64 {
    let d = spot_radius / body.radius;
    doppler_shift(range_rate(body, altitude, center_angle + d), frequency)
        - doppler_shift(range_rate(body, altitude, center_angle - d), frequency)
}

/// Propagation-delay spread (s) across a spot of `spot_radius` (m) centered
/// at `center_angle` (rad).
pub fn delay_spread_across_spot(
    body: &CentralBody,
    altitude: f64,
    center_angle: f64,
    spot_radius: f64,
) -> f64 {
    let d = spot_radius / body.radius;
    (slant_range(body, altitude, center_angle + d) - slant_range(body, altitude, center_angle - d))
        / SPEED_OF_LIGHT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage::footprint_radius;

    const MIN_ELEVATION: f64 = 25.0 * std::f64::consts::PI / 180.0;
    const KA: f64 = 30e9;

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
    fn overhead_user_sees_zero_range_rate() {
        let p = reference_planet();
        assert!(range_rate(&p, 2_200e3, 0.0).abs() < 1e-9);
    }

    #[test]
    fn edge_user_sees_max_range_rate_and_doppler() {
        // At the 25°-elevation coverage edge of the 2,200 km shell the
        // range rate is ≈4.59 km/s: ≈460 kHz of Doppler at Ka band.
        let p = reference_planet();
        let edge = footprint_radius(&p, 2_200e3, MIN_ELEVATION) / p.radius;
        let rate = range_rate(&p, 2_200e3, edge);
        assert_close(rate, 4_594.0, 1e-3);
        assert_close(doppler_shift(rate, KA), 4.597e5, 1e-3);
    }

    #[test]
    fn one_degree_beam_paints_a_19_km_spot() {
        assert_close(
            nadir_spot_radius(2_200e3, 1.0_f64.to_radians()),
            1.92e4,
            1e-3,
        );
    }

    #[test]
    fn spot_collapses_doppler_and_delay_spread() {
        // Worst-case spot at the coverage edge, 19.2 km radius: ~2.3 kHz of
        // Doppler spread and ~116 µs of delay spread across the whole spot —
        // versus ±460 kHz and a multi-millisecond window across the full
        // footprint.
        let p = reference_planet();
        let edge = footprint_radius(&p, 2_200e3, MIN_ELEVATION) / p.radius;
        let spot = 1.92e4;
        assert_close(
            doppler_spread_across_spot(&p, 2_200e3, edge, spot, KA),
            2.25e3,
            2e-2,
        );
        assert_close(
            delay_spread_across_spot(&p, 2_200e3, edge, spot),
            1.16e-4,
            2e-2,
        );
    }
}
