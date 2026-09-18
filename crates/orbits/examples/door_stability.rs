// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Does the route to a *held* anchor flap between the two sides of the ring?
//!
//! A ring is a cycle, so a door on the far side can be reached either way
//! round. `routing::exit_gateway` scores every slot and takes the cheapest,
//! with no memory of the door it chose a moment ago -- so where two doors on
//! opposite sides sit within noise of each other, the minimum can cross over
//! and back, and the path to an unchanged anchor flips sides. Nothing moves
//! when it does (working memory stays where it is: this is a routing event),
//! but packets reorder and forwarding state churns for no gain.
//!
//! The constellation explorer shows this at a wide re-anchor margin, at about
//! a thousand times real speed, which is exactly the frame rate at which a
//! rare event looks constant. So count it in real time instead:
//!
//!   - a **door change** is the gateway slot changing while the anchor and the
//!     serving satellite both stay put -- a genuine re-route to the same place
//!   - a **side flip** is a door change where the signed offset around the ring
//!     changes sign, both sides non-zero: the sidestep swaps hands
//!   - a **flap** is a side flip reversed again within `FLAP_WINDOW`
//!
//! This is not a re-derivation of `feeder_terminals` section H, and its
//! anchor-move column will not match it: 200 towns rather than 1,000, 30 s
//! steps rather than 300, and no activation plan deciding which satellite may
//! serve. A finer step catches crossings a coarse one steps over, so the
//! figure here runs a little high. Section H stays the number the policy is
//! argued from; what this example is for is the shape of the door churn,
//! which no sampling artifact invents.
//!
//! Run: cargo run --release -p terminus-orbits --example door_stability

use terminus_orbits::backbone::{max_shell_separation, select_anchor, separation};
use terminus_orbits::constellation::{band_point, plane_phases, PhaseMode, PolarConstellation};
use terminus_orbits::handover::{best_visible, HandoverPolicy};
use terminus_orbits::routing::{exit_gateway, NECKLACE_LINKS, RELAY_DELAY};
use terminus_orbits::walker::{shell_sat_position, WalkerShell};
use terminus_orbits::CentralBody;

const ACCESS_ALT: f64 = 2_200e3;
const MEO_ALT: f64 = 20_000e3;
const MASK: f64 = 25.0 * std::f64::consts::PI / 180.0;
const HYSTERESIS: f64 = 3.0 * std::f64::consts::PI / 180.0;
const BAND: f64 = 20.0 * std::f64::consts::PI / 180.0;
const HOP_RANGE: f64 = 4_437e3;

/// Fewer towns than `feeder_terminals` and a far finer step: this example is
/// asking how *often* something happens, not how big it is, so time
/// resolution matters more than population.
const TOWNS: usize = 200;
const STEP: f64 = 30.0;
const SPAN: f64 = 86_400.0;

/// Margins to compare: the adopted policy, and the two wide settings where a
/// session is pinned to an anchor its own satellite cannot best reach.
const MARGINS: [f64; 3] = [5_000e3, 20_000e3, 25_000e3];

/// A side flip reversed within this long is a flap rather than a drift.
const FLAP_WINDOW: f64 = 1_800.0;

struct Town {
    unit: [f64; 3],
    access: Option<(usize, usize)>,
    anchor: Vec<Option<usize>>,
    /// Door last used under each margin, and when it was last changed.
    door: Vec<Option<usize>>,
    side: Vec<i32>,
    flipped_at: Vec<Option<f64>>,
}

#[derive(Default, Clone, Copy)]
struct Counts {
    samples: usize,
    offring: usize,
    door_changes: usize,
    side_flips: usize,
    flaps: usize,
    anchor_changes: usize,
    shortest_gap: f64,
}

/// Signed distance from `from` to `to` around a ring of `n`, in slots:
/// negative one way, positive the other, zero for the same satellite.
fn signed_offset(from: usize, to: usize, n: usize) -> i32 {
    let raw = (to + n - from) % n;
    if raw as f64 > n as f64 / 2.0 {
        raw as i32 - n as i32
    } else {
        raw as i32
    }
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

    let mut towns: Vec<Town> = (0..TOWNS)
        .map(|i| {
            let az =
                (i as f64 * 7.3 * std::f64::consts::TAU / TOWNS as f64) % std::f64::consts::TAU;
            let off = ((i % 5) as f64 - 2.0) / 2.0 * BAND;
            Town {
                unit: band_point(az, off),
                access: None,
                anchor: vec![None; MARGINS.len()],
                door: vec![None; MARGINS.len()],
                side: vec![0; MARGINS.len()],
                flipped_at: vec![None; MARGINS.len()],
            }
        })
        .collect();

    let policy = HandoverPolicy::sticky(MASK, HYSTERESIS);
    let limb = max_shell_separation(&planet, ACCESS_ALT, MEO_ALT);
    let mut counts = [Counts {
        shortest_gap: f64::INFINITY,
        ..Default::default()
    }; MARGINS.len()];

    let mut t = 0.0;
    while t < SPAN {
        let anchor_pos: Vec<[f64; 3]> = anchor_ids
            .iter()
            .map(|&(k, j)| shell_sat_position(&planet, &shell, k, j, t))
            .collect();
        let ring_pos: Vec<Vec<[f64; 3]>> = (0..wheel.planes)
            .map(|k| {
                (0..wheel.sats_per_plane)
                    .map(|slot| {
                        let raan = k as f64 * std::f64::consts::PI / wheel.planes as f64;
                        let theta0 = slot as f64 * std::f64::consts::TAU
                            / wheel.sats_per_plane as f64
                            + phases[k];
                        terminus_orbits::constellation::polar_sat_position(
                            &planet,
                            wheel.altitude,
                            raan,
                            theta0,
                            t,
                        )
                    })
                    .collect()
            })
            .collect();

        for town in towns.iter_mut() {
            let held = town.access.filter(|&(k, j)| {
                terminus_orbits::constellation::elevation(&planet, town.unit, ring_pos[k][j])
                    >= policy.min_elevation - policy.hysteresis
            });
            let serving = held.or_else(|| {
                best_visible(&planet, &wheel, town.unit, &phases, None, MASK, t).map(|(id, _)| id)
            });
            let handed_over = serving != town.access;
            town.access = serving;
            let Some((k, j)) = serving else { continue };

            let route = |a: usize| {
                let ap = anchor_pos[a];
                exit_gateway(
                    j,
                    wheel.sats_per_plane,
                    NECKLACE_LINKS,
                    HOP_RANGE,
                    RELAY_DELAY,
                    |slot| {
                        let p = ring_pos[k][slot];
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
            let path_cost = |a: usize| route(a).map(|g| g.path);

            for (m, &margin) in MARGINS.iter().enumerate() {
                let Some(pick) = select_anchor(anchor_ids.len(), path_cost, town.anchor[m], margin)
                else {
                    continue;
                };
                let moved_anchor = town.anchor[m].is_some_and(|h| h != pick);
                if moved_anchor {
                    counts[m].anchor_changes += 1;
                }
                town.anchor[m] = Some(pick);

                let Some(g) = route(pick) else { continue };
                let side = signed_offset(j, g.slot, wheel.sats_per_plane);
                counts[m].samples += 1;
                if g.hops > 0 {
                    counts[m].offring += 1;
                }

                // A door change only counts when nothing else explains it:
                // the anchor is the same one, and the session is still on the
                // same access satellite.
                if let Some(prev) = town.door[m] {
                    if prev != g.slot && !moved_anchor && !handed_over {
                        counts[m].door_changes += 1;
                        let flipped = side != 0
                            && town.side[m] != 0
                            && side.signum() != town.side[m].signum();
                        if flipped {
                            counts[m].side_flips += 1;
                            if let Some(when) = town.flipped_at[m] {
                                let gap = t - when;
                                if gap <= FLAP_WINDOW {
                                    counts[m].flaps += 1;
                                }
                                counts[m].shortest_gap = counts[m].shortest_gap.min(gap);
                            }
                            town.flipped_at[m] = Some(t);
                        }
                    }
                }
                town.door[m] = Some(g.slot);
                town.side[m] = side;
            }
        }
        t += STEP;
    }

    let days = SPAN / 86_400.0;
    println!(
        "Door stability: {TOWNS} towns, {:.0} s steps over {:.0} h.\n\
         \x20 A door change is a re-route to the SAME anchor from the SAME access\n\
         \x20 satellite; a side flip swaps which way round the ring it goes; a flap\n\
         \x20 is a side flip reversed within {:.0} min.\n",
        STEP,
        SPAN / 3600.0,
        FLAP_WINDOW / 60.0
    );
    println!(
        "{:>12} {:>12} {:>14} {:>14} {:>12} {:>16}",
        "margin (km)", "off-ring %", "door chg/day", "side flips/day", "flaps/day", "closest flip"
    );
    for (m, &margin) in MARGINS.iter().enumerate() {
        let c = counts[m];
        let per = |n: usize| n as f64 / TOWNS as f64 / days;
        println!(
            "{:>12.0} {:>11.1}% {:>14.2} {:>14.2} {:>12.2} {:>16}",
            margin / 1e3,
            100.0 * c.offring as f64 / c.samples.max(1) as f64,
            per(c.door_changes),
            per(c.side_flips),
            per(c.flaps),
            if c.shortest_gap.is_finite() {
                format!("{:.0} min", c.shortest_gap / 60.0)
            } else {
                "never twice".to_string()
            },
        );
    }
    println!(
        "\n   Anchor moves per session per day, for scale (a different\n\
         \x20 population and step from section H, so not its 12.70): {}",
        MARGINS
            .iter()
            .enumerate()
            .map(|(m, &margin)| format!(
                "{:.0}k: {:.2}",
                margin / 1e3,
                counts[m].anchor_changes as f64 / TOWNS as f64 / days
            ))
            .collect::<Vec<_>>()
            .join("   ")
    );
}
