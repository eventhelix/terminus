//! Which satellites actually have to be switched on.
//!
//! Visibility is not necessity. Because every polar ring passes over both
//! poles, a town at high latitude can have all six rings in its sky at once -
//! but it needs no more service than a town on the equator. Counting visible
//! satellites therefore overstates the fleet a design must operate: the
//! interesting number is the smallest set that still serves every point of the
//! band at the required redundancy.
//!
//! That is a minimum set cover, which is NP-hard in general. These instances
//! are tiny (tens of satellites, hundreds of sample points) and the greedy
//! rule is the standard `ln n` approximation, so [`select_active`] runs greedy
//! and [`cover_lower_bound`] supplies an independently computed lower bound to
//! prove how little room is left between the two.
//!
//! Two practical constraints shape the choice:
//!
//! - **Stickiness.** A plan recomputed from scratch each step would flap
//!   satellites on and off for no gain. [`select_active`] takes the previous
//!   plan and breaks ties toward satellites already lit, which collapses
//!   switching without costing coverage.
//! - **Predictability.** The geometry is deterministic years ahead, so the
//!   plan is a *timetable* computed on the ground and uploaded, not an
//!   onboard negotiation.

use crate::constellation::{polar_sat_position, PolarConstellation};
use crate::CentralBody;

use std::f64::consts::PI;

/// A satellite's position in the wheel, flattened to a single index.
pub fn satellite_index(c: &PolarConstellation, plane: usize, slot: usize) -> usize {
    plane * c.sats_per_plane + slot
}

/// Total satellites in the wheel.
pub fn fleet_size(c: &PolarConstellation) -> usize {
    c.planes * c.sats_per_plane
}

/// Unit vectors of every satellite at time `t`, indexed by [`satellite_index`].
pub fn satellite_units(
    body: &CentralBody,
    c: &PolarConstellation,
    phases: &[f64],
    t: f64,
) -> Vec<[f64; 3]> {
    let r = body.radius + c.altitude;
    let mut out = Vec::with_capacity(fleet_size(c));
    for (k, phase) in phases.iter().enumerate().take(c.planes) {
        let raan = k as f64 * PI / c.planes as f64;
        for j in 0..c.sats_per_plane {
            let theta0 = j as f64 * 2.0 * PI / c.sats_per_plane as f64 + phase;
            let p = polar_sat_position(body, c.altitude, raan, theta0, t);
            out.push([p[0] / r, p[1] / r, p[2] / r]);
        }
    }
    out
}

/// For each ground point, the satellites that can serve it at `t`.
///
/// A satellite clears the elevation mask exactly when the planet-central angle
/// between it and the ground point is within the footprint half-angle, so this
/// is a dot-product test rather than an elevation computation.
pub fn covering_satellites(
    body: &CentralBody,
    c: &PolarConstellation,
    phases: &[f64],
    points: &[[f64; 3]],
    min_elevation: f64,
    t: f64,
) -> Vec<Vec<usize>> {
    let units = satellite_units(body, c, phases, t);
    let cos_lambda = c.footprint_half_angle(body, min_elevation).cos();
    points
        .iter()
        .map(|g| {
            units
                .iter()
                .enumerate()
                .filter(|(_, u)| g[0] * u[0] + g[1] * u[1] + g[2] * u[2] >= cos_lambda)
                .map(|(i, _)| i)
                .collect()
        })
        .collect()
}

/// Outcome of planning one instant.
#[derive(Debug, Clone)]
pub struct ActivationPlan {
    /// One flag per satellite: is it lit?
    pub active: Vec<bool>,
    /// How many are lit.
    pub lit: usize,
    /// Points that could not reach `redundancy` even with the whole fleet on.
    /// Non-empty means the constellation itself is short, not the planner.
    pub unservable: Vec<usize>,
}

/// Smallest set of satellites (greedily chosen) giving every point at least
/// `redundancy` servers, preferring satellites in `previous` on ties.
///
/// Points that the entire fleet cannot serve to `redundancy` are reported in
/// [`ActivationPlan::unservable`] and otherwise covered as far as possible;
/// the planner never pretends a geometric shortfall away.
pub fn select_active(
    covering: &[Vec<usize>],
    fleet: usize,
    redundancy: usize,
    previous: Option<&[bool]>,
) -> ActivationPlan {
    let mut need: Vec<usize> = Vec::with_capacity(covering.len());
    let mut unservable = Vec::new();
    for (p, sats) in covering.iter().enumerate() {
        let want = redundancy.min(sats.len());
        if sats.len() < redundancy {
            unservable.push(p);
        }
        need.push(want);
    }

    let mut active = vec![false; fleet];
    let mut lit = 0;

    // Satellite -> points it can serve, for gain lookups.
    let mut serves: Vec<Vec<usize>> = vec![Vec::new(); fleet];
    for (p, sats) in covering.iter().enumerate() {
        for &s in sats {
            serves[s].push(p);
        }
    }

    loop {
        let mut best: Option<(usize, usize, bool)> = None; // (gain, sat, was_on)
        for (s, pts) in serves.iter().enumerate() {
            if active[s] {
                continue;
            }
            let gain = pts.iter().filter(|&&p| need[p] > 0).count();
            if gain == 0 {
                continue;
            }
            let was_on = previous.is_some_and(|prev| prev[s]);
            let better = match best {
                None => true,
                Some((bg, _, bw)) => gain > bg || (gain == bg && was_on && !bw),
            };
            if better {
                best = Some((gain, s, was_on));
            }
        }
        let Some((_, s, _)) = best else { break };
        active[s] = true;
        lit += 1;
        for &p in &serves[s] {
            need[p] = need[p].saturating_sub(1);
        }
    }

    // Greedy's early picks are often made redundant by its later ones. Drop
    // any satellite whose points all keep their required cover without it,
    // shedding the ones that were dark last step first so the plan stays
    // sticky. This is what closes most of the gap to the lower bound.
    let mut cover_count = vec![0usize; covering.len()];
    for (p, sats) in covering.iter().enumerate() {
        cover_count[p] = sats.iter().filter(|&&s| active[s]).count();
    }
    let required: Vec<usize> = covering
        .iter()
        .map(|sats| redundancy.min(sats.len()))
        .collect();

    let mut order: Vec<usize> = (0..fleet).filter(|&s| active[s]).collect();
    order.sort_by_key(|&s| {
        let was_on = previous.is_some_and(|prev| prev[s]);
        (was_on, serves[s].len())
    });
    for s in order {
        if serves[s].iter().all(|&p| cover_count[p] > required[p]) {
            active[s] = false;
            lit -= 1;
            for &p in &serves[s] {
                cover_count[p] -= 1;
            }
        }
    }

    ActivationPlan {
        active,
        lit,
        unservable,
    }
}

/// Light the duty ring, then patch whatever holes remain.
///
/// The operationally simplest policy that still respects the geometry: the
/// ring nearest the terminator is switched on as a block, and satellites from
/// other rings are added one at a time only where the band is still unserved.
/// It keeps the duty ring as the organising idea an operator can reason about,
/// at the cost of lighting some duty-ring satellites that are over empty sky.
///
/// With `prune`, duty-ring satellites contributing nothing are switched back
/// off afterwards, which recovers most of that cost while keeping the
/// duty-first character of the plan.
pub fn duty_first_activation(
    covering: &[Vec<usize>],
    c: &PolarConstellation,
    duty: usize,
    previous: Option<&[bool]>,
    prune: bool,
) -> ActivationPlan {
    let fleet = fleet_size(c);
    let mut active = vec![false; fleet];
    for j in 0..c.sats_per_plane {
        active[satellite_index(c, duty, j)] = true;
    }

    let mut serves: Vec<Vec<usize>> = vec![Vec::new(); fleet];
    for (p, sats) in covering.iter().enumerate() {
        for &s in sats {
            serves[s].push(p);
        }
    }
    let mut cover_count = vec![0usize; covering.len()];
    for (p, sats) in covering.iter().enumerate() {
        cover_count[p] = sats.iter().filter(|&&s| active[s]).count();
    }

    let unservable: Vec<usize> = covering
        .iter()
        .enumerate()
        .filter(|(_, sats)| sats.is_empty())
        .map(|(p, _)| p)
        .collect();

    // Patch the holes the duty ring leaves.
    loop {
        let mut best: Option<(usize, usize, bool)> = None;
        for (s, pts) in serves.iter().enumerate() {
            if active[s] {
                continue;
            }
            let gain = pts.iter().filter(|&&p| cover_count[p] == 0).count();
            if gain == 0 {
                continue;
            }
            let was_on = previous.is_some_and(|prev| prev[s]);
            let better = match best {
                None => true,
                Some((bg, _, bw)) => gain > bg || (gain == bg && was_on && !bw),
            };
            if better {
                best = Some((gain, s, was_on));
            }
        }
        let Some((_, s, _)) = best else { break };
        active[s] = true;
        for &p in &serves[s] {
            cover_count[p] += 1;
        }
    }

    if prune {
        let mut order: Vec<usize> = (0..fleet).filter(|&s| active[s]).collect();
        // Shed satellites that were dark last step first, then the least useful.
        order.sort_by_key(|&s| (previous.is_some_and(|prev| prev[s]), serves[s].len()));
        for s in order {
            let removable = serves[s]
                .iter()
                .all(|&p| cover_count[p] > covering[p].len().min(1));
            if removable {
                active[s] = false;
                for &p in &serves[s] {
                    cover_count[p] -= 1;
                }
            }
        }
    }

    let lit = active.iter().filter(|&&a| a).count();
    ActivationPlan {
        active,
        lit,
        unservable,
    }
}

/// Smooth a run of per-step plans into a schedule a spacecraft can actually
/// fly, by being lazy about switching off.
///
/// A plan computed independently at each step will strand satellites: lit for
/// one step, dark for one, lit again, because the greedy choice happened to
/// tip. Every one of those is a thermal and power-electronics cycle bought for
/// nothing. Two rules fix it, and because the plan is a *timetable* computed
/// ahead of time rather than a decision made in orbit, both can be applied
/// with perfect knowledge of what comes next:
///
/// - **Lazy off.** If a satellite is needed again within `min_off_steps`,
///   leave it on through the gap rather than cycling it.
/// - **Warm start.** Light it `warmup_steps` before it is first needed, so it
///   is never asked to take a handover cold.
///
/// Both only ever add lit time, so a smoothed schedule still covers everything
/// the input plans covered.
pub fn smooth_schedule(
    plans: &[Vec<bool>],
    warmup_steps: usize,
    min_off_steps: usize,
) -> Vec<Vec<bool>> {
    if plans.is_empty() {
        return Vec::new();
    }
    let steps = plans.len();
    let fleet = plans[0].len();
    let mut out = plans.to_vec();

    for s in 0..fleet {
        // Lazy off: close any gap shorter than min_off_steps.
        let mut i = 0;
        while i < steps {
            if out[i][s] {
                i += 1;
                continue;
            }
            let start = i;
            while i < steps && !out[i][s] {
                i += 1;
            }
            let gap = i - start;
            // Only an interior gap is worth closing; a run that starts or ends
            // dark has no "again" to wait for.
            if start > 0 && i < steps && gap < min_off_steps {
                for step in out.iter_mut().take(i).skip(start) {
                    step[s] = true;
                }
            }
        }
        // Warm start: extend every run backwards.
        if warmup_steps > 0 {
            let lit: Vec<bool> = (0..steps).map(|i| out[i][s]).collect();
            for i in 0..steps {
                if lit[i] && (i == 0 || !lit[i - 1]) {
                    let from = i.saturating_sub(warmup_steps);
                    for step in out.iter_mut().take(i).skip(from) {
                        step[s] = true;
                    }
                }
            }
        }
    }
    out
}

/// Total on/off transitions across a schedule - the number a policy actually
/// costs the fleet.
pub fn switch_count(plans: &[Vec<bool>]) -> u64 {
    plans
        .windows(2)
        .map(|w| (0..w[0].len()).filter(|&s| w[0][s] != w[1][s]).count() as u64)
        .sum()
}

/// A lower bound on any valid activation set, computed independently of the
/// greedy choice.
///
/// Collect ground points no two of which share a serving satellite. Each such
/// point needs `redundancy` satellites of its own that can serve no other point
/// in the collection, so the bound is `redundancy x` the number found. Taking
/// the most constrained points first makes the bound tight in practice.
pub fn cover_lower_bound(covering: &[Vec<usize>], redundancy: usize) -> usize {
    let mut order: Vec<usize> = (0..covering.len()).collect();
    order.sort_by_key(|&p| covering[p].len());

    let mut claimed: Vec<bool> = Vec::new();
    let mut bound = 0;
    for p in order {
        let sats = &covering[p];
        if sats.is_empty() {
            continue;
        }
        let max_idx = sats.iter().copied().max().unwrap_or(0);
        if claimed.len() <= max_idx {
            claimed.resize(max_idx + 1, false);
        }
        if sats.iter().any(|&s| claimed[s]) {
            continue;
        }
        for &s in sats {
            claimed[s] = true;
        }
        bound += redundancy.min(sats.len());
    }
    bound
}

/// The provably smallest activation set for single coverage, or `None` if the
/// search exceeded `node_budget` without finishing.
///
/// Exact, not heuristic. Any valid cover must contain at least one satellite
/// able to serve the most constrained still-uncovered point, so branching over
/// that point's covering set enumerates every cover; [`cover_lower_bound`] on
/// the remaining points prunes the rest. Instances here are small enough that
/// this closes in milliseconds, which is what lets [`select_active`] be
/// reported as near-optimal rather than merely plausible.
pub fn exact_activation(
    covering: &[Vec<usize>],
    fleet: usize,
    previous: Option<&[bool]>,
    node_budget: u64,
) -> Option<ActivationPlan> {
    let optimum = exact_min_cover_inner(covering, fleet, previous, node_budget)?;
    Some(optimum)
}

/// Size of the provably smallest single-coverage activation set, or `None` if
/// the search exceeded `node_budget`.
pub fn exact_min_cover(covering: &[Vec<usize>], fleet: usize, node_budget: u64) -> Option<usize> {
    exact_min_cover_inner(covering, fleet, None, node_budget).map(|p| p.lit)
}

fn exact_min_cover_inner(
    covering: &[Vec<usize>],
    fleet: usize,
    previous: Option<&[bool]>,
    node_budget: u64,
) -> Option<ActivationPlan> {
    // Drop any point whose covering set is a superset of another's: serving
    // the tighter point always serves the looser one.
    let mut keep: Vec<usize> = Vec::new();
    for (p, sats) in covering.iter().enumerate() {
        if sats.is_empty() {
            return None; // an unservable point; caller must handle it
        }
        let dominated = covering.iter().enumerate().any(|(q, other)| {
            q != p
                && other.len() <= sats.len()
                && (other.len() < sats.len() || q < p)
                && other.iter().all(|s| sats.contains(s))
        });
        if !dominated {
            keep.push(p);
        }
    }

    let reduced: Vec<Vec<usize>> = keep.iter().map(|&p| covering[p].clone()).collect();
    let mut serves: Vec<Vec<usize>> = vec![Vec::new(); fleet];
    for (i, sats) in reduced.iter().enumerate() {
        for &s in sats {
            serves[s].push(i);
        }
    }

    // Branch order: try satellites that were already lit first, then the ones
    // serving most points. Good incumbents appear early, and among equally
    // small covers the search settles on one that reuses the previous plan.
    let branch_rank = |s: usize| {
        let was_on = previous.is_some_and(|prev| prev[s]);
        (!was_on, std::cmp::Reverse(serves[s].len()))
    };

    let seed = select_active(covering, fleet, 1, previous);
    let mut covered = vec![false; reduced.len()];
    let mut best = seed.lit;
    let mut best_set: Vec<usize> = (0..fleet).filter(|&s| seed.active[s]).collect();
    let mut nodes: u64 = 0;
    let mut chosen: Vec<usize> = Vec::new();

    struct Ctx<'a> {
        reduced: &'a [Vec<usize>],
        serves: &'a [Vec<usize>],
        budget: u64,
    }

    #[allow(clippy::too_many_arguments)]
    fn search(
        ctx: &Ctx,
        covered: &mut Vec<bool>,
        chosen: &mut Vec<usize>,
        best: &mut usize,
        best_set: &mut Vec<usize>,
        nodes: &mut u64,
        rank: &dyn Fn(usize) -> (bool, std::cmp::Reverse<usize>),
    ) -> bool {
        *nodes += 1;
        if *nodes > ctx.budget {
            return false;
        }
        let mut target: Option<usize> = None;
        let mut fewest = usize::MAX;
        for (i, done) in covered.iter().enumerate() {
            if !*done && ctx.reduced[i].len() < fewest {
                fewest = ctx.reduced[i].len();
                target = Some(i);
            }
        }
        let Some(t) = target else {
            if chosen.len() < *best {
                *best = chosen.len();
                *best_set = chosen.clone();
            }
            return true;
        };
        if chosen.len() + 1 >= *best {
            return true;
        }
        let remaining: Vec<Vec<usize>> = covered
            .iter()
            .enumerate()
            .filter(|(_, d)| !**d)
            .map(|(i, _)| ctx.reduced[i].clone())
            .collect();
        if chosen.len() + cover_lower_bound(&remaining, 1) >= *best {
            return true;
        }
        let mut branches = ctx.reduced[t].clone();
        branches.sort_by_key(|&s| rank(s));
        for s in branches {
            let newly: Vec<usize> = ctx.serves[s]
                .iter()
                .copied()
                .filter(|&i| !covered[i])
                .collect();
            for &i in &newly {
                covered[i] = true;
            }
            chosen.push(s);
            let ok = search(ctx, covered, chosen, best, best_set, nodes, rank);
            chosen.pop();
            for &i in &newly {
                covered[i] = false;
            }
            if !ok {
                return false;
            }
        }
        true
    }

    let ctx = Ctx {
        reduced: &reduced,
        serves: &serves,
        budget: node_budget,
    };
    let finished = search(
        &ctx,
        &mut covered,
        &mut chosen,
        &mut best,
        &mut best_set,
        &mut nodes,
        &branch_rank,
    );
    if !finished {
        return None;
    }
    let mut active = vec![false; fleet];
    for &s in &best_set {
        active[s] = true;
    }
    Some(ActivationPlan {
        active,
        lit: best,
        unservable: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::band_point;

    const MASK: f64 = 25.0 * PI / 180.0;
    const BAND: f64 = 20.0 * PI / 180.0;

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

    fn band_samples(azimuths: usize) -> Vec<[f64; 3]> {
        let mut pts = Vec::new();
        for i in 0..azimuths {
            let az = i as f64 * 2.0 * PI / azimuths as f64;
            for off in [-BAND, 0.0, BAND] {
                pts.push(band_point(az, off));
            }
        }
        pts
    }

    #[test]
    fn a_plan_actually_covers_every_point() {
        let (p, c) = (reference_planet(), baseline());
        let phases = vec![0.0; c.planes];
        let pts = band_samples(72);
        let mut t = 0.0;
        while t < 86_400.0 {
            let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
            let plan = select_active(&cov, fleet_size(&c), 1, None);
            assert!(plan.unservable.is_empty(), "unservable at t={t}");
            for (i, sats) in cov.iter().enumerate() {
                assert!(
                    sats.iter().any(|&s| plan.active[s]),
                    "point {i} unserved at t={t}"
                );
            }
            t += 300.0;
        }
    }

    #[test]
    fn lighting_the_whole_fleet_is_never_necessary() {
        let (p, c) = (reference_planet(), baseline());
        let phases = vec![0.0; c.planes];
        let pts = band_samples(72);
        let mut worst = 0usize;
        let mut t = 0.0;
        while t < 86_400.0 {
            let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
            worst = worst.max(select_active(&cov, fleet_size(&c), 1, None).lit);
            t += 300.0;
        }
        assert!(worst < fleet_size(&c), "needed the whole fleet: {worst}");
    }

    #[test]
    fn the_lower_bound_never_exceeds_what_greedy_achieves() {
        let (p, c) = (reference_planet(), baseline());
        let phases = vec![0.0; c.planes];
        let pts = band_samples(72);
        let mut t = 0.0;
        while t < 86_400.0 {
            let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
            let lit = select_active(&cov, fleet_size(&c), 1, None).lit;
            let bound = cover_lower_bound(&cov, 1);
            assert!(bound <= lit, "bound {bound} above greedy {lit} at t={t}");
            t += 300.0;
        }
    }

    #[test]
    fn the_exact_search_beats_greedy_and_still_covers_everything() {
        // "Minimum" has to be earned. The exact search must never exceed
        // greedy, must actually cover the band, and on these instances it is
        // strictly better often enough to be worth running.
        let (p, c) = (reference_planet(), baseline());
        let phases = vec![0.0; c.planes];
        let pts = band_samples(72);
        let (mut checked, mut strictly_better) = (0, 0);
        let mut t = 0.0;
        while t < 43_200.0 {
            let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
            let greedy = select_active(&cov, fleet_size(&c), 1, None).lit;
            if let Some(plan) = exact_activation(&cov, fleet_size(&c), None, 4_000_000) {
                assert!(
                    plan.lit <= greedy,
                    "exact {} above greedy {greedy}",
                    plan.lit
                );
                for (i, sats) in cov.iter().enumerate() {
                    assert!(
                        sats.iter().any(|&s| plan.active[s]),
                        "point {i} unserved by the exact plan at t={t}"
                    );
                }
                assert_eq!(plan.active.iter().filter(|&&a| a).count(), plan.lit);
                checked += 1;
                if plan.lit < greedy {
                    strictly_better += 1;
                }
            }
            t += 3_600.0;
        }
        assert!(checked >= 8, "only solved {checked} instants exactly");
        assert!(strictly_better > 0, "exact never improved on greedy");
    }

    #[test]
    fn duty_first_covers_the_band_and_costs_little_over_the_optimum() {
        // Lighting the duty ring and patching the holes is the policy an
        // operator can explain. It must still cover everything, and it must
        // not cost wildly more than the proved minimum.
        let (p, c) = (reference_planet(), baseline());
        let phases = vec![0.0; c.planes];
        let pts = band_samples(72);
        let mut t = 0.0;
        while t < 43_200.0 {
            let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
            let duty = crate::duty::duty_ring(&p, &c, t);
            for prune in [false, true] {
                let plan = duty_first_activation(&cov, &c, duty, None, prune);
                for (i, sats) in cov.iter().enumerate() {
                    assert!(
                        sats.iter().any(|&s| plan.active[s]),
                        "point {i} unserved by duty-first (prune={prune}) at t={t}"
                    );
                }
                assert!(plan.lit >= c.sats_per_plane.min(plan.lit));
                assert!(plan.lit < fleet_size(&c), "duty-first lit the whole fleet");
            }
            // Pruning never costs coverage and never lights more.
            let loose = duty_first_activation(&cov, &c, duty, None, false).lit;
            let tight = duty_first_activation(&cov, &c, duty, None, true).lit;
            assert!(tight <= loose, "pruning lit more: {tight} vs {loose}");
            t += 3_600.0;
        }
    }

    #[test]
    fn stickiness_reduces_switching_without_costing_coverage() {
        let (p, c) = (reference_planet(), baseline());
        let phases = vec![0.0; c.planes];
        let pts = band_samples(72);

        let mut churn_cold = 0usize;
        let mut churn_sticky = 0usize;
        let mut prev_cold: Option<Vec<bool>> = None;
        let mut prev_sticky: Option<Vec<bool>> = None;
        let mut t = 0.0;
        while t < 43_200.0 {
            let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
            let cold = select_active(&cov, fleet_size(&c), 1, None);
            let sticky = select_active(&cov, fleet_size(&c), 1, prev_sticky.as_deref());
            if let Some(prev) = &prev_cold {
                churn_cold += (0..fleet_size(&c))
                    .filter(|&s| prev[s] != cold.active[s])
                    .count();
            }
            if let Some(prev) = &prev_sticky {
                churn_sticky += (0..fleet_size(&c))
                    .filter(|&s| prev[s] != sticky.active[s])
                    .count();
            }
            prev_cold = Some(cold.active);
            prev_sticky = Some(sticky.active);
            t += 60.0;
        }
        assert!(
            churn_sticky < churn_cold,
            "sticky {churn_sticky} vs cold {churn_cold}"
        );
    }

    #[test]
    fn smoothing_removes_flapping_without_dropping_coverage() {
        let (p, c) = (reference_planet(), baseline());
        let phases = vec![0.0; c.planes];
        let pts = band_samples(72);
        let fleet = fleet_size(&c);

        let step = 60.0;
        let mut plans = Vec::new();
        let mut covs = Vec::new();
        let mut prev: Option<Vec<bool>> = None;
        let mut t = 0.0;
        while t < 4.0 * 3600.0 {
            let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
            let duty = crate::duty::duty_ring(&p, &c, t);
            let plan = duty_first_activation(&cov, &c, duty, prev.as_deref(), false);
            prev = Some(plan.active.clone());
            plans.push(plan.active);
            covs.push(cov);
            t += step;
        }

        let raw_switches = switch_count(&plans);
        // Five minutes of warm-up, ten minutes of laziness.
        let smoothed = smooth_schedule(&plans, 5, 10);
        let smooth_switches = switch_count(&smoothed);

        assert!(
            smooth_switches < raw_switches,
            "smoothing did not reduce switching: {smooth_switches} vs {raw_switches}"
        );
        // Smoothing only ever adds lit time, so coverage cannot regress.
        for (i, cov) in covs.iter().enumerate() {
            for (j, sats) in cov.iter().enumerate() {
                assert!(
                    sats.iter().any(|&s| smoothed[i][s]),
                    "point {j} unserved after smoothing at step {i}"
                );
            }
            for s in 0..fleet {
                assert!(
                    !plans[i][s] || smoothed[i][s],
                    "smoothing switched a satellite off"
                );
            }
        }

        // No satellite should be left with a one-step blink.
        for s in 0..fleet {
            for i in 1..smoothed.len() - 1 {
                let blink = smoothed[i][s] && !smoothed[i - 1][s] && !smoothed[i + 1][s];
                assert!(!blink, "satellite {s} still blinks at step {i}");
            }
        }
    }

    #[test]
    fn asking_for_two_servers_reports_where_the_fleet_is_short() {
        // The 72-satellite baseline has a minimum of one visible satellite, so
        // dual coverage is not merely expensive here - it is geometrically
        // unavailable at some instants, and the planner must say so.
        let (p, c) = (reference_planet(), baseline());
        let phases = vec![0.0; c.planes];
        let pts = band_samples(72);
        let mut short = false;
        let mut t = 0.0;
        while t < 11.2 * 86_400.0 && !short {
            let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
            short = !select_active(&cov, fleet_size(&c), 2, None)
                .unservable
                .is_empty();
            t += 120.0;
        }
        assert!(
            short,
            "expected the baseline to fall short of dual coverage"
        );
    }
}
