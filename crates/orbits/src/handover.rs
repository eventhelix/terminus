//! Which satellite serves a ground point, and how often that has to change.
//!
//! Selection is parameterized by one number, the hysteresis margin: a ground
//! point holds its satellite until that satellite falls below
//! `min_elevation - hysteresis`, then takes the highest one in view. Zero
//! margin is greedy selection — always the highest satellite.
//!
//! Two results this module exists to pin down, both counter to the obvious
//! intuition (see the tests, and the `handover_cadence` example):
//!
//! - **Greedy selection does not churn** in a filed constellation. The
//!   in-plane successor rises as the incumbent sets, so the highest
//!   satellite changes exactly when the incumbent would have been dropped
//!   anyway. At a dense baseline the two policies produce the same handover
//!   count and never a return to a satellite just left. Hysteresis is a
//!   guard against noisy elevation estimates, not a cure for ping-pong.
//! - **The cadence is the in-plane spacing**, `period / sats_per_plane`, not
//!   the pass duration from [`crate::coverage::max_pass_duration`]. A town
//!   is handed from one satellite to the next in the same plane long before
//!   any of them sets, so a shell's handover rate is set by how closely its
//!   satellites are filed, not by how long one of them could serve.
//!
//! Where footprints barely overlap the trade inverts: holding a sinking
//! satellite means landing on whatever is left when it finally goes, and
//! handovers go up. Hysteresis is a knob to be set per shell, not a default.

use crate::constellation::{elevation, polar_sat_position, PolarConstellation};
use crate::CentralBody;

/// How a ground point chooses, and keeps, its serving satellite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandoverPolicy {
    /// Elevation (rad) below which a satellite is unusable.
    pub min_elevation: f64,
    /// Extra margin (rad) the serving satellite is allowed to sink below
    /// `min_elevation` before it is dropped. Zero means greedy selection.
    pub hysteresis: f64,
}

impl HandoverPolicy {
    /// Sticky selection with the given usable-elevation floor and margin.
    pub fn sticky(min_elevation: f64, hysteresis: f64) -> Self {
        Self {
            min_elevation,
            hysteresis,
        }
    }

    /// Always take the highest satellite in view.
    pub fn greedy(min_elevation: f64) -> Self {
        Self {
            min_elevation,
            hysteresis: 0.0,
        }
    }
}

/// One satellite of a polar constellation, by plane and slot.
pub type SatelliteId = (usize, usize);

/// A change of serving satellite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandoverEvent {
    /// Time (s) at which the ground point moved to `to`.
    pub time: f64,
    /// The satellite that was serving, or `None` for the first acquisition.
    pub from: Option<SatelliteId>,
    pub to: SatelliteId,
}

fn sat_elevation(
    body: &CentralBody,
    c: &PolarConstellation,
    id: SatelliteId,
    ground_unit: [f64; 3],
    t: f64,
) -> f64 {
    let (k, j) = id;
    let raan = k as f64 * std::f64::consts::PI / c.planes as f64;
    let theta0 =
        j as f64 * 2.0 * std::f64::consts::PI / c.sats_per_plane as f64 + k as f64 * c.interplane_phase;
    elevation(
        body,
        ground_unit,
        polar_sat_position(body, c.altitude, raan, theta0, t),
    )
}

/// Highest satellite at or above `min_elevation`, if any.
pub fn best_visible(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    min_elevation: f64,
    t: f64,
) -> Option<(SatelliteId, f64)> {
    let mut best: Option<(SatelliteId, f64)> = None;
    for k in 0..c.planes {
        for j in 0..c.sats_per_plane {
            let e = sat_elevation(body, c, (k, j), ground_unit, t);
            if e >= min_elevation && best.map_or(true, |(_, be)| e > be) {
                best = Some(((k, j), e));
            }
        }
    }
    best
}

/// Serving-satellite changes over `duration` (s), sampled every `step` (s).
///
/// The first entry is the initial acquisition (`from: None`); every later
/// entry is a handover.
pub fn handover_timeline(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    policy: HandoverPolicy,
    duration: f64,
    step: f64,
) -> Vec<HandoverEvent> {
    let drop_below = policy.min_elevation - policy.hysteresis;
    let mut events = Vec::new();
    let mut current: Option<SatelliteId> = None;
    let mut t = 0.0;
    while t <= duration {
        let hold = current
            .map(|id| sat_elevation(body, c, id, ground_unit, t) >= drop_below)
            .unwrap_or(false);
        if !hold {
            if let Some((id, _)) = best_visible(body, c, ground_unit, policy.min_elevation, t) {
                if current != Some(id) {
                    events.push(HandoverEvent {
                        time: t,
                        from: current,
                        to: id,
                    });
                    current = Some(id);
                }
            } else {
                current = None;
            }
        }
        t += step;
    }
    events
}

/// Number of handovers (excluding the first acquisition) over `duration`.
pub fn handover_count(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    policy: HandoverPolicy,
    duration: f64,
    step: f64,
) -> usize {
    handover_timeline(body, c, ground_unit, policy, duration, step)
        .iter()
        .filter(|e| e.from.is_some())
        .count()
}

/// Mean interval (s) between handovers — how long a link lasts in practice.
/// Returns `None` if the ground point never handed over.
pub fn mean_service_interval(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    policy: HandoverPolicy,
    duration: f64,
    step: f64,
) -> Option<f64> {
    let events = handover_timeline(body, c, ground_unit, policy, duration, step);
    if events.len() < 2 {
        return None;
    }
    let first = events.first().unwrap().time;
    let last = events.last().unwrap().time;
    Some((last - first) / (events.len() - 1) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circular::orbital_period;
    use crate::constellation::band_point;
    use crate::coverage::max_pass_duration;

    const MIN_ELEVATION: f64 = 25.0 * std::f64::consts::PI / 180.0;
    const HYSTERESIS: f64 = 3.0 * std::f64::consts::PI / 180.0;

    fn reference_planet() -> CentralBody {
        CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0)
    }

    fn baseline() -> PolarConstellation {
        PolarConstellation {
            altitude: 2_200e3,
            planes: 6,
            sats_per_plane: 12,
            interplane_phase: 0.0,
        }
    }

    fn towns() -> [[f64; 3]; 5] {
        let edge = 20.0_f64.to_radians();
        [
            band_point(0.7, 0.0),
            band_point(2.4, 0.0),
            band_point(5.0, 0.0),
            band_point(1.3, edge),
            band_point(3.9, -edge),
        ]
    }

    #[test]
    fn cadence_is_the_in_plane_spacing_not_the_pass_duration() {
        // A town is not given a whole pass. It is handed from one satellite
        // to the next in the same plane as they file past, so the interval
        // between handovers is the plane's own inter-satellite spacing —
        // period / sats_per_plane — which is well under the best-case
        // zenith pass.
        let p = reference_planet();
        let c = baseline();
        let spacing = orbital_period(&p, c.altitude) / c.sats_per_plane as f64;
        let policy = HandoverPolicy::sticky(MIN_ELEVATION, HYSTERESIS);
        for town in towns() {
            let interval = mean_service_interval(&p, &c, town, policy, 6.0 * 3_600.0, 5.0)
                .expect("a band town hands over many times in six hours");
            let rel = ((interval - spacing) / spacing).abs();
            assert!(
                rel < 0.05,
                "interval {interval} s is not the {spacing} s in-plane spacing (rel {rel})"
            );
        }
        // ≈11.0 min, against a 16.6 min zenith pass.
        assert!(spacing < max_pass_duration(&p, c.altitude, MIN_ELEVATION));
        assert!((spacing - 658.0).abs() < 10.0, "spacing {spacing} s");
    }

    #[test]
    fn selection_is_stable_without_hysteresis_at_the_baseline() {
        // Worth pinning because the intuition runs the other way: with 72
        // satellites overhead, "always attach to the highest" sounds like it
        // would churn. It does not. In a filed queue of satellites the
        // in-plane successor rises as the incumbent sets, so the highest
        // satellite changes exactly when the incumbent would have been
        // dropped anyway — same count, and never a return to a satellite
        // just left. Hysteresis is a guard here, not a fix.
        let p = reference_planet();
        let c = baseline();
        let duration = 6.0 * 3_600.0;
        for town in towns() {
            let greedy =
                handover_timeline(&p, &c, town, HandoverPolicy::greedy(MIN_ELEVATION), duration, 5.0);
            let sticky = handover_timeline(
                &p,
                &c,
                town,
                HandoverPolicy::sticky(MIN_ELEVATION, HYSTERESIS),
                duration,
                5.0,
            );
            assert_eq!(
                greedy.iter().filter(|e| e.from.is_some()).count(),
                sticky.iter().filter(|e| e.from.is_some()).count(),
                "greedy and sticky should hand over the same number of times"
            );
            // No ping-pong: never back to the satellite served two changes ago.
            assert_eq!(greedy.windows(3).filter(|w| w[0].to == w[2].to).count(), 0);
        }
    }

    #[test]
    fn hysteresis_costs_handovers_where_coverage_is_thin() {
        // The other side of the trade, and the reason hysteresis is a knob
        // rather than a default: at an altitude whose footprints barely
        // overlap, holding a sinking satellite past the floor means landing
        // on whatever is left when it finally goes — often another satellite
        // on its way down. Handovers go up, not down.
        let p = reference_planet();
        let thin = PolarConstellation {
            altitude: 1_200e3,
            ..baseline()
        };
        let duration = 6.0 * 3_600.0;
        let mut greedy_total = 0;
        let mut sticky_total = 0;
        for town in towns() {
            greedy_total += handover_count(
                &p,
                &thin,
                town,
                HandoverPolicy::greedy(MIN_ELEVATION),
                duration,
                5.0,
            );
            sticky_total += handover_count(
                &p,
                &thin,
                town,
                HandoverPolicy::sticky(MIN_ELEVATION, HYSTERESIS),
                duration,
                5.0,
            );
        }
        assert!(
            sticky_total > greedy_total,
            "sticky {sticky_total} should cost more than greedy {greedy_total} at 1,200 km"
        );
    }

    #[test]
    fn a_chosen_satellite_is_always_above_the_floor() {
        let p = reference_planet();
        let c = baseline();
        let town = band_point(1.9, 0.1);
        let policy = HandoverPolicy::sticky(MIN_ELEVATION, HYSTERESIS);
        let events = handover_timeline(&p, &c, town, policy, 3.0 * 3_600.0, 10.0);
        assert!(events.len() > 2);
        for e in &events {
            let elev = sat_elevation(&p, &c, e.to, town, e.time);
            assert!(
                elev >= policy.min_elevation - 1e-9,
                "chose a satellite at {elev} rad, below the {} rad floor",
                policy.min_elevation
            );
        }
    }
}
