//! Which ring is on duty, and whether one ring could ever do the job alone.
//!
//! The access wheel's planes are pinned to the stars while the terminator
//! sweeps past them, so at any instant one ring lies closest to the twilight
//! band. Calling that ring "on duty" is a useful scheduling idea: it names the
//! ring doing the most work, and it changes on a slow, predictable beat.
//!
//! It is not, however, a coverage mechanism. The inhabited band has width, and
//! the duty ring is exactly aligned only for an instant. A band point can sit
//!
//! ```text
//! band_halfwidth + max_duty_misalignment
//! ```
//!
//! away from the duty ring's ground track, and if that exceeds a satellite's
//! footprint half-angle the ring cannot reach the point at all. That is a
//! reach failure rather than a spacing failure, so adding satellites to the
//! ring does not fix it: [`min_sats_per_ring_for_duty_only`] returns `None` in
//! exactly that case. Serving the whole band takes the neighbouring rings too.

use crate::constellation::{plane_visible_count, PolarConstellation};
use crate::CentralBody;

use std::f64::consts::PI;

/// Index of the ring whose plane currently lies closest to the terminator.
pub fn duty_ring(body: &CentralBody, c: &PolarConstellation, t: f64) -> usize {
    (0..c.planes)
        .min_by(|&a, &b| {
            misalignment_of(body, c, a, t)
                .partial_cmp(&misalignment_of(body, c, b, t))
                .expect("finite misalignment")
        })
        .expect("at least one plane")
}

/// Angle (rad) between plane `k` and the terminator plane, in `[0, pi/2]`.
pub fn misalignment_of(body: &CentralBody, c: &PolarConstellation, k: usize, t: f64) -> f64 {
    let d = (c.node(body, k, t) - PI / 2.0).rem_euclid(PI);
    d.min(PI - d)
}

/// Angle (rad) between the duty ring and the terminator at time `t`. Zero for
/// an instant mid-shift, worst at handover.
pub fn duty_misalignment(body: &CentralBody, c: &PolarConstellation, t: f64) -> f64 {
    misalignment_of(body, c, duty_ring(body, c, t), t)
}

/// Worst misalignment (rad) the duty ring ever suffers: half the spacing
/// between adjacent planes, since nodes are spread over 180 degrees.
///
/// It is reached exactly at handover, when the terminator lies midway between
/// the ring going off duty and the one coming on.
pub fn max_duty_misalignment(planes: usize) -> f64 {
    PI / (2.0 * planes as f64)
}

/// Greatest cross-track distance (rad) between a band point and the duty
/// ring's ground track: the band's own half-width plus the worst misalignment.
///
/// The two add at the band edge, over the terminator's equator crossings.
pub fn worst_cross_track(band_halfwidth: f64, planes: usize) -> f64 {
    band_halfwidth + max_duty_misalignment(planes)
}

/// Whether the duty ring alone could cover the band, ignoring satellite count:
/// does one satellite's footprint reach as far as [`worst_cross_track`]?
pub fn duty_only_reaches_band(
    body: &CentralBody,
    c: &PolarConstellation,
    band_halfwidth: f64,
    min_elevation: f64,
) -> bool {
    c.footprint_half_angle(body, min_elevation) > worst_cross_track(band_halfwidth, c.planes)
}

/// Satellites per ring a *strict* duty-ring design would need, or `None` when
/// no number suffices because the ring cannot reach the band's far edge.
///
/// Where the ring does reach, a point at cross-track `d` is covered over an
/// along-orbit arc of `+/- acos(cos lambda / cos d)`, so the satellites must
/// be spaced no wider than twice that.
pub fn min_sats_per_ring_for_duty_only(
    body: &CentralBody,
    c: &PolarConstellation,
    band_halfwidth: f64,
    min_elevation: f64,
) -> Option<usize> {
    if !duty_only_reaches_band(body, c, band_halfwidth, min_elevation) {
        return None;
    }
    let lambda = c.footprint_half_angle(body, min_elevation);
    let d = worst_cross_track(band_halfwidth, c.planes);
    let half_window = (lambda.cos() / d.cos()).acos();
    Some((PI / half_window).ceil() as usize)
}

/// How many distinct rings can reach `ground_unit` at all at time `t`,
/// regardless of where their satellites happen to be.
pub fn rings_in_reach(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    min_elevation: f64,
    t: f64,
) -> usize {
    let lambda = c.footprint_half_angle(body, min_elevation);
    (0..c.planes)
        .filter(|&k| {
            let node = c.node(body, k, t);
            let a = ground_unit[0] * node.cos() + ground_unit[1] * node.sin();
            let b = ground_unit[2];
            (a * a + b * b).sqrt() > lambda.cos()
        })
        .count()
}

/// How many distinct rings actually have a satellite up at `ground_unit`.
pub fn rings_serving(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    phases: &[f64],
    min_elevation: f64,
    t: f64,
) -> usize {
    (0..c.planes)
        .filter(|&k| plane_visible_count(body, c, k, ground_unit, phases[k], min_elevation, t) > 0)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::band_point;

    const MIN_ELEVATION: f64 = 25.0 * PI / 180.0;
    const BAND: f64 = 20.0 * PI / 180.0;

    fn reference_planet() -> CentralBody {
        CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0)
    }

    fn baseline(altitude: f64, sats: usize) -> PolarConstellation {
        PolarConstellation {
            altitude,
            planes: 6,
            sats_per_plane: sats,
            interplane_phase: 0.0,
        }
    }

    #[test]
    fn six_rings_put_the_duty_ring_at_most_fifteen_degrees_off() {
        assert!((max_duty_misalignment(6).to_degrees() - 15.0).abs() < 1e-12);
        assert!((worst_cross_track(BAND, 6).to_degrees() - 35.0).abs() < 1e-12);
    }

    #[test]
    fn duty_misalignment_swings_between_zero_and_the_half_spacing() {
        let p = reference_planet();
        let c = baseline(2_200e3, 12);
        let (mut lo, mut hi) = (f64::MAX, 0.0f64);
        let mut t = 0.0;
        while t < 11.2 * 86_400.0 {
            let m = duty_misalignment(&p, &c, t);
            lo = lo.min(m);
            hi = hi.max(m);
            t += 60.0;
        }
        assert!(lo.to_degrees() < 0.1, "min misalignment {}", lo.to_degrees());
        assert!(
            (hi.to_degrees() - 15.0).abs() < 0.1,
            "max misalignment {}",
            hi.to_degrees()
        );
    }

    #[test]
    fn the_baseline_duty_ring_cannot_reach_the_band_edge_at_any_count() {
        let p = reference_planet();
        for sats in [12, 24, 48, 96] {
            let c = baseline(2_200e3, sats);
            assert!(!duty_only_reaches_band(&p, &c, BAND, MIN_ELEVATION));
            assert_eq!(
                min_sats_per_ring_for_duty_only(&p, &c, BAND, MIN_ELEVATION),
                None
            );
        }
    }

    #[test]
    fn a_strict_duty_ring_needs_a_far_higher_shelf() {
        let p = reference_planet();
        // Below ~5,200 km the footprint never spans the 35 deg reach.
        assert!(!duty_only_reaches_band(
            &p,
            &baseline(5_000e3, 12),
            BAND,
            MIN_ELEVATION
        ));
        assert!(duty_only_reaches_band(
            &p,
            &baseline(5_400e3, 12),
            BAND,
            MIN_ELEVATION
        ));
        // And once it does reach, the count it needs is modest.
        assert_eq!(
            min_sats_per_ring_for_duty_only(&p, &baseline(7_300e3, 12), BAND, MIN_ELEVATION),
            Some(9)
        );
    }

    #[test]
    fn how_many_rings_reach_a_town_depends_on_its_latitude() {
        // Polar planes are 30 deg apart at the equator but all converge at the
        // poles, so the wheel is a two-ring affair over the equator and a
        // six-ring pile-up over the caps. Neither "one ring serves" nor "three
        // rings serve" describes it.
        let p = reference_planet();
        let c = baseline(2_200e3, 12);
        let (mut low_max, mut high_min) = (0usize, usize::MAX);
        let mut ever_alone = false;
        let mut t = 0.0;
        while t < 11.2 * 86_400.0 {
            for i in 0..72 {
                let az = i as f64 * 2.0 * PI / 72.0;
                for off in [-BAND, 0.0, BAND] {
                    let g = band_point(az, off);
                    let lat = g[2].asin().to_degrees().abs();
                    let reach = rings_in_reach(&p, &c, g, MIN_ELEVATION, t);
                    assert!(reach > 0, "a band point had no ring in reach at t={t}");
                    if lat < 30.0 {
                        low_max = low_max.max(reach);
                        ever_alone |= reach == 1;
                    }
                    if lat > 70.0 {
                        high_min = high_min.min(reach);
                    }
                }
            }
            t += 120.0;
        }
        assert_eq!(low_max, 2, "equatorial towns see at most two rings");
        assert!(ever_alone, "equatorial towns are sometimes down to one ring");
        assert_eq!(high_min, c.planes, "polar towns always see every ring");
    }
}
