// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Does the day/night boundary stay put? Solar day and terminator drift for
//! Mercury (3:2 spin-orbit resonance) versus a 1:1 tidally locked,
//! Earth-sized reference planet.
//!
//! Run: cargo run -p terminus-orbits --example terminator_drift

use terminus_orbits::spin_orbit::{solar_day, terminator_drift_speed};

const DAY: f64 = 86_400.0;

fn report(name: &str, radius: f64, rotation_period: f64, orbital_period: f64) {
    println!("{name}:");
    println!(
        "  rotation period: {:>8.3} Earth days",
        rotation_period / DAY
    );
    println!(
        "  orbital period:  {:>8.3} Earth days",
        orbital_period / DAY
    );
    match solar_day(rotation_period, orbital_period) {
        Some(s) => {
            println!("  solar day:       {:>8.1} Earth days", s / DAY);
            println!(
                "  terminator drift at equator: {:.2} m/s",
                terminator_drift_speed(radius, s)
            );
        }
        None => {
            println!("  solar day:       infinite (1:1 locked)");
            println!("  terminator drift at equator: 0 m/s — fixed on the surface");
        }
    }
    println!();
}

fn main() {
    report(
        "Mercury (3:2 resonance)",
        2.4397e6,
        58.646 * DAY,
        87.969 * DAY,
    );
    report(
        "Tidally locked reference planet (11.2-day period)",
        6.371e6,
        11.2 * DAY,
        11.2 * DAY,
    );
}
