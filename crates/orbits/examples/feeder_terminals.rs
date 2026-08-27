//! What does the backbone cost in optical terminals?
//!
//! Each end of a laser link needs its own terminal, so the topology question
//! is really a hardware question. Three things are priced here:
//!
//!   A. how far a satellite can see along its own ring, which decides whether
//!      a ring can relay at all, and turns out to answer the question at both
//!      altitudes without any traffic model
//!   B. the direct topology — no ring-to-ring links, no anchor-to-anchor
//!      links, every session reaching its own anchor
//!   C. the gateway topology — each access satellite feeds its nearest anchor
//!      and the shell carries traffic onward
//!
//! The direct topology is the simplest thing to describe and the most
//! expensive thing to build, because sessions hold their anchors across
//! roughly nineteen access handovers and so end up scattered across the shell.
//!
//! Run: cargo run --release -p terminus-orbits --example feeder_terminals

use std::collections::BTreeMap;

use terminus_orbits::activation::{covering_satellites, duty_first_activation, satellite_index};
use terminus_orbits::backbone::{
    intra_plane_range, intra_plane_reach, max_intra_shell_range_rate, max_shell_range_rate,
    max_shell_separation, separation,
};
use terminus_orbits::constellation::{band_point, plane_phases, PhaseMode, PolarConstellation};
use terminus_orbits::duty::duty_ring;
use terminus_orbits::handover::{best_visible, HandoverPolicy};
use terminus_orbits::topology::{
    direct_demand, gateway_demand, relay_demand, Session, TerminalDemand,
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
/// Feeder-equipped satellites per ring. Twelve satellites at two hops of
/// reach need a host at least every five places, so three is the floor.
const FEEDER_HOSTS: usize = 3;

struct Town {
    unit: [f64; 3],
    access: Option<(usize, usize)>,
    anchor: Option<usize>,
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
        "\n   So traffic on the wheel travels at most a hop or two along its ring\n\
         \x20  and no further -- that phrase is geometry, not policy. In the shell a\n\
         \x20  four-satellite plane reaches one neighbour each way and the satellite\n\
         \x20  directly opposite is permanently occulted: a broken necklace."
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
         \x20  satellites sweep past each other however equal their periods. So an\n\
         \x20  anchor-to-anchor link is not a second necklace: only the one plane\n\
         \x20  mate each anchor can reach is free to point, and the shell cannot be\n\
         \x20  connected on those alone -- section A, the necklace that will not close."
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
                anchor: None,
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
    let mut relay_instants: Vec<TerminalDemand> = Vec::new();

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

            let raan = k as f64 * std::f64::consts::PI / wheel.planes as f64;
            let theta0 = j as f64 * std::f64::consts::TAU / wheel.sats_per_plane as f64 + phases[k];
            let serving_at = |u: f64| {
                terminus_orbits::constellation::polar_sat_position(
                    &planet,
                    wheel.altitude,
                    raan,
                    theta0,
                    u,
                )
            };
            if let Some(pick) = terminus_orbits::backbone::select_anchor(
                &planet,
                ACCESS_ALT,
                MEO_ALT,
                serving_at,
                &anchors_at,
                town.anchor,
                t,
            ) {
                town.anchor = Some(pick);
            }
        }
        previous = Some(plan);

        // Sessions as the topology model sees them, plus each access
        // satellite's nearest anchor for the gateway case.
        let mut sessions = Vec::with_capacity(towns.len());
        let mut gateway: BTreeMap<usize, usize> = BTreeMap::new();
        for town in towns.iter() {
            let (Some((k, j)), Some(anchor)) = (town.access, town.anchor) else {
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

        direct_instants.push(direct_demand(&sessions));
        gateway_instants.push(gateway_demand(&sessions, &gateway));
        // Every satellite must sit within `reach` hops of a feeder host.
        let per_ring = wheel.sats_per_plane;
        relay_instants.push(relay_demand(&sessions, |a| a / per_ring, FEEDER_HOSTS));
        t += STEP;
    }

    let direct = TerminalDemand::over_time(direct_instants);
    let gw = TerminalDemand::over_time(gateway_instants);
    let relay = TerminalDemand::over_time(relay_instants);

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
         \x20  and which section A says a 4-satellite plane cannot fully close."
    );

    println!("\n\nD. The necklace: pool each ring, only some satellites feed\n");
    println!(
        "   {} feeder hosts per ring of {}. Anchor spread belongs to the sessions\n\
         \x20  and does not shrink -- but the ring stops paying for it twelve times.\n",
        FEEDER_HOSTS, wheel.sats_per_plane
    );
    println!(
        "   feeder terminals on a host:               median {}, p90 {}, max {}",
        relay.access_quantile(0.5),
        relay.access_quantile(0.9),
        relay.max_access()
    );
    println!(
        "   the other {} satellites in the ring:       0 feeder, 2 necklace each",
        wheel.sats_per_plane - FEEDER_HOSTS
    );
    println!(
        "   feeder terminals on one anchor:           max {}",
        relay.max_anchor()
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
    let ring_wheel = 6 * (FEEDER_HOSTS * relay.max_access()) + 72 * 2;
    let fleet_relay = ring_wheel + 24 * relay.max_anchor();
    println!(
        "{:>34} {:>10} {:>10} {:>8}",
        "necklace + direct feeder",
        ring_wheel,
        24 * relay.max_anchor(),
        fleet_relay
    );
    println!(
        "\n   The gateway row counts feeder terminals only, so its total is a\n\
         \x20  floor: its anchor-to-anchor links are real and unpriced. Even so,\n\
         \x20  the simplest topology to describe is the most expensive to build.\n\
         \x20  Retention is why: a session keeps its anchor across ~19 access\n\
         \x20  handovers, so the sessions riding one access satellite were anchored\n\
         \x20  at different moments from different places, and a direct topology has\n\
         \x20  to make every one of those pairings into hardware."
    );
}
