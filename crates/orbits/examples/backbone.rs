// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Backbone geometry for the reference architecture: intra-ring laser
//! links, LEO→MEO feeder-link visibility and Doppler, and the light-time
//! spread the anchor-selection policy works within.
//!
//! Run: cargo run -p terminus-orbits --example backbone

use terminus_orbits::backbone::{
    intra_plane_neighbor_range, max_shell_range_rate, max_shell_separation, shell_visible_fraction,
};
use terminus_orbits::placement::{one_way_light_time, shell_distance};
use terminus_orbits::CentralBody;

const LEO: f64 = 2_200e3;
const MEO: f64 = 20_000e3;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);

    let chord = intra_plane_neighbor_range(&planet, LEO, 12);
    println!("Intra-ring links (12 satellites per ring at 2,200 km):");
    println!(
        "  neighbor range: {:.0} km, constant — zero relative motion, zero\n\
         \x20 Doppler: point the lasers once and hold forever\n",
        chord / 1e3
    );

    let psi = max_shell_separation(&planet, LEO, MEO);
    println!("LEO → MEO feeder links (2,200 km → 20,000 km):");
    println!(
        "  mutual visibility out to {:.1}° of separation — {:.0}% of the\n\
         \x20 entire MEO shell is above the limb from any access satellite",
        psi.to_degrees(),
        shell_visible_fraction(&planet, LEO, MEO) * 100.0
    );
    for (label, sep_deg) in [
        ("overhead", 0.0_f64),
        ("60° (budget policy)", 60.0),
        ("limb-to-limb", psi.to_degrees()),
    ] {
        let d = shell_distance(&planet, LEO, MEO, sep_deg.to_radians());
        println!(
            "  range at {label:<20} {:>8.0} km  ({:>5.1} ms one way)",
            d / 1e3,
            one_way_light_time(d) * 1e3
        );
    }
    println!(
        "  worst-case range rate: {:.2} km/s — large, but fully deterministic\n\
         \x20 between two known orbits: precompensated exactly like the user\n\
         \x20 beams (ADR-0006)\n",
        max_shell_range_rate(&planet, LEO, MEO) / 1e3
    );

    println!(
        "Routing: one duty ring serves the band at a time (two adjacent\n\
         rings only during the 22.4 h seam window), and every ring reaches\n\
         the MEO shell directly — no inter-ring links required.\n\
         Synchronization: two-way time transfer over these same links;\n\
         MEO master clocks discipline each ring — the timing fabric PNT\n\
         (volume 3) will inherit."
    );
}
