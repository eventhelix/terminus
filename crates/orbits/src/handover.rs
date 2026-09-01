// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

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

use crate::activation::satellite_index;
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
    phases: &[f64],
    t: f64,
) -> f64 {
    let (k, j) = id;
    let raan = k as f64 * std::f64::consts::PI / c.planes as f64;
    let theta0 = j as f64 * 2.0 * std::f64::consts::PI / c.sats_per_plane as f64 + phases[k];
    elevation(
        body,
        ground_unit,
        polar_sat_position(body, c.altitude, raan, theta0, t),
    )
}

/// Highest satellite at or above `min_elevation`, if any.
///
/// `phases` carries one along-orbit phase (rad) per plane; build it with
/// [`crate::constellation::plane_phases`], or with
/// [`PolarConstellation::uniform_phases`] for the uniform stagger. Selection
/// scans the whole fleet, so which satellite wins genuinely depends on how the
/// rings are phased against each other -- this is not a parameter the caller
/// can skip and still get the same answer.
///
/// `active` is an activation plan from [`crate::activation`], indexed by
/// [`satellite_index`]. `None` considers every satellite; `Some(plan)`
/// considers only the ones the plan has lit, which is the honest question --
/// a satellite that is coasting between shifts cannot take a session, however
/// high it happens to be sitting. The two subsystems answered separately for
/// a long time: `activation` decided who was radiating, `handover` decided who
/// served, and nothing joined them, so a town could be shown attached to a
/// satellite the same plan had switched off.
///
/// Returns `None` when nothing in the allowed set is above the mask. With a
/// plan that is the caller's cue: a plan covers the band *sample points* it was
/// built from, and a town between two of them can briefly see no lit satellite
/// at all.
pub fn best_visible(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    phases: &[f64],
    active: Option<&[bool]>,
    min_elevation: f64,
    t: f64,
) -> Option<(SatelliteId, f64)> {
    let mut best: Option<(SatelliteId, f64)> = None;
    for k in 0..c.planes {
        for j in 0..c.sats_per_plane {
            if let Some(plan) = active {
                if !plan[satellite_index(c, k, j)] {
                    continue;
                }
            }
            let e = sat_elevation(body, c, (k, j), ground_unit, phases, t);
            if e >= min_elevation && best.map_or(true, |(_, be)| e > be) {
                best = Some(((k, j), e));
            }
        }
    }
    best
}

// Every argument here names a distinct physical input -- body, fleet, place,
// phasing, lit set, policy, span, resolution -- and bundling any pair of them
// would invent a concept the model does not have.
#[allow(clippy::too_many_arguments)]
/// Serving-satellite changes over `duration` (s), sampled every `step` (s).
///
/// The first entry is the initial acquisition (`from: None`); every later
/// entry is a handover.
pub fn handover_timeline(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    phases: &[f64],
    active: Option<&[bool]>,
    policy: HandoverPolicy,
    duration: f64,
    step: f64,
) -> Vec<HandoverEvent> {
    // `active` gates ACQUISITION, never RETENTION. A plan says which satellites
    // must be radiating; it does not get to switch one off mid-session, so the
    // incumbent is held on geometry alone and the plan is consulted only when a
    // new satellite has to be chosen. Gating retention as well makes the link
    // chase the planner instead of the sky: at the 2,200 km baseline that more
    // than doubles the handover rate (6/h -> 13/h) purely from plan churn.
    let drop_below = policy.min_elevation - policy.hysteresis;
    let mut events = Vec::new();
    let mut current: Option<SatelliteId> = None;
    let mut t = 0.0;
    while t <= duration {
        let hold = current
            .map(|id| sat_elevation(body, c, id, ground_unit, phases, t) >= drop_below)
            .unwrap_or(false);
        if !hold {
            if let Some((id, _)) = best_visible(
                body,
                c,
                ground_unit,
                phases,
                active,
                policy.min_elevation,
                t,
            ) {
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

// Every argument here names a distinct physical input -- body, fleet, place,
// phasing, lit set, policy, span, resolution -- and bundling any pair of them
// would invent a concept the model does not have.
#[allow(clippy::too_many_arguments)]
/// Number of handovers (excluding the first acquisition) over `duration`.
pub fn handover_count(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    phases: &[f64],
    active: Option<&[bool]>,
    policy: HandoverPolicy,
    duration: f64,
    step: f64,
) -> usize {
    handover_timeline(body, c, ground_unit, phases, active, policy, duration, step)
        .iter()
        .filter(|e| e.from.is_some())
        .count()
}

// Every argument here names a distinct physical input -- body, fleet, place,
// phasing, lit set, policy, span, resolution -- and bundling any pair of them
// would invent a concept the model does not have.
#[allow(clippy::too_many_arguments)]
/// Mean interval (s) between handovers — how long a link lasts in practice.
/// Returns `None` if the ground point never handed over.
pub fn mean_service_interval(
    body: &CentralBody,
    c: &PolarConstellation,
    ground_unit: [f64; 3],
    phases: &[f64],
    active: Option<&[bool]>,
    policy: HandoverPolicy,
    duration: f64,
    step: f64,
) -> Option<f64> {
    let events = handover_timeline(body, c, ground_unit, phases, active, policy, duration, step);
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

    /// A serving satellite must be one the plan actually lit. `activation` and
    /// `handover` answered separately until now -- one decided who was
    /// radiating, the other who served -- so nothing stopped a town being
    /// handed to a satellite the same plan had switched off.
    ///
    /// The gap this test leaves open is deliberate and measured: a plan covers
    /// the band *sample points* it was built from, so a town between two of
    /// them can briefly see no lit satellite. The assertion is therefore
    /// conditional -- when a lit satellite is in view, that is who serves --
    /// and the rate at which none is in view is pinned below.
    #[test]
    fn a_served_satellite_is_one_the_plan_lit() {
        use crate::activation::{covering_satellites, duty_first_activation, satellite_index};
        use crate::duty::duty_ring;

        let p = reference_planet();
        let c = baseline();
        let phases = c.uniform_phases();
        let band = 20.0_f64.to_radians();
        let points: Vec<[f64; 3]> = (0..72)
            .flat_map(|i| {
                let az = i as f64 * 2.0 * std::f64::consts::PI / 72.0;
                [-band, 0.0, band].map(|off| band_point(az, off))
            })
            .collect();
        // Off the plan's own sample grid on purpose: a town does not stand
        // where the planner happened to look.
        let towns: Vec<[f64; 3]> = (0..11)
            .flat_map(|i| {
                let az = 0.137 + i as f64 * 0.5717;
                [0.0, band * 0.5, band, -band * 0.77].map(|off| band_point(az, off))
            })
            .collect();

        let (mut checked, mut unlit_moments) = (0, 0);
        for i in 0..40 {
            let t = (i as f64 / 40.0) * 11.2 * 86_400.0;
            let cov = covering_satellites(&p, &c, &phases, &points, MIN_ELEVATION, t);
            let plan = duty_first_activation(&cov, &c, duty_ring(&p, &c, t), None, true).active;
            for &town in &towns {
                checked += 1;
                match best_visible(&p, &c, town, &phases, Some(&plan), MIN_ELEVATION, t) {
                    Some(((k, j), _)) => assert!(
                        plan[satellite_index(&c, k, j)],
                        "served by a satellite the plan left dark"
                    ),
                    None => unlit_moments += 1,
                }
            }
        }
        // Sampling artefact, not a coverage hole: well under 1% of instants.
        assert!(
            unlit_moments * 100 < checked,
            "no lit satellite in view for {unlit_moments} of {checked} town-instants"
        );
    }

    /// An activation plan must not cost handovers. It decides who is radiating,
    /// and a satellite already carrying a session is one of them -- so the plan
    /// gates which satellite a town may move TO, never how long it keeps the one
    /// it has. Get that wrong and the link re-homes every time the planner
    /// changes its mind, which is churn the sky never asked for.
    #[test]
    fn a_plan_gates_acquisition_but_never_costs_a_handover() {
        use crate::activation::{covering_satellites, duty_first_activation};
        use crate::duty::duty_ring;

        let p = reference_planet();
        let c = baseline();
        let phases = c.uniform_phases();
        let band = 20.0_f64.to_radians();
        let points: Vec<[f64; 3]> = (0..72)
            .flat_map(|i| {
                let az = i as f64 * 2.0 * std::f64::consts::PI / 72.0;
                [-band, 0.0, band].map(|off| band_point(az, off))
            })
            .collect();
        let policy = HandoverPolicy::sticky(MIN_ELEVATION, HYSTERESIS);
        let duration = 6.0 * 3_600.0;
        let plan = duty_first_activation(
            &covering_satellites(&p, &c, &phases, &points, MIN_ELEVATION, 0.0),
            &c,
            duty_ring(&p, &c, 0.0),
            None,
            true,
        )
        .active;

        for town in towns() {
            let free = handover_count(&p, &c, town, &phases, None, policy, duration, 5.0);
            let planned = handover_count(&p, &c, town, &phases, Some(&plan), policy, duration, 5.0);
            assert!(
                planned <= free,
                "a plan cost handovers: {planned} against {free} for the free fleet"
            );
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
        let phases = c.uniform_phases();
        let spacing = orbital_period(&p, c.altitude) / c.sats_per_plane as f64;
        let policy = HandoverPolicy::sticky(MIN_ELEVATION, HYSTERESIS);
        for town in towns() {
            let interval =
                mean_service_interval(&p, &c, town, &phases, None, policy, 6.0 * 3_600.0, 5.0)
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
        let phases = c.uniform_phases();
        let duration = 6.0 * 3_600.0;
        for town in towns() {
            let greedy = handover_timeline(
                &p,
                &c,
                town,
                &phases,
                None,
                HandoverPolicy::greedy(MIN_ELEVATION),
                duration,
                5.0,
            );
            let sticky = handover_timeline(
                &p,
                &c,
                town,
                &phases,
                None,
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
        let phases = thin.uniform_phases();
        let duration = 6.0 * 3_600.0;
        let mut greedy_total = 0;
        let mut sticky_total = 0;
        for town in towns() {
            greedy_total += handover_count(
                &p,
                &thin,
                town,
                &phases,
                None,
                HandoverPolicy::greedy(MIN_ELEVATION),
                duration,
                5.0,
            );
            sticky_total += handover_count(
                &p,
                &thin,
                town,
                &phases,
                None,
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
        let phases = c.uniform_phases();
        let town = band_point(1.9, 0.1);
        let policy = HandoverPolicy::sticky(MIN_ELEVATION, HYSTERESIS);
        let events = handover_timeline(&p, &c, town, &phases, None, policy, 3.0 * 3_600.0, 10.0);
        assert!(events.len() > 2);
        for e in &events {
            let elev = sat_elevation(&p, &c, e.to, town, &phases, e.time);
            assert!(
                elev >= policy.min_elevation - 1e-9,
                "chose a satellite at {elev} rad, below the {} rad floor",
                policy.min_elevation
            );
        }
    }
}
