//! Spot-beam geometry: range rate and Doppler seen by ground users, and how
//! the timing/Doppler spread collapses when a beam covers only a small spot.
//!
//! In-plane geometry for a circular orbit: the ground user sits at central
//! angle `ground_angle` (rad) from the sub-satellite point, in the orbit
//! plane. Rates use the orbital angular rate; the planet's own rotation
//! (11.2 days vs ~2 h orbits for the reference planet) is neglected.

use crate::circular::{orbital_period, orbital_velocity};
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

/// Signed Doppler shift (Hz) actually received on the ground: positive when
/// the satellite approaches (`range_rate` negative) — the received frequency
/// sits *above* the carrier, a blue shift. Negative when it recedes: red.
pub fn received_doppler(range_rate: f64, frequency: f64) -> f64 {
    -range_rate / SPEED_OF_LIGHT * frequency
}

/// Ground position anywhere in the footprint, as angular offsets (rad) from
/// the sub-satellite point: `along_track` positive *ahead* of the satellite
/// in its direction of motion, `cross_track` perpendicular to the track.
fn ground_position(body: &CentralBody, along_track: f64, cross_track: f64) -> [f64; 3] {
    // Orbit plane = x–z plane, satellite at (0, 0, R+h) moving toward +x.
    let (r, a, c) = (body.radius, along_track, cross_track);
    [r * a.sin() * c.cos(), r * c.sin(), r * a.cos() * c.cos()]
}

fn dot(u: [f64; 3], v: [f64; 3]) -> f64 {
    u[0] * v[0] + u[1] * v[1] + u[2] * v[2]
}

/// Slant range (m) from a ground user at (`along_track`, `cross_track`)
/// angular offsets (rad) to the satellite overhead at `altitude` (m).
pub fn slant_range_at(body: &CentralBody, altitude: f64, along_track: f64, cross_track: f64) -> f64 {
    let user = ground_position(body, along_track, cross_track);
    let sat = [0.0, 0.0, body.radius + altitude];
    let los = [sat[0] - user[0], sat[1] - user[1], sat[2] - user[2]];
    dot(los, los).sqrt()
}

/// Rate of change of slant range (m/s, positive receding) for a ground user
/// anywhere in the footprint: the satellite's velocity vector projected onto
/// the line of sight — one dot product, `v · r̂`.
pub fn range_rate_at(body: &CentralBody, altitude: f64, along_track: f64, cross_track: f64) -> f64 {
    let user = ground_position(body, along_track, cross_track);
    let sat = [0.0, 0.0, body.radius + altitude];
    let omega = 2.0 * std::f64::consts::PI / orbital_period(body, altitude);
    let velocity = [(body.radius + altitude) * omega, 0.0, 0.0];
    let los = [sat[0] - user[0], sat[1] - user[1], sat[2] - user[2]];
    let slant = dot(los, los).sqrt();
    let los_unit = [los[0] / slant, los[1] / slant, los[2] / slant];
    dot(velocity, los_unit)
}

/// Ground radius (m) of the spot painted by a satellite beam of full width
/// `beamwidth` (rad) pointed at nadir from `altitude`.
pub fn nadir_spot_radius(altitude: f64, beamwidth: f64) -> f64 {
    altitude * (beamwidth / 2.0).tan()
}

/// Nadir angle (rad) at the satellite — how far off straight-down it must
/// look — to see a ground user at central angle `ground_angle`.
pub fn nadir_angle(body: &CentralBody, altitude: f64, ground_angle: f64) -> f64 {
    let r = body.radius + altitude;
    (body.radius * ground_angle.sin()).atan2(r - body.radius * ground_angle.cos())
}

/// Ground central angle (rad) where a ray leaving the satellite `nadir_angle`
/// off straight-down strikes the surface (the near intersection).
pub fn ray_ground_angle(body: &CentralBody, altitude: f64, nadir_angle: f64) -> f64 {
    let r = body.radius + altitude;
    let sin_user = (r / body.radius) * nadir_angle.sin();
    // The interior angle at the user is obtuse for the near intersection.
    let user = std::f64::consts::PI - sin_user.min(1.0).asin();
    std::f64::consts::PI - nadir_angle - user
}

/// In-plane (radial) half-extent (m) of the ground spot painted by a beam of
/// full width `beamwidth` (rad) aimed at a spot center at `center_angle`.
///
/// A leaning beam's spot elongates for three stacking reasons: it lands
/// *farther* (longer slant range), *flatter* (oblique incidence, ~1/sin ε),
/// and *fatter* (a planar array's beam broadens by 1/cos of the scan angle,
/// which for a nadir-mounted array is the nadir angle η). At the reference
/// footprint edge the product is ~5.3×: a 19 km nadir radius becomes ±102 km.
pub fn spot_half_extent(body: &CentralBody, altitude: f64, center_angle: f64, beamwidth: f64) -> f64 {
    let eta = nadir_angle(body, altitude, center_angle);
    let broadened = beamwidth / eta.cos();
    body.radius
        * (ray_ground_angle(body, altitude, eta + broadened / 2.0)
            - ray_ground_angle(body, altitude, eta - broadened / 2.0))
        / 2.0
}

/// Cross-track half-extent (m) of the same spot: the slant range times the
/// half beamwidth (no scan broadening or obliquity in that plane).
pub fn spot_cross_half_extent(
    body: &CentralBody,
    altitude: f64,
    center_angle: f64,
    beamwidth: f64,
) -> f64 {
    slant_range(body, altitude, center_angle) * (beamwidth / 2.0).tan()
}

/// Doppler spread (Hz) across the spot of *any* beam of full width
/// `beamwidth` from the array — independent of where the beam points.
///
/// In-plane the received shift is (f/c)·v·sin η, so its slope per unit of
/// beam angle is (f/c)·v·cos η — and the beam is broadened by exactly
/// 1/cos η, so the product collapses to (f/c)·v·β for every spot: a beam's
/// Doppler spread is set by its width alone.
pub fn beam_doppler_spread(body: &CentralBody, altitude: f64, beamwidth: f64, frequency: f64) -> f64 {
    orbital_velocity(body, altitude) * beamwidth / SPEED_OF_LIGHT * frequency
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
        // A FIXED 19.2 km ground extent at the coverage edge: ~2.3 kHz of
        // Doppler spread and ~116 µs of delay spread — versus ±460 kHz and
        // a multi-millisecond window across the full footprint. The real
        // beam's spot there is 5.3× longer (spot_half_extent); these are
        // the per-kilometer field gradients the maps' contours show.
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

    #[test]
    fn dot_product_form_matches_the_in_plane_closed_form() {
        // range_rate_at computes v·r̂ with explicit vectors; range_rate is
        // the in-plane closed form R·r·ω·sinγ/slant. On the orbit track they
        // must agree exactly. The closed form's positive angle is a receding
        // satellite, i.e. a user *behind* it: along_track = −γ.
        let p = reference_planet();
        let edge = footprint_radius(&p, 2_200e3, MIN_ELEVATION) / p.radius;
        for frac in [0.05, 0.25, 0.5, 0.75, 1.0] {
            let g = edge * frac;
            assert_close(range_rate_at(&p, 2_200e3, -g, 0.0), range_rate(&p, 2_200e3, g), 1e-9);
            assert_close(range_rate_at(&p, 2_200e3, g, 0.0), -range_rate(&p, 2_200e3, g), 1e-9);
            assert_close(
                slant_range_at(&p, 2_200e3, g, 0.0),
                slant_range(&p, 2_200e3, g),
                1e-9,
            );
        }
    }

    #[test]
    fn cross_track_axis_sees_zero_doppler() {
        // A user displaced purely cross-track sees a line of sight with no
        // component along the velocity: v·r̂ = 0. This is the map's zero
        // (iso-Doppler) line through the sub-satellite point.
        let p = reference_planet();
        let edge = footprint_radius(&p, 2_200e3, MIN_ELEVATION) / p.radius;
        for frac in [0.25, 0.5, 1.0] {
            assert!(range_rate_at(&p, 2_200e3, 0.0, edge * frac).abs() < 1e-9);
        }
    }

    #[test]
    fn slant_range_depends_only_on_the_central_angle() {
        // Iso-delay contours are concentric circles: any (along, cross) pair
        // with the same central angle has the same slant range.
        let p = reference_planet();
        let g = 0.2;
        let along = slant_range_at(&p, 2_200e3, g, 0.0);
        let cross = slant_range_at(&p, 2_200e3, 0.0, g);
        assert_close(along, cross, 1e-9);
        assert_close(along, slant_range(&p, 2_200e3, g), 1e-9);
    }

    #[test]
    fn received_doppler_is_blue_ahead_and_red_behind() {
        // Ahead of the satellite (positive along-track) the range closes:
        // negative range rate, positive received shift — a blue shift.
        let p = reference_planet();
        let ahead = range_rate_at(&p, 2_200e3, 0.1, 0.0);
        assert!(ahead < 0.0);
        assert!(received_doppler(ahead, KA) > 0.0);
        let behind = range_rate_at(&p, 2_200e3, -0.1, 0.0);
        assert!(received_doppler(behind, KA) < 0.0);
    }

    #[test]
    fn doppler_gradient_is_steepest_at_nadir() {
        // For a FIXED ground extent (±19.2 km), Doppler spread peaks at
        // nadir (~11.9 kHz) where the shift sweeps steeply through zero,
        // and falls to 2.25 kHz at the edge near the curve's stationary
        // maximum. Real beams elongate toward the edge by exactly the
        // inverse factor — see beam_doppler_spread_is_the_same_everywhere.
        let p = reference_planet();
        let edge = footprint_radius(&p, 2_200e3, MIN_ELEVATION) / p.radius;
        let spot = 1.92e4;
        let nadir = doppler_spread_across_spot(&p, 2_200e3, 0.0, spot, KA);
        assert_close(nadir, 1.191e4, 2e-2);
        for frac in [0.1, 0.25, 0.5, 0.75, 1.0] {
            assert!(doppler_spread_across_spot(&p, 2_200e3, edge * frac, spot, KA) < nadir);
        }
        assert!(delay_spread_across_spot(&p, 2_200e3, 0.0, spot).abs() < 1e-12);
    }

    #[test]
    fn nadir_spot_half_extent_reduces_to_the_nadir_radius() {
        let p = reference_planet();
        assert_close(
            spot_half_extent(&p, 2_200e3, 0.0, 1.0_f64.to_radians()),
            nadir_spot_radius(2_200e3, 1.0_f64.to_radians()),
            1e-2,
        );
        assert_close(
            spot_cross_half_extent(&p, 2_200e3, 0.0, 1.0_f64.to_radians()),
            nadir_spot_radius(2_200e3, 1.0_f64.to_radians()),
            1e-9,
        );
    }

    #[test]
    fn edge_spot_is_farther_flatter_fatter() {
        // At the footprint edge a 1° beam lands 1.66× farther (slant),
        // 2.37× flatter (1/sin 25°), and 1.35× fatter (scan broadening,
        // 1/cos η): ±102 km radial, ±32 km cross-track — 5.3× elongated.
        let p = reference_planet();
        let beam = 1.0_f64.to_radians();
        let edge = footprint_radius(&p, 2_200e3, MIN_ELEVATION) / p.radius;
        let half = spot_half_extent(&p, 2_200e3, edge, beam);
        assert_close(half, 1.020e5, 1e-2);
        assert_close(spot_cross_half_extent(&p, 2_200e3, edge, beam), 3.18e4, 1e-2);
        let farther = slant_range(&p, 2_200e3, edge) / 2_200e3;
        let flatter = 1.0 / MIN_ELEVATION.sin();
        let fatter = 1.0 / nadir_angle(&p, 2_200e3, edge).cos();
        assert_close(
            half / nadir_spot_radius(2_200e3, beam),
            farther * flatter * fatter,
            1e-2,
        );
    }

    #[test]
    fn beam_doppler_spread_is_the_same_everywhere() {
        // The invariant: (f/c)·v·β. The shift's slope per beam angle,
        // (f/c)·v·cos η, and the array's scan broadening, 1/cos η, cancel
        // exactly, so every beam's spot spans the same ~11.9 kHz.
        let p = reference_planet();
        let beam = 1.0_f64.to_radians();
        let edge = footprint_radius(&p, 2_200e3, MIN_ELEVATION) / p.radius;
        let invariant = beam_doppler_spread(&p, 2_200e3, beam, KA);
        assert_close(invariant, 1.190e4, 1e-2);
        for frac in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let center = edge * frac;
            let half = spot_half_extent(&p, 2_200e3, center, beam);
            let field = doppler_spread_across_spot(&p, 2_200e3, center, half, KA).abs();
            assert_close(field, invariant, 1e-2);
        }
    }

    #[test]
    fn delay_spread_grows_to_617_us_at_the_rim() {
        // With the elongated spot, the rim beam's timing spread is 617 µs
        // (residual ±308 µs after precompensation), not the 116 µs a
        // nadir-sized spot would suggest.
        let p = reference_planet();
        let beam = 1.0_f64.to_radians();
        let edge = footprint_radius(&p, 2_200e3, MIN_ELEVATION) / p.radius;
        let half = spot_half_extent(&p, 2_200e3, edge, beam);
        assert_close(delay_spread_across_spot(&p, 2_200e3, edge, half), 6.17e-4, 1e-2);
    }
}
