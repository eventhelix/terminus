//! What does the backbone cost in optical terminals?
//!
//! Each end of a laser link needs its own terminal, so the topology question
//! is really a hardware question:
//!
//!   A. how far a satellite can see along its own ring, which decides whether
//!      a ring can relay at all, and turns out to answer the question at both
//!      altitudes without any traffic model
//!   B. the direct topology — no ring-to-ring links, no anchor-to-anchor
//!      links, every session reaching its own anchor
//!   C. the gateway topology — each access satellite feeds its nearest anchor
//!      and the shell carries traffic onward
//!   D. the necklace, sized for a fleet built to one drawing
//!   E. what links the shell would need of its own
//!   F. whether it needs any at all, given the wheel can relay anchor to anchor
//!   G. what one lost telescope costs, and what buys the loss back
//!   H. how often a session actually moves, now that nothing forces it to
//!
//! The direct topology is the simplest thing to describe and the most
//! expensive thing to build, because sessions hold their anchors for hours and
//! so end up scattered across the shell.
//!
//! Run: cargo run --release -p terminus-orbits --example feeder_terminals

use std::collections::BTreeMap;

use terminus_orbits::activation::{covering_satellites, duty_first_activation, satellite_index};
use terminus_orbits::backbone::{
    intra_plane_diameter, intra_plane_range, intra_plane_reach, max_intra_shell_range_rate,
    max_shell_range_rate, max_shell_separation, separation,
};
use terminus_orbits::circular::orbital_period;
use terminus_orbits::constellation::{band_point, plane_phases, PhaseMode, PolarConstellation};
use terminus_orbits::coverage::edge_slant_range;
use terminus_orbits::duty::duty_ring;
use terminus_orbits::handover::{best_visible, HandoverPolicy};
use terminus_orbits::placement::{one_way_latency, one_way_light_time};
use terminus_orbits::routing::{exit_gateway, feeder_route, RELAY_DELAY};
use terminus_orbits::topology::{
    direct_demand, gateway_demand, pair_load, uniform_relay_demand, Session, TerminalDemand,
};
use terminus_orbits::walker::{shell_sat_position, WalkerShell};
use terminus_orbits::CentralBody;

const ACCESS_ALT: f64 = 2_200e3;
const MEO_ALT: f64 = 20_000e3;
const MASK: f64 = 25.0 * std::f64::consts::PI / 180.0;
const HYSTERESIS: f64 = 3.0 * std::f64::consts::PI / 180.0;
const BAND: f64 = 20.0 * std::f64::consts::PI / 180.0;
const TOWNS: usize = 1_000;
const STEP: f64 = 300.0;
const SPAN: f64 = 86_400.0;
/// How far a session may travel along the necklace to borrow a feeder
/// terminal. The lasers stay powered even where the radios are dark, so the
/// necklace is a standing relay and hops chain: half a ring reaches all of it.
const REACH: usize = 6;
/// What a hop costs when a session leaves through a ring mate instead of its
/// own satellite. Terminals point one place each way, so a hop moves one
/// place -- see `routing::NECKLACE_LINKS`.
const HOP_RANGE: f64 = 4_437e3;

/// Re-anchor margins to sweep (m of path). The policy has to choose one, and
/// choosing it from a curve beats choosing it from taste.
const MARGINS: [f64; 7] = [
    0.0, 2_500e3, 5_000e3, 10_000e3, 20_000e3, 25_000e3, 30_000e3,
];
/// The RFP's first-token budget (ms, TER-REQ-003), amended to cover
/// failure-free operation only. Whatever the round trip does not spend, the
/// model gets to think in -- which is what makes the margin a latency choice
/// and not only a bandwidth one.
const FIRST_TOKEN_BUDGET_MS: f64 = 300.0;
/// TER-REQ-003's degraded budget (ms): applies while a telescope is dark and
/// traffic rides the plane link in its place. Section G scores the detour
/// against both.
const DEGRADED_BUDGET_MS: f64 = 600.0;
/// Instants sampled across the day for section G's detour walk.
const INSTANTS: usize = 24;
/// Budgets to show the margin against. The first is TER-REQ-003 as written
/// and the one the policy is chosen under; the others are there because a
/// requirement that sizes an entire backbone should be visibly a choice.
const BUDGETS_MS: [f64; 3] = [300.0, 500.0, 600.0];

/// Index into `MARGINS` of the policy the rest of this example reports on.
/// It is the same number the library states as `REANCHOR_MARGIN`.
const CHOSEN_MARGIN: usize = 2;

struct Town {
    unit: [f64; 3],
    access: Option<(usize, usize)>,
    /// Held anchor under each margin in `MARGINS`, tracked in parallel so one
    /// pass of the simulation produces the whole curve.
    anchor: Vec<Option<usize>>,
}

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let wheel = PolarConstellation {
        altitude: ACCESS_ALT,
        planes: 6,
        sats_per_plane: 12,
        interplane_phase: 0.0,
    };
    let phases = plane_phases(
        PhaseMode::Random,
        wheel.planes,
        wheel.sats_per_plane,
        0x51E7_2026,
    );
    let shell = WalkerShell {
        altitude: MEO_ALT,
        planes: 6,
        sats_per_plane: 4,
        inclination: 55.0_f64.to_radians(),
        phase_factor: 1.0,
    };
    let anchor_ids: Vec<(usize, usize)> = (0..shell.planes)
        .flat_map(|k| (0..shell.sats_per_plane).map(move |j| (k, j)))
        .collect();
    let anchors_at: Vec<_> = anchor_ids
        .iter()
        .map(|&(k, j)| move |u: f64| shell_sat_position(&planet, &shell, k, j, u))
        .collect();

    println!("A. How far can a satellite see along its own ring?\n");
    println!(
        "   A ring can only relay as far as the planet lets it. Both answers\n\
         \x20  fall out of one division, and they run opposite ways.\n"
    );
    println!(
        "{:>14} {:>10} {:>10} {:>9} {:>8} {:>12}",
        "shell", "alt (km)", "per ring", "spacing", "LOS max", "reach"
    );
    for (label, alt, n) in [
        ("access wheel", ACCESS_ALT, 12usize),
        ("MEO shell", MEO_ALT, 4usize),
    ] {
        let limit = max_shell_separation(&planet, alt, alt).to_degrees();
        let spacing = 360.0 / n as f64;
        let reach = intra_plane_reach(&planet, alt, n);
        println!(
            "{:>14} {:>10.0} {:>10} {:>8.1}° {:>7.1}° {:>9} hop(s)",
            label,
            alt / 1e3,
            n,
            spacing,
            limit,
            reach
        );
    }
    println!(
        "\n   Reach is what a satellite can SEE along its ring, and it is not what\n\
         \x20  it can talk to. Each access satellite carries two necklace terminals,\n\
         \x20  aimed one place each way and held there for years, so a hop moves one\n\
         \x20  place and a ring of twelve crosses in six. The spare sightline is\n\
         \x20  margin, not a shortcut: a satellite can see past its neighbour and\n\
         \x20  cannot talk past it. There are no links between rings at all. In the\n\
         \x20  shell a four-satellite plane reaches one neighbour each way and the\n\
         \x20  satellite opposite is permanently occulted; the plane still closes,\n\
         \x20  as a cycle of four with no chords."
    );
    for hops in 1..=2 {
        println!(
            "     ring mate {} hop(s) away: {:>7.0} km at 2,200 km   {:>7.0} km at 20,000 km",
            hops,
            intra_plane_range(&planet, ACCESS_ALT, 12, hops) / 1e3,
            intra_plane_range(&planet, MEO_ALT, 4, hops) / 1e3,
        );
    }

    println!(
        "
   And whether those links can be pointed once and left alone:
"
    );
    println!(
        "     wheel, ring mate         : {:>6.2} km/s -- frozen, point once and hold",
        max_shell_range_rate(&planet, ACCESS_ALT, ACCESS_ALT) / 1e3
    );
    println!(
        "     shell, plane mate        : {:>6.2} km/s -- frozen for the same reason",
        max_shell_range_rate(&planet, MEO_ALT, MEO_ALT) / 1e3
    );
    println!(
        "     shell, ACROSS planes     : {:>6.2} km/s -- steers, and precompensates",
        max_intra_shell_range_rate(&planet, &shell, 60.0) / 1e3
    );
    println!(
        "     wheel to shell (feeder)  : {:>6.2} km/s -- steers, and precompensates",
        max_shell_range_rate(&planet, ACCESS_ALT, MEO_ALT) / 1e3
    );
    println!(
        "\n   The closed form reports 0.00 within a shell because it turns on the\n\
         \x20  difference in mean motion. That is right for a plane mate and wrong\n\
         \x20  across planes, where two inclined orbits cross at an angle and their\n\
         \x20  satellites sweep past each other however equal their periods. An\n\
         \x20  anchor-to-anchor link therefore splits in two. Plane mates are frozen\n\
         \x20  and close their own plane into a cycle; joining plane to plane is the\n\
         \x20  half that steers, and no arrangement of frozen links can do it."
    );

    // ---- the session population ------------------------------------------
    let mut towns: Vec<Town> = (0..TOWNS)
        .map(|i| {
            let az =
                (i as f64 * 7.3 * std::f64::consts::TAU / TOWNS as f64) % std::f64::consts::TAU;
            let off = ((i % 5) as f64 - 2.0) / 2.0 * BAND;
            Town {
                unit: band_point(az, off),
                access: None,
                anchor: vec![None; MARGINS.len()],
            }
        })
        .collect();

    let band_points: Vec<[f64; 3]> = (0..72)
        .flat_map(|i| {
            let az = i as f64 * std::f64::consts::TAU / 72.0;
            [-BAND, 0.0, BAND].map(|off| band_point(az, off))
        })
        .collect();

    let policy = HandoverPolicy::sticky(MASK, HYSTERESIS);
    let limb = max_shell_separation(&planet, ACCESS_ALT, MEO_ALT);
    let mut previous: Option<Vec<bool>> = None;
    // Every instant is a sample. Sizing wants the day's distribution, not one
    // snapshot: a single instant has only as many samples as there are lit
    // satellites, so its p99 and its maximum are the same number.
    let mut direct_instants: Vec<TerminalDemand> = Vec::new();
    let mut gateway_instants: Vec<TerminalDemand> = Vec::new();
    // Two readings of the necklace, and the gap between them is the argument.
    let mut relay_lit: Vec<TerminalDemand> = Vec::new();
    let mut session_snapshots: Vec<Vec<Session>> = Vec::new();
    // Migrations are now a policy outcome rather than a geometric
    // certainty, so they have to be counted rather than assumed.
    let mut anchor_changes = [0usize; MARGINS.len()];
    let mut path_sum = [0.0f64; MARGINS.len()];
    let mut path_worst = [0.0f64; MARGINS.len()];
    // TER-REQ-003 is a p95 requirement, so the distribution is what to judge
    // against; the maximum alone would be the wrong test.
    let mut path_samples: Vec<Vec<f64>> = vec![Vec::new(); MARGINS.len()];
    // What the town actually waits, which is the path plus the radio leg plus
    // a relay at every satellite that forwards. Sampled rather than derived
    // from the p95 path, because the hop count varies independently of it.
    let mut rtt_samples: Vec<Vec<f64>> = vec![Vec::new(); MARGINS.len()];
    let mut hops_worst = [0usize; MARGINS.len()];
    let mut path_n = [0usize; MARGINS.len()];
    // The radio leg is the same for every route, so it is measured once, at
    // the edge of the footprint where it is longest.
    let user_leg = one_way_light_time(edge_slant_range(&planet, ACCESS_ALT, MASK));
    let mut relay_all: Vec<TerminalDemand> = Vec::new();
    let mut lit_counts: Vec<(usize, usize)> = Vec::new();

    let mut t = 0.0;
    while t < SPAN {
        let cov = covering_satellites(&planet, &wheel, &phases, &band_points, MASK, t);
        let mut plan = duty_first_activation(
            &cov,
            &wheel,
            duty_ring(&planet, &wheel, t),
            previous.as_deref(),
            true,
        )
        .active;

        for town in towns.iter_mut() {
            // Retention: hold the access satellite while geometry allows.
            let held = town.access.filter(|&(k, j)| {
                let raan = k as f64 * std::f64::consts::PI / wheel.planes as f64;
                let theta0 =
                    j as f64 * std::f64::consts::TAU / wheel.sats_per_plane as f64 + phases[k];
                let p = terminus_orbits::constellation::polar_sat_position(
                    &planet,
                    wheel.altitude,
                    raan,
                    theta0,
                    t,
                );
                terminus_orbits::constellation::elevation(&planet, town.unit, p)
                    >= policy.min_elevation - policy.hysteresis
            });
            let serving = held.or_else(|| {
                best_visible(&planet, &wheel, town.unit, &phases, Some(&plan), MASK, t)
                    .or_else(|| best_visible(&planet, &wheel, town.unit, &phases, None, MASK, t))
                    .map(|(id, _)| id)
            });
            town.access = serving;
            let Some((k, j)) = serving else { continue };
            // A satellite carrying a session is radiating by definition.
            plan[satellite_index(&wheel, k, j)] = true;

            // The session may leave the ring through any ring mate, so the
            // cost of an anchor is the best path the whole ring can offer, not
            // the distance from the satellite that happens to be serving.
            let ring_pos: Vec<[f64; 3]> = (0..wheel.sats_per_plane)
                .map(|slot| {
                    let raan = k as f64 * std::f64::consts::PI / wheel.planes as f64;
                    let theta0 = slot as f64 * std::f64::consts::TAU / wheel.sats_per_plane as f64
                        + phases[k];
                    terminus_orbits::constellation::polar_sat_position(
                        &planet,
                        wheel.altitude,
                        raan,
                        theta0,
                        t,
                    )
                })
                .collect();
            let route = |a: usize| {
                let ap = anchors_at[a](t);
                exit_gateway(
                    j,
                    wheel.sats_per_plane,
                    terminus_orbits::routing::NECKLACE_LINKS,
                    HOP_RANGE,
                    RELAY_DELAY,
                    |slot| {
                        let d = separation(ring_pos[slot], ap);
                        if d > limb {
                            None
                        } else {
                            let p = ring_pos[slot];
                            Some(
                                ((p[0] - ap[0]).powi(2)
                                    + (p[1] - ap[1]).powi(2)
                                    + (p[2] - ap[2]).powi(2))
                                .sqrt(),
                            )
                        }
                    },
                )
            };
            // The anchor policy still argues in metres of path: the margin is
            // a stated distance, and relays are a thirtieth of what a hop
            // costs, so they decide which door to leave by and not which
            // anchor to hold.
            let path_cost = |a: usize| route(a).map(|g| g.path);
            for (m, &margin) in MARGINS.iter().enumerate() {
                if let Some(pick) = terminus_orbits::backbone::select_anchor(
                    anchors_at.len(),
                    path_cost,
                    town.anchor[m],
                    margin,
                ) {
                    if town.anchor[m].is_some_and(|held| held != pick) {
                        anchor_changes[m] += 1;
                    }
                    town.anchor[m] = Some(pick);
                    if let Some(g) = route(pick) {
                        path_sum[m] += g.path;
                        path_worst[m] = path_worst[m].max(g.path);
                        path_samples[m].push(g.path);
                        hops_worst[m] = hops_worst[m].max(g.hops);
                        // Relays: the serving satellite, plus one for each hop
                        // along the necklace. The anchor terminates.
                        rtt_samples[m].push(
                            2.0 * (user_leg + one_way_latency(g.path, 1 + g.hops, RELAY_DELAY)),
                        );
                        path_n[m] += 1;
                    }
                }
            }
        }
        let lit = plan.clone();
        previous = Some(plan);

        // Sessions as the topology model sees them, plus each access
        // satellite's nearest anchor for the gateway case.
        let mut sessions = Vec::with_capacity(towns.len());
        let mut gateway: BTreeMap<usize, usize> = BTreeMap::new();
        for town in towns.iter() {
            let (Some((k, j)), Some(anchor)) = (town.access, town.anchor[CHOSEN_MARGIN]) else {
                continue;
            };
            let access = satellite_index(&wheel, k, j);
            sessions.push(Session { access, anchor });
            gateway.entry(access).or_insert_with(|| {
                let raan = k as f64 * std::f64::consts::PI / wheel.planes as f64;
                let theta0 =
                    j as f64 * std::f64::consts::TAU / wheel.sats_per_plane as f64 + phases[k];
                let p = terminus_orbits::constellation::polar_sat_position(
                    &planet,
                    wheel.altitude,
                    raan,
                    theta0,
                    t,
                );
                let mut best = (0usize, f64::MAX);
                for (i, at) in anchors_at.iter().enumerate() {
                    let s = separation(p, at(t));
                    if s <= limb && s < best.1 {
                        best = (i, s);
                    }
                }
                best.0
            });
        }

        session_snapshots.push(sessions.clone());
        direct_instants.push(direct_demand(&sessions));
        gateway_instants.push(gateway_demand(&sessions, &gateway));
        // Every satellite must sit within `reach` hops of a feeder host.
        let per_ring = wheel.sats_per_plane;
        let duty = duty_ring(&planet, &wheel, t);
        // Lasers stay powered whatever the radios are doing, so every
        // satellite in a ring relays and the whole ring pools.
        relay_lit.push(uniform_relay_demand(
            &sessions,
            |a| (a / per_ring, a % per_ring),
            |_, _| true,
            per_ring,
            REACH,
        ));
        // For contrast: what the necklace would be worth if a dark radio meant
        // a dark satellite, and only lit satellites relayed.
        relay_all.push(uniform_relay_demand(
            &sessions,
            |a| (a / per_ring, a % per_ring),
            |ring, pos| lit[ring * per_ring + pos],
            per_ring,
            2,
        ));
        let on_duty = (0..per_ring).filter(|&j| lit[duty * per_ring + j]).count();
        lit_counts.push((on_duty, lit.iter().filter(|&&b| b).count() - on_duty));
        t += STEP;
    }

    let direct = TerminalDemand::over_time(direct_instants);
    let gw = TerminalDemand::over_time(gateway_instants);
    let relay = TerminalDemand::over_time(relay_lit);
    let relay_if_only_lit_relayed = TerminalDemand::over_time(relay_all);
    let mean_duty = lit_counts.iter().map(|c| c.0).sum::<usize>() as f64 / lit_counts.len() as f64;
    let mean_other = lit_counts.iter().map(|c| c.1).sum::<usize>() as f64 / lit_counts.len() as f64;

    println!("\n\nB. The direct topology: no ring-to-ring, no anchor-to-anchor\n");
    println!(
        "   {TOWNS} towns, each holding one long-lived session, over {:.0} h.\n\
         \x20  One link per distinct (access satellite, anchor) pair.\n",
        SPAN / 3_600.0
    );
    println!(
        "   feeder terminals on one access satellite: median {}, p90 {}, p99 {}, max {}",
        direct.access_quantile(0.5),
        direct.access_quantile(0.9),
        direct.access_quantile(0.99),
        direct.max_access()
    );
    println!(
        "   feeder terminals on one anchor:           median {}, p90 {}, max {}",
        direct.anchor_quantile(0.5),
        direct.anchor_quantile(0.9),
        direct.max_anchor()
    );

    println!("\n\nC. The gateway topology: one feeder up, the shell routes onward\n");
    println!(
        "   feeder terminals on one access satellite: {} (a second buys make-before-break)",
        gw.max_access()
    );
    println!(
        "   feeder terminals on one anchor:           max {}",
        gw.max_anchor()
    );
    println!(
        "   plus anchor-to-anchor links, which this model does not price --\n\
         \x20  and which section E prices: a plane closes, joining planes steers."
    );

    println!("\n\nD. The necklace, with one drawing for the whole fleet\n");
    println!(
        "   Every satellite is the same satellite, so each carries the worst\n\
         \x20  case -- designating a few feeder hosts would be a second model, and a\n\
         \x20  programme that builds two pays for two of everything. The necklace\n\
         \x20  still helps, and more than it looks: the lasers stay powered even\n\
         \x20  where the radios are dark, so the necklace is a standing relay and a\n\
         \x20  session can chain hops all the way round. The whole ring of {} pools\n\
         \x20  its feeder terminals, {} hops worst case at {:.1} ms a hop.\n",
        wheel.sats_per_plane,
        intra_plane_diameter(&planet, ACCESS_ALT, wheel.sats_per_plane).unwrap_or(0),
        intra_plane_range(&planet, ACCESS_ALT, wheel.sats_per_plane, 1) / 299_792_458.0 * 1e3
    );
    println!(
        "   Of {:.0} satellites lit at a time, {:.0} are the duty ring and {:.0} are\n\
         \x20  singles scattered through the other five rings. With the lasers on,\n\
         \x20  that no longer matters: a single still has eleven ring mates relaying.\n",
        mean_duty + mean_other,
        mean_duty,
        mean_other
    );
    println!(
        "   feeder terminals on EVERY access satellite: median {}, p90 {}, max {}",
        relay.access_quantile(0.5),
        relay.access_quantile(0.9),
        relay.max_access()
    );
    println!(
        "     if a dark radio meant a dark satellite:    max {}",
        relay_if_only_lit_relayed.max_access()
    );
    println!("   plus 2 necklace terminals on every one: 4,437 km, frozen, free to point");
    println!(
        "   feeder terminals on EVERY anchor:           max {}",
        relay.max_anchor()
    );

    // ---- E. the shell's own links ----------------------------------------
    println!(
        "

E. Does the shell need links of its own?
"
    );
    let plane_mate = intra_plane_range(&planet, MEO_ALT, shell.sats_per_plane, 1);
    println!(
        "   Intra-plane: {} anchors 90 deg apart, so each reaches both neighbours\n\
         \x20  and the plane closes as a cycle -- {} hops across it. {:.0} km a hop,\n\
         \x20  {:.0} ms, frozen at {:.2} km/s: two terminals each, pointed once.\n",
        shell.sats_per_plane,
        intra_plane_diameter(&planet, MEO_ALT, shell.sats_per_plane).unwrap_or(0),
        plane_mate / 1e3,
        plane_mate / 299_792_458.0 * 1e3,
        max_shell_range_rate(&planet, MEO_ALT, MEO_ALT) / 1e3
    );

    // How long does a cross-plane pair stay usable, and how many partners does
    // one anchor cycle through in a day?
    let limb_shell = max_shell_separation(&planet, MEO_ALT, MEO_ALT);
    let mut partners = 0usize;
    let mut longest_gap = 0.0f64;
    let a0 = (0usize, 0usize);
    let mut current: Option<(usize, usize)> = None;
    let mut held_since = 0.0f64;
    let mut u = 0.0;
    while u < SPAN {
        let p0 = shell_sat_position(&planet, &shell, a0.0, a0.1, u);
        // Nearest visible partner in another plane.
        let mut best: Option<((usize, usize), f64)> = None;
        for &(k, j) in anchor_ids.iter() {
            if k == a0.0 {
                continue;
            }
            let q = shell_sat_position(&planet, &shell, k, j, u);
            let sep = separation(p0, q);
            if sep <= limb_shell && best.map_or(true, |(_, b)| sep < b) {
                best = Some(((k, j), sep));
            }
        }
        if let Some((id, _)) = best {
            if current != Some(id) {
                if current.is_some() {
                    longest_gap = longest_gap.max(u - held_since);
                    partners += 1;
                }
                current = Some(id);
                held_since = u;
            }
        }
        u += STEP;
    }
    println!(
        "   Inter-plane: one anchor's nearest partner in another plane changes\n\
         \x20  {} times a day, holding {:.1} h at most. {:.2} km/s of Doppler, and a\n\
         \x20  new partner means re-pointing: this terminal steers for a living.\n",
        partners,
        longest_gap / 3_600.0,
        max_intra_shell_range_rate(&planet, &shell, 60.0) / 1e3
    );
    println!(
        "   Whether the shell needs either is a routing question, not a geometry\n\
         \x20  one. With the wheel pooling a whole ring, every ring already reaches\n\
         \x20  every anchor it needs directly, and no session traffic crosses the\n\
         \x20  shell. What would use these links is anchor migration -- gigabytes of\n\
         \x20  working memory, make-before-break -- and clock transfer for the\n\
         \x20  navigation service -- and section F shows the wheel can carry both."
    );

    // ---- F. can the shell do without its own links entirely? -------------
    println!("\n\nF. Anchor to anchor, with no links in the shell at all\n");
    println!(
        "   Classic GPS ran without crosslinks because a ground segment did the\n\
         \x20  work -- and later blocks added them anyway, for autonomy when that\n\
         \x20  segment is out of reach. There is no ground segment here at all, so\n\
         \x20  the question is whether the wheel can stand in: can any two anchors\n\
         \x20  always find an access satellite that sees them both?\n"
    );
    let mut pairs = 0usize;
    let mut unreachable = 0usize;
    let mut worst_relays = usize::MAX;
    let mut u = 0.0;
    while u < SPAN {
        let leo_pos: Vec<[f64; 3]> = (0..wheel.planes)
            .flat_map(|k| (0..wheel.sats_per_plane).map(move |j| (k, j)))
            .map(|(k, j)| {
                let raan = k as f64 * std::f64::consts::PI / wheel.planes as f64;
                let theta0 =
                    j as f64 * std::f64::consts::TAU / wheel.sats_per_plane as f64 + phases[k];
                terminus_orbits::constellation::polar_sat_position(
                    &planet,
                    wheel.altitude,
                    raan,
                    theta0,
                    u,
                )
            })
            .collect();
        let anchor_pos: Vec<[f64; 3]> = anchor_ids
            .iter()
            .map(|&(k, j)| shell_sat_position(&planet, &shell, k, j, u))
            .collect();
        for i in 0..anchor_pos.len() {
            for j in (i + 1)..anchor_pos.len() {
                pairs += 1;
                let relays = leo_pos
                    .iter()
                    .filter(|p| {
                        separation(**p, anchor_pos[i]) <= limb
                            && separation(**p, anchor_pos[j]) <= limb
                    })
                    .count();
                if relays == 0 {
                    unreachable += 1;
                }
                worst_relays = worst_relays.min(relays);
            }
        }
        u += 1_800.0;
    }
    println!(
        "   {} anchor pairs sampled across the day.\n\
         \x20  pairs with no access satellite seeing both:      {}\n\
         \x20  fewest access satellites able to relay any pair: {}\n\
         \n\
         \x20  So the shell needs no links of its own. Two anchors always have a\n\
         \x20  relay below them, and never fewer than a score of candidates -- the\n\
         \x20  wheel is the shell's control segment. A migration is then two feeder\n\
         \x20  hops through one access satellite, which is what its second feeder\n\
         \x20  terminal is for: hold the old anchor and the new one at once, and\n\
         \x20  make-before-break falls out of the hardware already counted.",
        pairs, unreachable, worst_relays
    );

    println!("\n\nH. How often does a session actually move?\n");
    println!(
        "   Migration used to be forced: an anchor sank below the serving\n\
         \x20  satellite's horizon and the session had to follow. It is not forced\n\
         \x20  any more. A ring sees every anchor at every instant, so a session\n\
         \x20  could hold one for ever. What moves it now is the latency policy,\n\
         \x20  and the margin is a choice rather than a fact -- so here is the\n\
         \x20  curve it gets chosen from.\n"
    );
    println!(
        "{:>10} {:>10} {:>9} {:>9} {:>9} {:>5} {:>10} {:>11}",
        "margin (km)",
        "changes/day",
        "mean (km)",
        "p95 (km)",
        "worst (km)",
        "hops",
        "p95 RTT",
        "think left"
    );
    for (m, &margin) in MARGINS.iter().enumerate() {
        let per_day = anchor_changes[m] as f64 / TOWNS as f64 / (SPAN / 86_400.0);
        let mean = path_sum[m] / path_n[m] as f64;
        path_samples[m].sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let p95 = path_samples[m][(0.95 * (path_samples[m].len() - 1) as f64) as usize];
        rtt_samples[m].sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let rtt_ms = rtt_samples[m][(0.95 * (rtt_samples[m].len() - 1) as f64) as usize] * 1e3;
        println!(
            "{:>10.0} {:>10.2} {:>9.0} {:>9.0} {:>9.0} {:>5} {:>7.0} ms {:>8.0} ms{}",
            margin / 1e3,
            per_day,
            mean / 1e3,
            p95 / 1e3,
            path_worst[m] / 1e3,
            hops_worst[m],
            rtt_ms,
            FIRST_TOKEN_BUDGET_MS - rtt_ms,
            if m == CHOSEN_MARGIN {
                "  <- chosen"
            } else {
                ""
            }
        );
    }
    println!(
        "\n   Thinking time left under other first-token budgets. TER-REQ-003\n\
         \x20  is the {:.0} ms column and the policy is chosen under it; the rest\n\
         \x20  show what that requirement costs.\n",
        BUDGETS_MS[0]
    );
    print!("{:>11}", "margin (km)");
    for b in BUDGETS_MS {
        print!("{:>12}", format!("{b:.0} ms"));
    }
    println!();
    for (m, &margin) in MARGINS.iter().enumerate() {
        let rtt_ms = rtt_samples[m][(0.95 * (rtt_samples[m].len() - 1) as f64) as usize] * 1e3;
        print!("{:>11.0}", margin / 1e3);
        for b in BUDGETS_MS {
            let left = b - rtt_ms;
            print!("{:>12}", format!("{left:.0} ms"));
        }
        println!();
    }
    // The p95 is what the policy is argued over, but the compliance matrix has
    // to quote a worst case, so state it rather than leaving it to arithmetic
    // somewhere downstream.
    let worst_rtt = rtt_samples[CHOSEN_MARGIN]
        .last()
        .copied()
        .expect("the chosen margin has samples")
        * 1e3;
    println!(
        "\n   At the chosen margin the worst round trip any session saw was\n\
         \x20  {:.0} ms, against {:.0} ms at the p95 -- so the tail costs {:.0} ms more\n\
         \x20  and still leaves {:.0} ms to think in.\n",
        worst_rtt,
        rtt_samples[CHOSEN_MARGIN][(0.95 * (rtt_samples[CHOSEN_MARGIN].len() - 1) as f64) as usize]
            * 1e3,
        worst_rtt
            - rtt_samples[CHOSEN_MARGIN]
                [(0.95 * (rtt_samples[CHOSEN_MARGIN].len() - 1) as f64) as usize]
                * 1e3,
        FIRST_TOKEN_BUDGET_MS - worst_rtt
    );
    println!(
        "   Holding harder buys fewer migrations and pays for them in path\n\
         \x20  length -- and past a point it pays in thinking time. The RFP allows\n\
         \x20  {:.0} ms to the first token, so whatever the light does not spend,\n\
         \x20  the model gets to think in.\n\n\
         \x20  Both ends of this curve are unaffordable. Chase the shortest path\n\
         \x20  and the backbone carries more working memory than a 100 Gbps link\n\
         \x20  can hold, which `link_throughput` prices. Hold hard enough that a\n\
         \x20  session never moves at all -- 25,000 km, where the backbone carries\n\
         \x20  no working memory except after a failure -- and there are 10 ms left\n\
         \x20  to think in, which is not a budget.\n\n\
         \x20  So the margin is a latency choice and a bandwidth choice at once,\n\
         \x20  and the default is the setting where both are still affordable.",
        FIRST_TOKEN_BUDGET_MS
    );

    // ---- G. what one lost telescope costs -------------------------------
    println!("\n\nG. Losing one telescope\n");
    println!(
        "   The wheel is richly redundant: a ring pools two feeder telescopes on\n\
         \x20  each of twelve satellites, so a session that loses one path borrows\n\
         \x20  another. The shell has no such depth. With the ring pooling, an\n\
         \x20  anchor holds exactly ONE telescope per ring that talks to it, so\n\
         \x20  every (ring, anchor) pair is a single point of failure.\n"
    );
    let ring_of = |a: usize| a / wheel.sats_per_plane;
    let mut worst_pair = 0usize;
    let mut worst_anchor_total = 0usize;
    for inst in session_snapshots.iter() {
        let load = pair_load(inst, ring_of);
        worst_pair = worst_pair.max(load.last().copied().unwrap_or(0));
        let mut by_anchor: std::collections::BTreeMap<usize, usize> =
            std::collections::BTreeMap::new();
        for s in inst.iter() {
            *by_anchor.entry(s.anchor).or_insert(0) += 1;
        }
        worst_anchor_total = worst_anchor_total.max(by_anchor.values().copied().max().unwrap_or(0));
    }
    println!(
        "   Of {} sessions, the busiest single (ring, anchor) telescope carries {},\n\
         \x20  and the busiest anchor holds {} across all six of its links. Lose one\n\
         \x20  telescope and that first number is not degraded, it is stranded: every\n\
         \x20  one of those sessions must re-anchor at once, each dragging its\n\
         \x20  working memory across the sky. A migration storm out of one failure.\n",
        TOWNS, worst_pair, worst_anchor_total
    );
    // What the plane-link cure actually costs. For every (ring, anchor) pair
    // at a spread of instants, compare reaching the anchor directly against
    // reaching it through a plane mate with that pair's telescope dark. Both
    // sides climb from ring slot 0 so the delta is the detour and nothing
    // else.
    let plane_link = intra_plane_range(&planet, MEO_ALT, shell.sats_per_plane, 1);
    let mut detour_extra_ms: Vec<f64> = Vec::new();
    let mut detour_total_ms: Vec<f64> = Vec::new();
    let mut stranded = 0usize;
    let distinct_pairs = wheel.planes * anchor_ids.len();
    for step in 0..INSTANTS {
        let t = step as f64 * 3_600.0;
        let anchor_pos: Vec<[f64; 3]> = anchors_at.iter().map(|f| f(t)).collect();
        for ring in 0..wheel.planes {
            let ring_pos: Vec<[f64; 3]> = (0..wheel.sats_per_plane)
                .map(|slot| {
                    let raan = ring as f64 * std::f64::consts::PI / wheel.planes as f64;
                    let theta0 = slot as f64 * std::f64::consts::TAU / wheel.sats_per_plane as f64
                        + phases[ring];
                    terminus_orbits::constellation::polar_sat_position(
                        &planet,
                        wheel.altitude,
                        raan,
                        theta0,
                        t,
                    )
                })
                .collect();
            let exit_via = |a: usize| {
                let ap = anchor_pos[a];
                exit_gateway(
                    0,
                    wheel.sats_per_plane,
                    terminus_orbits::routing::NECKLACE_LINKS,
                    HOP_RANGE,
                    RELAY_DELAY,
                    |slot| {
                        let p = ring_pos[slot];
                        if separation(p, ap) > limb {
                            None
                        } else {
                            Some(
                                ((p[0] - ap[0]).powi(2)
                                    + (p[1] - ap[1]).powi(2)
                                    + (p[2] - ap[2]).powi(2))
                                .sqrt(),
                            )
                        }
                    },
                )
            };
            for (a, &(plane, slot)) in anchor_ids.iter().enumerate() {
                // A plane of four closes into a cycle; the opposite satellite
                // is permanently behind the planet, so the mates are the two
                // neighbours.
                let n = shell.sats_per_plane;
                let mates = [plane * n + (slot + 1) % n, plane * n + (slot + n - 1) % n];
                // `&exit_via` rather than `exit_via`: the parameter is
                // `impl Fn`, which takes ownership, and the closure is needed
                // twice. `&F` implements `Fn` when `F` does.
                let Some(direct) =
                    feeder_route(a, &mates, |_| true, &exit_via, plane_link, RELAY_DELAY)
                else {
                    continue;
                };
                match feeder_route(a, &mates, |x| x != a, &exit_via, plane_link, RELAY_DELAY) {
                    Some(detour) => {
                        // Round trip: both legs, plus the town's radio hop at
                        // each end, plus the relay on the access satellite.
                        let rt = |one_way: f64| 2.0 * (user_leg + one_way + RELAY_DELAY) * 1e3;
                        detour_extra_ms.push(rt(detour.latency) - rt(direct.latency));
                        detour_total_ms.push(rt(detour.latency));
                    }
                    None => stranded += 1,
                }
            }
        }
    }
    detour_extra_ms.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    detour_total_ms.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let median = detour_extra_ms[detour_extra_ms.len() / 2];
    let worst_extra = *detour_extra_ms.last().expect("samples");
    let worst_total = *detour_total_ms.last().expect("samples");
    let over_nominal = detour_total_ms
        .iter()
        .filter(|ms| **ms > FIRST_TOKEN_BUDGET_MS)
        .count();
    let over_degraded = detour_total_ms
        .iter()
        .filter(|ms| **ms > DEGRADED_BUDGET_MS)
        .count();
    // Say what was measured. This walks EVERY anchor from ring slot 0, not the
    // anchor the policy would have chosen from the town's own serving
    // satellite, so these totals are the geometry's whole range and not
    // section H's population. The conclusion does not turn on the difference --
    // the plane link alone is 249 ms of round trip -- but the basis belongs in
    // the output rather than in a reader's assumption.
    println!(
        "   What the detour costs. Every (ring, anchor) pair walked from ring\n\
         \x20  slot 0 at {} hourly instants -- the geometry's whole range, not\n\
         \x20  the anchors the policy actually holds:\n\n\
         \x20    (ring, anchor) pairs   {} distinct, {} samples\n\
         \x20    extra round trip       median {:.0} ms, worst {:.0} ms\n\
         \x20    worst round trip       {:.0} ms\n\
         \x20    over {:.0} ms nominal     {} of {} samples ({:.0}%)\n\
         \x20    over {:.0} ms degraded    {} of {} samples ({:.0}%)\n\
         \x20    samples with no mate   {}\n\n\
         \x20  The plane link alone costs 249 ms of round trip, so no detour in\n\
         \x20  this geometry can meet the nominal budget. That is arithmetic\n\
         \x20  rather than sampling. What it can meet is the degraded budget,\n\
         \x20  which is the point of having one: the cure is not free, and it\n\
         \x20  was never going to be invisible.\n",
        INSTANTS,
        distinct_pairs,
        detour_total_ms.len(),
        median,
        worst_extra,
        worst_total,
        FIRST_TOKEN_BUDGET_MS,
        over_nominal,
        detour_total_ms.len(),
        100.0 * over_nominal as f64 / detour_total_ms.len() as f64,
        DEGRADED_BUDGET_MS,
        over_degraded,
        detour_total_ms.len(),
        100.0 * over_degraded as f64 / detour_total_ms.len() as f64,
        stranded
    );

    println!(
        "   Two remedies, and they are not the same shape:\n\n\
         \x20    a spare feeder telescope    +1 per anchor  (+{} fleet)\n\
         \x20      A seventh, steerable, cold. It repoints at whichever ring went\n\
         \x20      dark. No traffic moves and no neighbour is burdened -- but it\n\
         \x20      must acquire before it carries, and it protects one failure.\n\n\
         \x20    intra-plane links           +2 per anchor  (+{} fleet)\n\
         \x20      The frozen kind: {:.0} km, {:.2} km/s, pointed once at launch and\n\
         \x20      held. A crippled anchor reaches its ring through a plane mate\n\
         \x20      that still has one. The sessions stay reachable rather than\n\
         \x20      stranded -- but the detour busts the nominal budget and clears\n\
         \x20      only the degraded one, sized for exactly this failure. The plane\n\
         \x20      is a cycle, so either neighbour will do.\n",
        shell.total(),
        2 * shell.total(),
        intra_plane_range(&planet, MEO_ALT, shell.sats_per_plane, 1) / 1e3,
        max_shell_range_rate(&planet, MEO_ALT, MEO_ALT) / 1e3
    );
    println!(
        "   The second costs twice the telescopes and turns a stranding into a\n\
         \x20  detour that meets the degraded budget but not the nominal one. It\n\
         \x20  is also the only path here that does not route a migration through\n\
         \x20  the wheel. Two things it does NOT do. It cannot save a session from\n\
         \x20  an anchor that dies outright --\n\
         \x20  the working memory dies with the machine, and the vault answers that\n\
         \x20  -- and the relaying neighbour now carries two rings' traffic on one\n\
         \x20  telescope, which is a capacity question this model does not price.\n\n\
         \x20  Section F showed the shell needs no links to be REACHABLE. What it\n\
         \x20  costs to STAY reachable through a failure is a different question,\n\
         \x20  and it gets a different answer."
    );

    let fleet_direct = 72 * direct.max_access() + 24 * direct.max_anchor();
    let fleet_gateway = 72 * 2 + 24 * gw.max_anchor();
    println!("\n\nSized to the peak each spacecraft must meet:\n");
    println!(
        "{:>34} {:>10} {:>10} {:>8}",
        "topology", "wheel", "shell", "total"
    );
    println!(
        "{:>34} {:>10} {:>10} {:>8}",
        "direct (no ISLs anywhere)",
        72 * direct.max_access(),
        24 * direct.max_anchor(),
        fleet_direct
    );
    println!(
        "{:>34} {:>10} {:>10} {:>8}",
        "gateway, feeder terminals only",
        72 * 2,
        24 * gw.max_anchor(),
        fleet_gateway
    );
    let ring_wheel = 72 * (relay.max_access() + 2);
    let fleet_relay = ring_wheel + 24 * relay.max_anchor();
    println!(
        "{:>34} {:>10} {:>10} {:>8}",
        "necklace + direct feeder",
        ring_wheel,
        24 * relay.max_anchor(),
        fleet_relay
    );
    // How long a session holds an anchor is policy, not geometry, so it is read
    // off the chosen row of section H rather than off a MEO pass.
    let chosen_per_day = anchor_changes[CHOSEN_MARGIN] as f64 / TOWNS as f64 / (SPAN / 86_400.0);
    let retention = 86_400.0 / chosen_per_day;
    let handover_interval = orbital_period(&planet, ACCESS_ALT) / wheel.sats_per_plane as f64;
    println!(
        "\n   The gateway row counts feeder terminals only, so its total is a\n\
         \x20  floor: its anchor-to-anchor links are real and unpriced. Even so,\n\
         \x20  the simplest topology to describe is the most expensive to build.\n\
         \x20  Retention is why: at the chosen margin a session keeps its anchor\n\
         \x20  for {:.0} min, about {:.0} access handovers, so the sessions riding\n\
         \x20  one access satellite were anchored at different moments from\n\
         \x20  different places, and a direct topology has to make every one of\n\
         \x20  those pairings into hardware. Holding harder makes that worse: this\n\
         \x20  row is the one number in the table that the re-anchor margin moves,\n\
         \x20  and a policy that never re-anchored would scatter the pairings\n\
         \x20  further still.",
        retention / 60.0,
        retention / handover_interval
    );
}
