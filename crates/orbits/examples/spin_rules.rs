// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! The three spin rules of the lock plate on "know your planet": the
//! reference planet run tidally locked (1:1), with no spin at all, and in
//! Mercury's 3:2 resonance — rotations per orbit, sunrises per orbit for a
//! town at a fixed spot on the surface, and the resulting solar day.
//!
//! Run: cargo run -p terminus-orbits --example spin_rules

use terminus_orbits::spin_orbit::{solar_day, sunrises_per_orbit};

const DAY: f64 = 86_400.0;
const ORBIT_DAYS: f64 = 11.2;

fn report(name: &str, rotations_per_orbit: f64) {
    let sunrises = sunrises_per_orbit(rotations_per_orbit);
    let solar = if sunrises == 0.0 {
        "infinite — the star never rises or sets".to_string()
    } else {
        format!(
            "{:.1} Earth days (one sunrise per {:.1} orbits)",
            ORBIT_DAYS / sunrises,
            1.0 / sunrises
        )
    };
    println!("{name}:");
    println!("  rotations per orbit: {rotations_per_orbit:>5.3}");
    println!("  sunrises per orbit:  {sunrises:>5.3}");
    println!("  solar day:           {solar}");
    println!();
}

fn main() {
    println!("Reference planet, orbital period {ORBIT_DAYS} Earth days, under three spin rules:");
    println!();
    report("Locked 1:1", 1.0);
    report("No spin at all", 0.0);
    report("3:2 resonance (Mercury's)", 1.5);

    // Cross-check the 3:2 row against the period form: a rotation period of
    // 2/3 the orbit must give the same solar day as the ratio form.
    let orb = ORBIT_DAYS * DAY;
    let s = solar_day(orb * 2.0 / 3.0, orb).expect("3:2 is not locked");
    println!(
        "cross-check, 3:2 via solar_day(): {:.1} Earth days",
        s / DAY
    );
}
