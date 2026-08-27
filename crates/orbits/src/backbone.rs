//! Inter-satellite backbone geometry: intra-ring neighbor links, LEO→MEO
//! feeder-link visibility, and the worst-case Doppler the feeder links must
//! precompensate.

use crate::circular::orbital_period;
use crate::CentralBody;

/// Distance (m) between adjacent satellites evenly spaced on one circular
/// ring: the chord 2r·sin(π/n). Constant for all time — satellites sharing
/// a circular orbit do not move relative to each other.
pub fn intra_plane_neighbor_range(body: &CentralBody, altitude: f64, sats_per_plane: usize) -> f64 {
    intra_plane_range(body, altitude, sats_per_plane, 1)
}

/// Distance (m) to the satellite `hops` places away around one's own ring:
/// the chord 2r·sin(hops·π/n). Also constant, for the same reason.
pub fn intra_plane_range(
    body: &CentralBody,
    altitude: f64,
    sats_per_plane: usize,
    hops: usize,
) -> f64 {
    let r = body.radius + altitude;
    2.0 * r * (hops as f64 * std::f64::consts::PI / sats_per_plane as f64).sin()
}

/// How many places around its own ring a satellite can still see, before the
/// planet cuts the chord.
///
/// A satellite sees its own shell out to `2·acos(R/r)` of separation, and ring
/// mates sit `2π/n` apart, so the answer is a division — but it is a division
/// with consequences, and they run opposite ways at the two altitudes this
/// architecture uses.
///
/// At 2,200 km the limit is 84.0° against 30.0° spacing, so an access
/// satellite reaches two ring mates in each direction and the third is
/// occulted. That is not a routing policy: it is why traffic along the ring
/// travels "at most a hop or two" and no further.
///
/// At 20,000 km the limit is 152.0° against 90.0° spacing, so an anchor
/// reaches exactly one plane mate each way and the satellite directly opposite
/// it is permanently behind the planet. A four-satellite plane is a broken
/// necklace, not a closed one.
pub fn intra_plane_reach(body: &CentralBody, altitude: f64, sats_per_plane: usize) -> usize {
    let limit = 2.0 * (body.radius / (body.radius + altitude)).acos();
    let spacing = 2.0 * std::f64::consts::PI / sats_per_plane as f64;
    // A ring mate more than half way round is approached from the other side.
    let reach = (limit / spacing).floor() as usize;
    reach.min(sats_per_plane / 2)
}

/// Largest central angle (rad) at which satellites on two shells still see
/// each other over the planet's limb: acos(R/r₁) + acos(R/r₂).
pub fn max_shell_separation(body: &CentralBody, alt1: f64, alt2: f64) -> f64 {
    let a1 = (body.radius / (body.radius + alt1)).acos();
    let a2 = (body.radius / (body.radius + alt2)).acos();
    a1 + a2
}

/// Fraction of shell 2 visible from a satellite on shell 1:
/// (1 − cos ψ_max)/2.
pub fn shell_visible_fraction(body: &CentralBody, alt1: f64, alt2: f64) -> f64 {
    (1.0 - max_shell_separation(body, alt1, alt2).cos()) / 2.0
}

/// Worst-case range rate (m/s) between satellites on two circular shells,
/// scanning the coplanar separation angle over the mutually visible range:
/// ρ̇(Δ) = r₁r₂·(ω₁−ω₂)·sinΔ/ρ(Δ). Fully deterministic for known orbits.
///
/// **Only meaningful for shells at different altitudes.** The model turns on
/// the difference in mean motion, so equal altitudes give exactly zero — which
/// is the right answer for two satellites sharing a plane, and the wrong one
/// for two satellites in different planes of the same shell, which do move
/// relative to each other however equal their periods. Reach for a numerical
/// sweep over the actual shell for that case, not for this.
pub fn max_shell_range_rate(body: &CentralBody, alt1: f64, alt2: f64) -> f64 {
    let r1 = body.radius + alt1;
    let r2 = body.radius + alt2;
    let omega_rel = 2.0 * std::f64::consts::PI / orbital_period(body, alt1)
        - 2.0 * std::f64::consts::PI / orbital_period(body, alt2);
    let psi_max = max_shell_separation(body, alt1, alt2);
    let mut max_rate: f64 = 0.0;
    let steps = 10_000;
    for i in 1..=steps {
        let delta = psi_max * i as f64 / steps as f64;
        let rho = (r1 * r1 + r2 * r2 - 2.0 * r1 * r2 * delta.cos()).sqrt();
        let rate = (r1 * r2 * omega_rel * delta.sin() / rho).abs();
        max_rate = max_rate.max(rate);
    }
    max_rate
}

/// The feeder-link separation the routing policy prefers (rad).
///
/// Not a visibility limit -- [`max_shell_separation`] is nearly twice this --
/// but a latency one. At 2,200 km to 20,000 km, 60 deg of separation is a
/// 23,299 km hop and 77.7 ms one way, which is what keeps the session budget
/// honest. Anchors further out are reachable and are taken when nothing
/// closer is up; they just cost more.
pub const FEEDER_BUDGET: f64 = std::f64::consts::PI / 3.0;

/// Central angle (rad) between two positions in the same frame.
pub fn separation(a: [f64; 3], b: [f64; 3]) -> f64 {
    let na = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    let nb = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
    ((a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / (na * nb))
        .clamp(-1.0, 1.0)
        .acos()
}

/// How long (s) `anchor` stays within `limit` of the serving satellite, from
/// `t` and capped at `horizon`.
///
/// This is the quantity anchor selection actually wants, and the reason it is
/// not "highest" or "closest": both of those are instantaneous, and an anchor
/// on its way out is a re-anchor waiting to happen. Dwell also subsumes the
/// rising/setting distinction -- an anchor just entering the budget has hours
/// of it left, one just leaving has minutes -- without needing to test the
/// sign of anything.
pub fn anchor_dwell<S, A>(
    serving_at: S,
    anchor_at: A,
    limit: f64,
    t: f64,
    horizon: f64,
    step: f64,
) -> f64
where
    S: Fn(f64) -> [f64; 3],
    A: Fn(f64) -> [f64; 3],
{
    let mut u = t;
    while u <= t + horizon {
        if separation(serving_at(u), anchor_at(u)) > limit {
            return u - t;
        }
        u += step;
    }
    horizon
}

/// Which anchor a session on the serving satellite should hold.
///
/// The policy the reference architecture describes: every ring reaches the MEO
/// shell directly, so the candidates are the anchors above the serving
/// satellite's limb; among them the routing policy prefers those inside
/// [`FEEDER_BUDGET`]; and of those it takes the one that will stay there
/// longest, so the session is not re-anchored again in a minute.
///
/// `current` is the anchor the session already holds, and it is kept while it
/// stays REACHABLE -- above the limb -- not merely while it stays inside the
/// budget. The budget is a preference applied when choosing; it is not a
/// reason to move a running session. This is the same split
/// [`crate::handover::best_visible`] makes for access satellites: acquisition
/// answers to policy, retention answers to geometry.
///
/// It matters because the serving satellite changes every 11 minutes, and
/// tying retention to the budget would let each of those access handovers
/// knock the session off its anchor -- 20 to 79 re-anchors a day against the
/// ~19 access handovers one anchored session is supposed to ride out
/// (`compute_placement`: a 206.6 min MEO pass over an 11.0 min handover
/// interval).
///
/// Returns the index into `anchors_at`, or `None` when nothing is above the
/// limb at all.
#[allow(clippy::too_many_arguments)]
pub fn select_anchor<S, A>(
    body: &CentralBody,
    serving_alt: f64,
    shell_alt: f64,
    serving_at: S,
    anchors_at: &[A],
    current: Option<usize>,
    t: f64,
) -> Option<usize>
where
    S: Fn(f64) -> [f64; 3],
    A: Fn(f64) -> [f64; 3],
{
    let limb = max_shell_separation(body, serving_alt, shell_alt);
    let now = serving_at(t);

    // Hold what we have while it is still reachable.
    if let Some(i) = current {
        if i < anchors_at.len() && separation(now, anchors_at[i](t)) <= limb {
            return Some(i);
        }
    }

    let horizon = 6.0 * 3_600.0;
    let step = 60.0;
    let mut best: Option<(usize, f64)> = None;
    // Inside the budget first, on dwell. Only if nothing is inside the budget
    // does the wider limb-limited set get a look, and then on separation --
    // out there the honest goal is the shortest hop available, not the
    // longest-lived one.
    for (i, at) in anchors_at.iter().enumerate() {
        if separation(now, at(t)) > FEEDER_BUDGET {
            continue;
        }
        // Dwell to the limb, not to the budget: that is how long this anchor
        // could carry the session, which is what the choice is between.
        let d = anchor_dwell(&serving_at, at, limb, t, horizon, step);
        if best.map_or(true, |(_, bd)| d > bd) {
            best = Some((i, d));
        }
    }
    if let Some((i, _)) = best {
        return Some(i);
    }

    let mut fallback: Option<(usize, f64)> = None;
    for (i, at) in anchors_at.iter().enumerate() {
        let s = separation(now, at(t));
        if s <= limb && fallback.map_or(true, |(_, bs)| s < bs) {
            fallback = Some((i, s));
        }
    }
    fallback.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::polar_sat_position;

    /// The published claim that traffic travels "at most a hop or two" along
    /// its ring is not a routing choice. It is the planet: 84.0 deg of line of
    /// sight against 30.0 deg of spacing leaves room for two ring mates and no
    /// more.
    #[test]
    fn the_wheel_reaches_two_ring_mates_and_the_third_is_occulted() {
        let p = reference_planet();
        assert_eq!(intra_plane_reach(&p, 2_200e3, 12), 2);

        let limit = max_shell_separation(&p, 2_200e3, 2_200e3).to_degrees();
        assert!(
            (limit - 84.0).abs() < 0.1,
            "line of sight limit {limit} deg"
        );
        assert!(2.0 * 30.0 <= limit, "two hops must fit");
        assert!(3.0 * 30.0 > limit, "three must not");
    }

    /// The shell runs the other way. A four-satellite plane is 90 deg between
    /// neighbours, so an anchor reaches one each side and the satellite
    /// directly opposite is permanently behind the planet -- a necklace that
    /// cannot be closed, which is why anchor-to-anchor routing cannot simply
    /// go the long way round a plane.
    #[test]
    fn a_four_satellite_plane_cannot_close_its_necklace() {
        let p = reference_planet();
        assert_eq!(intra_plane_reach(&p, 20_000e3, 4), 1);

        let limit = max_shell_separation(&p, 20_000e3, 20_000e3).to_degrees();
        assert!(
            (limit - 152.0).abs() < 0.1,
            "line of sight limit {limit} deg"
        );
        assert!(180.0 > limit, "the antipodal plane mate is occulted");

        // And the reachable one is a long way off: eight times the wheel's hop.
        let meo = intra_plane_range(&p, 20_000e3, 4, 1);
        let leo = intra_plane_range(&p, 2_200e3, 12, 1);
        assert!((meo / 1e3 - 37_294.0).abs() < 1.0, "{meo} m");
        assert!(meo / leo > 8.0, "ratio {}", meo / leo);
    }

    fn nav_shell() -> crate::walker::WalkerShell {
        crate::walker::WalkerShell {
            altitude: 20_000e3,
            planes: 6,
            sats_per_plane: 4,
            inclination: 55.0_f64.to_radians(),
            phase_factor: 1.0,
        }
    }

    /// Anchor selection takes the anchor inside the budget that will stay
    /// reachable longest, not the closest one. The two disagree often: closest
    /// is instantaneous, and an anchor on its way out is a re-anchor waiting to
    /// happen. Dwell is what a session actually cares about, and it subsumes
    /// rising-versus-setting without testing the sign of anything.
    #[test]
    fn the_chosen_anchor_is_the_longest_reachable_one_in_budget() {
        let p = reference_planet();
        let shell = nav_shell();
        let serving = |u: f64| polar_sat_position(&p, 2_200e3, 0.0, 0.0, u);
        let anchors: Vec<_> = (0..shell.planes)
            .flat_map(|k| (0..shell.sats_per_plane).map(move |j| (k, j)))
            .map(|(k, j)| move |u: f64| crate::walker::shell_sat_position(&p, &shell, k, j, u))
            .collect();

        let mut disagreements = 0;
        for i in 0..60 {
            let t = i as f64 * 600.0;
            let Some(pick) = select_anchor(&p, 2_200e3, shell.altitude, serving, &anchors, None, t)
            else {
                continue;
            };
            let now = serving(t);
            // Never beyond the limb, and inside the budget whenever anything is.
            let limb = max_shell_separation(&p, 2_200e3, shell.altitude);
            assert!(separation(now, anchors[pick](t)) <= limb);
            let any_in_budget = anchors
                .iter()
                .any(|a| separation(now, a(t)) <= FEEDER_BUDGET);
            if !any_in_budget {
                continue;
            }
            let chosen = separation(now, anchors[pick](t));
            assert!(
                chosen <= FEEDER_BUDGET,
                "took a {:.0} deg hop with the budget available",
                chosen.to_degrees()
            );
            let d = anchor_dwell(serving, anchors[pick], limb, t, 6.0 * 3_600.0, 60.0);
            for a in anchors.iter() {
                if separation(now, a(t)) > FEEDER_BUDGET {
                    continue;
                }
                let other = anchor_dwell(serving, a, limb, t, 6.0 * 3_600.0, 60.0);
                assert!(other <= d, "a longer-lived anchor was passed over");
            }
            // Closest-is-not-longest-lived happens; if it never did, the test
            // would be pinning nothing.
            let closest = anchors
                .iter()
                .enumerate()
                .filter(|(_, a)| separation(now, a(t)) <= FEEDER_BUDGET)
                .min_by(|(_, x), (_, y)| {
                    separation(now, x(t))
                        .partial_cmp(&separation(now, y(t)))
                        .expect("finite")
                })
                .map(|(i, _)| i);
            if closest != Some(pick) {
                disagreements += 1;
            }
        }
        assert!(
            disagreements > 0,
            "dwell and proximity never disagreed -- the policy is untested"
        );
    }

    /// A session is not re-anchored for a marginally better hop. The held
    /// anchor is kept while it stays reachable, because a re-anchor costs a
    /// make-before-break migration -- and because the serving satellite below
    /// it changes every 11 minutes, which must not drag the anchor with it.
    #[test]
    fn a_held_anchor_is_kept_while_it_is_reachable() {
        let p = reference_planet();
        let shell = nav_shell();
        let serving = |u: f64| polar_sat_position(&p, 2_200e3, 0.0, 0.0, u);
        let anchors: Vec<_> = (0..shell.planes)
            .flat_map(|k| (0..shell.sats_per_plane).map(move |j| (k, j)))
            .map(|(k, j)| move |u: f64| crate::walker::shell_sat_position(&p, &shell, k, j, u))
            .collect();

        let mut held = None;
        let mut switches = 0;
        let mut free_switches = 0;
        let mut prev_free = None;
        for i in 0..1_440 {
            let t = i as f64 * 60.0;
            let next = select_anchor(&p, 2_200e3, shell.altitude, serving, &anchors, held, t);
            if let (Some(a), Some(b)) = (held, next) {
                if a != b {
                    switches += 1;
                }
            }
            held = next;
            // Same policy with no memory, for comparison.
            let free = select_anchor(&p, 2_200e3, shell.altitude, serving, &anchors, None, t);
            if let (Some(a), Some(b)) = (prev_free, free) {
                if a != b {
                    free_switches += 1;
                }
            }
            prev_free = free;
        }
        assert!(
            switches < free_switches,
            "holding saved nothing: {switches} switches against {free_switches} memoryless"
        );
    }

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
    fn ring_neighbors_sit_4437_km_apart() {
        let p = reference_planet();
        assert_close(intra_plane_neighbor_range(&p, 2_200e3, 12), 4.4367e6, 1e-3);
    }

    #[test]
    fn ring_neighbors_never_move_relative_to_each_other() {
        // Two satellites of the same ring, 30° apart in phase: their
        // separation is constant at every sampled time.
        let p = reference_planet();
        let sep = 2.0 * std::f64::consts::PI / 12.0;
        let dist = |t: f64| {
            let a = polar_sat_position(&p, 2_200e3, 0.3, 0.0, t);
            let b = polar_sat_position(&p, 2_200e3, 0.3, sep, t);
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
        };
        let d0 = dist(0.0);
        for t in [100.0, 3_600.0, 86_400.0, 500_000.0] {
            assert_close(dist(t), d0, 1e-9);
        }
    }

    #[test]
    fn most_of_the_meo_shell_is_visible_from_leo() {
        let p = reference_planet();
        assert_close(max_shell_separation(&p, 2_200e3, 20_000e3), 2.0601, 1e-3);
        assert_close(shell_visible_fraction(&p, 2_200e3, 20_000e3), 0.7349, 1e-3);
    }

    #[test]
    fn worst_feeder_doppler_is_about_5_5_km_per_s() {
        let p = reference_planet();
        assert_close(max_shell_range_rate(&p, 2_200e3, 20_000e3), 5.555e3, 2e-3);
    }
}
