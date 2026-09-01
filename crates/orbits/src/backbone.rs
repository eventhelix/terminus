// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

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
/// it is permanently behind the planet. The plane still closes — a cycle of
/// four, reached the long way round — it simply has no chords. Reach of one is
/// connectivity without shortcuts, which costs hops rather than reachability;
/// see [`intra_plane_diameter`].
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
/// relative to each other however equal their periods. Use
/// [`max_intra_shell_range_rate`] for that case, not this one.
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

/// Default hysteresis margin (m of path length) a rival anchor must beat the
/// held one by before a session is moved.
///
/// **This is an operating parameter, not a constant of the sky.** It is the
/// one number in the anchor policy that geometry does not fix, and it should
/// be tunable in flight: a fleet that later finds itself short of backbone,
/// or short of anchor compute, will want to move it. The default is stated
/// here so that everything downstream has something to quote, not because
/// 5,000 km is a law.
///
/// The default buys thinking time. Nothing forces a session to move — a ring
/// reaches every anchor at every instant, so the only reason to re-anchor is
/// to shorten the path — and `feeder_terminals` measures what holding costs.
/// Both ends of that curve are unaffordable. Holding an anchor for ever takes
/// a 25,000 km margin, whose p95 round trip of 290 ms spends all but 10 ms of
/// a 300 ms first-token budget before the model has read the question.
/// Chasing the shortest path costs more backbone than a 100 Gbps link can
/// carry. At this setting a session changes anchor 12.70 times a day, its p95
/// path is 22,925 km — a 178 ms round trip once the relays are paid, leaving
/// 122 ms of thinking time — and the busiest feeder link carries 17.5 Gbps of
/// working memory at the million-terminal ceiling.
///
/// What this setting costs is a feature. Sessions migrate in steady state, so
/// make-before-break transfer of working memory has to exist at first release
/// rather than being deferred; ADR-0021 was reversed for exactly this reason.
/// What it still does not buy is load balancing — nothing in this crate models
/// anchor compute — which is why this number should be expected to keep moving
/// in service, and why it must be a parameter rather than a constant.
pub const REANCHOR_MARGIN: f64 = 5_000e3;

/// Which anchor a session should hold, given what each one costs to reach.
///
/// `path_cost(i)` is the total path length (m) to anchor `i` — necklace hops
/// plus feeder link, as [`crate::routing::exit_gateway`] computes it — or
/// `None` if the ring cannot reach that anchor at all.
///
/// Two things are absent that an earlier version of this function had, and
/// both went for the same reason. It used to filter candidates by what the
/// *serving satellite* could see, and to break ties on how long an anchor
/// would stay visible from it. Neither survives the necklace: a session can
/// leave its ring through any ring mate, so the horizon that matters belongs
/// to the ring, and a ring can see every anchor at every instant. Reachability
/// stopped being the binding constraint, and dwell was only ever a way of
/// predicting when reachability would expire.
///
/// What binds instead is latency, so the rule is simply the shortest path —
/// with `margin` of hysteresis, because a rival anchor has to be worth a
/// migration, not merely worth a millisecond.
pub fn select_anchor(
    anchors: usize,
    path_cost: impl Fn(usize) -> Option<f64>,
    current: Option<usize>,
    margin: f64,
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for i in 0..anchors {
        let Some(cost) = path_cost(i) else {
            continue;
        };
        if best.map_or(true, |(_, b)| cost < b) {
            best = Some((i, cost));
        }
    }
    let (best_i, best_cost) = best?;

    // Hold what we have unless the challenger clears the margin.
    if let Some(held) = current {
        if let Some(held_cost) = path_cost(held) {
            if held_cost <= best_cost + margin {
                return Some(held);
            }
        }
    }
    Some(best_i)
}

/// Most hops needed to reach any satellite in one's own ring, or `None` if the
/// ring does not connect at all.
///
/// Counted over the **link** graph, not the visibility graph, and the two are
/// not the same. [`intra_plane_reach`] says how many ring mates a satellite can
/// *see*; [`crate::routing::NECKLACE_LINKS`] says how many it has a terminal
/// aimed at, which is one on each side. A satellite can see past its neighbour
/// and cannot talk past it, so a hop moves one place and a ring of twelve takes
/// **six** hops to cross — not the three its sightlines would allow.
///
/// The gap between the two is margin rather than capability, and it is checked
/// here: a link the planet hides is not a link.
///
/// Hops are not free. Each is 14.8 ms of light time around the wheel and 124 ms
/// around a MEO plane, and a session pays it twice.
pub fn intra_plane_diameter(
    body: &CentralBody,
    altitude: f64,
    sats_per_plane: usize,
) -> Option<usize> {
    let visible = intra_plane_reach(body, altitude, sats_per_plane);
    let links = crate::routing::NECKLACE_LINKS.min(visible);
    if links == 0 {
        return None;
    }
    Some((sats_per_plane / 2).div_ceil(links))
}

/// Worst-case range rate (m/s) between two satellites in *different planes of
/// one shell*, swept numerically over an orbital period.
///
/// [`max_shell_range_rate`] cannot answer this. Its closed form turns on the
/// difference in mean motion, which is exactly zero within a shell — true for
/// two satellites sharing a plane, which really are frozen relative to each
/// other, and badly false across planes. Two inclined planes cross at an
/// angle, and satellites in them sweep past each other at a good fraction of
/// orbital velocity however identical their periods.
///
/// It decides whether anchor-to-anchor links can be pointed once and left,
/// the way the wheel's necklace can, or have to steer and precompensate like
/// the feeder links. The answer is the second one.
///
/// Pairs the planet is blocking are skipped: Doppler on a link that cannot
/// close is not a design constraint.
pub fn max_intra_shell_range_rate(
    body: &CentralBody,
    shell: &crate::walker::WalkerShell,
    step: f64,
) -> f64 {
    let limb = max_shell_separation(body, shell.altitude, shell.altitude);
    let period = orbital_period(body, shell.altitude);
    let ids: Vec<(usize, usize)> = (0..shell.planes)
        .flat_map(|k| (0..shell.sats_per_plane).map(move |j| (k, j)))
        .collect();
    let range = |a: (usize, usize), b: (usize, usize), t: f64| {
        let p = crate::walker::shell_sat_position(body, shell, a.0, a.1, t);
        let q = crate::walker::shell_sat_position(body, shell, b.0, b.1, t);
        ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
    };

    let mut worst: f64 = 0.0;
    let mut t = 0.0;
    while t < period {
        for (i, &a) in ids.iter().enumerate() {
            for &b in ids.iter().skip(i + 1) {
                if a.0 == b.0 {
                    continue; // same plane: frozen, and the closed form knows it
                }
                let p = crate::walker::shell_sat_position(body, shell, a.0, a.1, t);
                let q = crate::walker::shell_sat_position(body, shell, b.0, b.1, t);
                if separation(p, q) > limb {
                    continue;
                }
                let rate = (range(a, b, t + step) - range(a, b, t)).abs() / step;
                worst = worst.max(rate);
            }
        }
        t += step;
    }
    worst
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
    /// directly opposite is permanently behind the planet. The plane still
    /// closes, as a cycle of four with no chords: anchor-to-anchor traffic can
    /// go the long way round, it just cannot cut across.
    #[test]
    fn a_four_satellite_plane_closes_the_long_way_round() {
        let p = reference_planet();
        assert_eq!(intra_plane_reach(&p, 20_000e3, 4), 1);

        let limit = max_shell_separation(&p, 20_000e3, 20_000e3).to_degrees();
        assert!(
            (limit - 152.0).abs() < 0.1,
            "line of sight limit {limit} deg"
        );
        assert!(180.0 > limit, "the antipodal plane mate is occulted");
        // Occulted diagonal, intact cycle: reachable in two hops, not one.
        assert_eq!(intra_plane_diameter(&p, 20_000e3, 4), Some(2));
        // The wheel crosses in six, not three: terminals point one place, and
        // seeing two places along the ring does not let a hop skip one.
        assert_eq!(intra_plane_diameter(&p, 2_200e3, 12), Some(6));

        // And the reachable one is a long way off: eight times the wheel's hop.
        let meo = intra_plane_range(&p, 20_000e3, 4, 1);
        let leo = intra_plane_range(&p, 2_200e3, 12, 1);
        assert!((meo / 1e3 - 37_294.0).abs() < 1.0, "{meo} m");
        assert!(meo / leo > 8.0, "ratio {}", meo / leo);
    }

    /// The trap, pinned. Within one shell the closed form reports no Doppler at
    /// all, which is right for a plane mate and wrong for everything else --
    /// cross-plane pairs sweep past each other at kilometres per second. An
    /// anchor-to-anchor link is therefore nothing like the wheel's necklace: it
    /// steers, and it precompensates.
    #[test]
    fn cross_plane_links_have_doppler_the_closed_form_reports_as_zero() {
        let p = reference_planet();
        let shell = nav_shell();

        let closed = max_shell_range_rate(&p, shell.altitude, shell.altitude);
        assert_eq!(closed, 0.0, "the closed form sees no relative motion");

        let swept = max_intra_shell_range_rate(&p, &shell, 60.0);
        assert!(
            swept > 4_000.0,
            "cross-plane range rate {swept} m/s should be kilometres per second"
        );
        assert!((swept - 4_890.0).abs() < 400.0, "{swept} m/s");

        // A plane mate really is frozen, which is why the closed form is not
        // simply wrong -- it answers a narrower question than it looks like it
        // answers.
        let a = crate::walker::shell_sat_position(&p, &shell, 0, 0, 0.0);
        let b = crate::walker::shell_sat_position(&p, &shell, 0, 1, 0.0);
        let a2 = crate::walker::shell_sat_position(&p, &shell, 0, 0, 600.0);
        let b2 = crate::walker::shell_sat_position(&p, &shell, 0, 1, 600.0);
        let d1 = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        let d2 =
            ((a2[0] - b2[0]).powi(2) + (a2[1] - b2[1]).powi(2) + (a2[2] - b2[2]).powi(2)).sqrt();
        assert!((d1 - d2).abs() < 1.0, "plane mates drifted {} m", d1 - d2);
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

    /// Selection is now simply the shortest path, because reachability stopped
    /// being the binding constraint the moment a session could leave its ring
    /// through any ring mate. The dwell tiebreak went with it: dwell only ever
    /// predicted when reachability would expire, and it no longer does.
    #[test]
    fn the_shortest_path_wins_when_nothing_is_held() {
        let costs = [30_000e3, 19_000e3, 26_000e3];
        assert_eq!(
            select_anchor(3, |i| Some(costs[i]), None, REANCHOR_MARGIN),
            Some(1)
        );
    }

    /// An anchor the ring cannot reach is not a candidate. With a whole ring
    /// to pick a door from this is rare, but the rule has to exist.
    #[test]
    fn an_unreachable_anchor_is_not_a_candidate() {
        let pick = select_anchor(
            3,
            |i| {
                if i == 1 {
                    None
                } else {
                    Some(30_000e3 - i as f64)
                }
            },
            None,
            REANCHOR_MARGIN,
        );
        assert_eq!(pick, Some(2));
        assert_eq!(select_anchor(2, |_| None, None, REANCHOR_MARGIN), None);
    }

    /// A held anchor is not given up for a millisecond. A re-anchor costs a
    /// make-before-break transfer of the whole working memory, so a rival has
    /// to clear the margin rather than merely come first.
    #[test]
    fn a_held_anchor_is_not_traded_for_a_marginal_gain() {
        // The margin is a parameter, so the behaviour is tested against a
        // stated one rather than against whatever the default happens to be.
        // Deliberately not the default value, so this test keeps its meaning
        // when the default moves.
        let margin = 8_000e3;
        let near = [26_000e3, 24_000e3];
        assert_eq!(
            select_anchor(2, |i| Some(near[i]), Some(0), margin),
            Some(0),
            "2,000 km of saving is not worth moving gigabytes"
        );
        let far = [31_000e3, 19_000e3];
        assert_eq!(select_anchor(2, |i| Some(far[i]), Some(0), margin), Some(1));
    }

    /// At the default margin a session does move, and that is the intent. The
    /// spread of path lengths the geometry produces is wider than the margin,
    /// so a rival does clear it: the worst path held at this setting is
    /// 25,205 km against a shortest-path mean of 18,569 km, a 6,636 km gap
    /// worth 44 ms of round trip and so worth a migration. Holding an anchor
    /// for ever would take a margin wider than that whole spread, and
    /// `feeder_terminals` prices what that costs in thinking time.
    #[test]
    fn the_default_margin_lets_a_worthwhile_rival_win() {
        let held_worst = 25_205e3;
        let rival_best = 18_569e3;
        assert!(
            held_worst - rival_best > REANCHOR_MARGIN,
            "the observed spread must exceed the default margin, or nothing migrates"
        );
        assert_eq!(
            select_anchor(
                2,
                |i| Some(if i == 0 { held_worst } else { rival_best }),
                Some(0),
                REANCHOR_MARGIN
            ),
            Some(1)
        );
    }

    /// If the held anchor becomes unreachable the session moves regardless of
    /// margin: there is nothing left to weigh against.
    #[test]
    fn an_unreachable_held_anchor_is_released() {
        let pick = select_anchor(
            2,
            |i| if i == 0 { None } else { Some(31_000e3) },
            Some(0),
            REANCHOR_MARGIN,
        );
        assert_eq!(pick, Some(1));
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
