// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! The canonical failure timeline: what happens, second by second, when one
//! feeder telescope dies — and how the outcome turns on which remedies exist.
//!
//!   A. the detour, priced across the geometry's whole range
//!   B. the race: a 5 s acquisition against a policy already moving everyone
//!   C. the timeline, and the four outcomes the two remedies produce
//!
//! Run: cargo run --release -p terminus-orbits --example failure_timeline

use terminus_orbits::backbone::{
    intra_plane_range, max_shell_separation, separation, REANCHOR_MARGIN,
};
use terminus_orbits::constellation::{plane_phases, PhaseMode, PolarConstellation};
use terminus_orbits::coverage::edge_slant_range;
use terminus_orbits::placement::{one_way_light_time, SPEED_OF_LIGHT};
use terminus_orbits::routing::{exit_gateway, feeder_route, ISL_REACQUIRE, RELAY_DELAY};
use terminus_orbits::walker::{shell_sat_position, WalkerShell};
use terminus_orbits::CentralBody;

const ACCESS_ALT: f64 = 2_200e3;
const MEO_ALT: f64 = 20_000e3;
const MASK: f64 = 25.0 * std::f64::consts::PI / 180.0;
/// What a hop costs when a session leaves through a ring mate instead of its
/// own satellite. Terminals point one place each way, so a hop moves one
/// place -- see `routing::NECKLACE_LINKS`.
const HOP_RANGE: f64 = 4_437e3;
/// The RFP's first-token budget (ms, TER-REQ-003), amended to cover
/// failure-free operation only. Whatever the round trip does not spend, the
/// model gets to think in -- which is what makes the margin a latency choice
/// and not only a bandwidth one.
const FIRST_TOKEN_BUDGET_MS: f64 = 300.0;
/// TER-REQ-003's degraded budget (ms): applies while a telescope is dark and
/// traffic rides the plane link in its place.
const DEGRADED_BUDGET_MS: f64 = 600.0;
/// Instants sampled across the day for the detour walk.
const INSTANTS: usize = 24;

/// Keep-alive heartbeat interval (ms), restating ADR-0009's declaration rule:
/// a telescope is declared dead after `MISSED_BEATS` heartbeats of this
/// spacing go unanswered.
const HEARTBEAT_MS: f64 = 100.0;
/// Missed heartbeats before a telescope is declared dead, per ADR-0009.
const MISSED_BEATS: usize = 3;

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
    let limb = max_shell_separation(&planet, ACCESS_ALT, MEO_ALT);

    // The radio leg is the same for every route, so it is measured once, at
    // the edge of the footprint where it is longest.
    let user_leg = one_way_light_time(edge_slant_range(&planet, ACCESS_ALT, MASK));

    // ---- A. what one lost telescope costs, priced across the whole range --
    println!("A. The detour, priced across the geometry's whole range\n");
    println!(
        "   The wheel pools two feeder telescopes per ring on every access\n\
         \x20  satellite, but the pooling still leaves each anchor exactly ONE\n\
         \x20  telescope per ring that talks to it: every (ring, anchor) pair is\n\
         \x20  a single point of failure. What buys the pair back is the frozen\n\
         \x20  intra-plane link -- the severed pair reaches its anchor through a\n\
         \x20  plane mate instead. Walked from ring slot 0 at hourly instants\n\
         \x20  across the day -- the geometry's whole range, not the anchors any\n\
         \x20  policy actually holds -- comparing the direct path against the\n\
         \x20  same pair with its telescope dark:\n"
    );

    let plane_link = intra_plane_range(&planet, MEO_ALT, shell.sats_per_plane, 1);
    let mut direct_total_ms: Vec<f64> = Vec::new();
    let mut detour_extra_ms: Vec<f64> = Vec::new();
    let mut detour_total_ms: Vec<f64> = Vec::new();
    let mut stranded = 0usize;
    let distinct_pairs = wheel.planes * anchor_ids.len();
    for step in 0..INSTANTS {
        let t = step as f64 * 3_600.0;
        let anchor_pos: Vec<[f64; 3]> = anchors_at.iter().map(|f| f(t)).collect();
        for (ring, &phase) in phases.iter().enumerate() {
            let ring_pos: Vec<[f64; 3]> = (0..wheel.sats_per_plane)
                .map(|slot| {
                    let raan = ring as f64 * std::f64::consts::PI / wheel.planes as f64;
                    let theta0 =
                        slot as f64 * std::f64::consts::TAU / wheel.sats_per_plane as f64 + phase;
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
                    feeder_route(a, &mates, |_| true, exit_via, plane_link, RELAY_DELAY)
                else {
                    continue;
                };
                // Round trip: both legs, plus the town's radio hop at each
                // end, plus the relay on the access satellite.
                let rt = |one_way: f64| 2.0 * (user_leg + one_way + RELAY_DELAY) * 1e3;
                // Section A addition: keep the direct side too, so the
                // healthy meter has a canonical value.
                direct_total_ms.push(rt(direct.latency));
                match feeder_route(a, &mates, |x| x != a, exit_via, plane_link, RELAY_DELAY) {
                    Some(detour) => {
                        detour_extra_ms.push(rt(detour.latency) - rt(direct.latency));
                        detour_total_ms.push(rt(detour.latency));
                    }
                    None => stranded += 1,
                }
            }
        }
    }
    direct_total_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    detour_extra_ms.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    detour_total_ms.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let healthy_ms = direct_total_ms[direct_total_ms.len() / 2];
    let median_total = detour_total_ms[detour_total_ms.len() / 2];
    let worst_total = *detour_total_ms.last().expect("samples");
    let median = detour_extra_ms[detour_extra_ms.len() / 2];
    let worst_extra = *detour_extra_ms.last().expect("samples");

    // The floor, not a sample: even the CHEAPEST detour this geometry admits
    // is the radio leg at the footprint edge, the shortest possible feeder
    // (straight up: MEO altitude minus access altitude), the plane link
    // itself, and two relays -- summed one way and doubled for the round
    // trip.
    let floor_feeder = MEO_ALT - ACCESS_ALT;
    let floor_one_way = user_leg
        + one_way_light_time(floor_feeder)
        + one_way_light_time(plane_link)
        + 2.0 * RELAY_DELAY;
    let floor_round_trip_ms = 2.0 * floor_one_way * 1e3;

    println!(
        "     (ring, anchor) pairs                  {} distinct, {} samples\n\
         \x20    healthy round trip (median direct)    {:.0} ms\n\
         \x20    detour round trip (median / worst)     {:.0} ms / {:.0} ms\n\
         \x20    extra round trip (median / worst)      {:.0} ms / {:.0} ms\n\
         \x20    floor (cheapest possible detour)       {:.0} ms\n\
         \x20    samples with no mate                   {}\n\n\
         \x20  The floor is not a sample: even the CHEAPEST detour this geometry\n\
         \x20  admits comes to {:.0} ms of round trip -- the radio leg at the\n\
         \x20  footprint edge, the shortest possible feeder, the plane link\n\
         \x20  itself, and two relays, summed one way and doubled. That busts\n\
         \x20  the {:.0} ms nominal budget and clears the {:.0} ms degraded one\n\
         \x20  with room to spare, so no detour in this geometry can meet the\n\
         \x20  nominal budget -- arithmetic, not sampling. What every detour\n\
         \x20  CAN meet is the degraded budget, which is the point of having\n\
         \x20  one: the cure is not free, and it was never going to be\n\
         \x20  invisible.\n",
        distinct_pairs,
        detour_total_ms.len(),
        healthy_ms,
        median_total,
        worst_total,
        median,
        worst_extra,
        floor_round_trip_ms,
        stranded,
        floor_round_trip_ms,
        FIRST_TOKEN_BUDGET_MS,
        DEGRADED_BUDGET_MS,
    );

    // ---- B. the race: a 5 s acquisition against a policy already moving --
    // The spare restores the direct path, so its benefit is not a route -- it
    // is a deadline. Compare what the detour costs in path against the margin
    // that decides whether the session waits for the spare or simply leaves.
    // `median` is an extra ROUND TRIP in ms. Halve it for one way, convert to
    // seconds, and multiply by c to get the extra path. Uses the crate's own
    // constant rather than a literal, so the figure traces like every other.
    let detour_extra_km = (median / 1e3) * 0.5 * SPEED_OF_LIGHT / 1e3;
    let ratio = detour_extra_km / (REANCHOR_MARGIN / 1e3);
    println!(
        "\n\nB. The race: a {:.0} s acquisition against a policy already moving everyone\n",
        ISL_REACQUIRE
    );
    println!(
        "   The spare telescope, and the race it has to win:\n\n\
         \x20    detour adds        {:.0} km of one-way path (median)\n\
         \x20    re-anchor margin   {:.0} km\n\
         \x20    ratio              {:.1}x the margin\n\
         \x20    spare acquires in  {:.0} s (stated, routing::ISL_REACQUIRE)\n\n\
         \x20  A detoured session is beaten by any rival anchor several times\n\
         \x20  over, so the policy moves it at the next evaluation. The spare\n\
         \x20  only saves the migration if it locks first -- which makes the\n\
         \x20  hold-off a requirement on the POLICY, not just a specification\n\
         \x20  for the hardware: a session whose anchor is known to be\n\
         \x20  reconfiguring must not be re-anchored during the acquisition\n\
         \x20  window. Without that rule the seventh telescope buys nothing,\n\
         \x20  because everyone it was meant to save has already left.\n",
        detour_extra_km,
        REANCHOR_MARGIN / 1e3,
        ratio,
        ISL_REACQUIRE
    );

    // ---- C. the timeline, and what each remedy changes --------------------
    println!("\n\nC. The timeline, and what each remedy changes\n");
    println!(
        "   t = 0            the telescope dies; the (ring, anchor) pair is severed\n\
         \x20  t = {:.0} ms       declared: {} heartbeats of {:.0} ms go unanswered\n\
         \x20  t = {:.0} ms       the route detours through a plane mate ({:.0} km, frozen);\n\
         \x20                   round trip {:.0} ms against the {:.0} ms degraded budget\n\
         \x20  t = {:.1} s        the spare locks, {:.1} s after the break: the direct\n\
         \x20                   path returns and the round trip is {:.0} ms again\n",
        MISSED_BEATS as f64 * HEARTBEAT_MS,
        MISSED_BEATS,
        HEARTBEAT_MS,
        MISSED_BEATS as f64 * HEARTBEAT_MS,
        plane_link / 1e3,
        worst_total,
        DEGRADED_BUDGET_MS,
        ISL_REACQUIRE,
        ISL_REACQUIRE,
        healthy_ms
    );
    println!(
        "   Four outcomes, two remedies. The detour adds {:.1}x the re-anchor\n\
         \x20  margin, so the policy moves a detoured session at its next\n\
         \x20  evaluation -- unless a rule stops it:\n\n\
         \x20    spare + hold-off   the session is pinned on the detour until the\n\
         \x20                       spare locks; it never leaves\n\
         \x20    spare alone        the bucket drains before the {:.0} s lock; the\n\
         \x20                       spare locks onto an empty bucket\n\
         \x20    hold-off alone     sessions sit at {:.0} ms waiting for a lock that\n\
         \x20                       never comes, then migrate anyway\n\
         \x20    neither            immediate migration at the next evaluation\n",
        ratio, ISL_REACQUIRE, worst_total
    );
}
