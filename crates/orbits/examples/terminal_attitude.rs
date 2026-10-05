// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Terminal attitude and handover pointing (ADR-0031): how a box that
//! landed askew learns enough of its own orientation to find the next
//! satellite, how it then points its Ka pencil, and what its landing tilt
//! costs.
//!
//! 1. Attitude narrows the sky. A TRIAD from gravity (a solid-state
//!    accelerometer) and one bearing, or from two bearings along the
//!    serving satellite's pass, says where the next satellite will be to a
//!    few degrees. Every bearing is priced at the cold-start compass floor
//!    (0.87° rms, first_contact); bearings taken while tracking are sharper.
//! 2. A listening beam sized to that uncertainty hears the incoming
//!    satellite's scheduled X dwell and reads its bearing off the wavefront,
//!    far finer than the Ka pencil needs.
//! 3. The Ka pencil is pointed there and tracks closed-loop.
//!
//! The landing requirement keeps the scan-loss budget honest: the box must
//! right itself to within 5° of level, or ask the villagers to.
//!
//! Run: cargo run -p terminus-orbits --example terminal_attitude

use terminus_orbits::acquisition::doa_rms;
use terminus_orbits::attitude::{
    gravity_and_bearing, handover_pointing_p95, listen_aperture, two_bearings,
};
use terminus_orbits::coverage::edge_slant_range;
use terminus_orbits::radio::{
    beamwidth_deg, fspl_db, planar_array_gain_dbi, scan_loss_db, scanned_beamwidth_deg,
    thermal_noise_dbw,
};
use terminus_orbits::CentralBody;

/// The cold-start compass floor: first_contact's weakest-point bearing.
const BEARING_RMS_DEG: f64 = 0.874;
/// A calibrated solid-state (MEMS) accelerometer's tilt error, rms per
/// axis. Stated, not derived: a common figure for such a part.
const GRAVITY_RMS_DEG: f64 = 0.25;
/// The landing requirement (ADR-0031): upright to within this, by the
/// box's own weighted, rounded base, or by the villagers when asked.
const LANDING_TILT_DEG: f64 = 5.0;
const TERMINAL_APERTURE: f64 = 0.5;
const SAT_APERTURE: f64 = 0.7;
const EFFICIENCY: f64 = 0.6;
const KA: f64 = 30e9;
const X: f64 = 8.4e9;
const ROLLOFF: f64 = 1.2;
const TRIALS: usize = 2_000;
const ALT: f64 = 2_200e3;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let min_el = 25.0_f64.to_radians();
    let (sb, sg) = (BEARING_RMS_DEG.to_radians(), GRAVITY_RMS_DEG.to_radians());
    let tilt = LANDING_TILT_DEG.to_radians();

    // ---- 1. attitude narrows the sky --------------------------------------
    println!(
        "1. Attitude: where the next satellite will be (95th percentile of the\n\
         \x20  worst error over the sky above 25°; bearings {BEARING_RMS_DEG}° rms, gravity\n\
         \x20  {GRAVITY_RMS_DEG}° rms, box tilted {LANDING_TILT_DEG}°):\n"
    );
    println!("   gravity + one bearing, serving satellite at zenith angle:");
    let mut gb = Vec::new();
    for z in [5.0_f64, 15.0, 30.0, 45.0, 64.0] {
        let e = handover_pointing_p95(
            TRIALS,
            tilt,
            min_el,
            11,
            gravity_and_bearing(sg, sb, z.to_radians()),
        )
        .to_degrees();
        gb.push((z, e));
        println!("     {z:>4.0}°: {e:>5.2}°");
    }
    println!("\n   two bearings to the serving satellite, first at 30° zenith, apart by:");
    let mut tb = Vec::new();
    for d in [10.0_f64, 20.0, 40.0, 60.0] {
        let e = handover_pointing_p95(
            TRIALS,
            tilt,
            min_el,
            13,
            two_bearings(sb, 30.0_f64.to_radians(), d.to_radians()),
        )
        .to_degrees();
        tb.push((d, e));
        println!("     {d:>4.0}°: {e:>5.2}°");
    }
    // The policy: gravity + bearing while the serving satellite is at least
    // 30° from the zenith; near the zenith, two bearings at least 40° apart,
    // taken earlier in the pass. The bound is the worse of the two.
    let at = |v: &[(f64, f64)], k: f64| v.iter().find(|(x, _)| *x == k).unwrap().1;
    let bound = at(&gb, 30.0).max(at(&tb, 40.0));
    let ka_half = beamwidth_deg(TERMINAL_APERTURE, KA) / 2.0;
    println!(
        "\n   policy bound: {bound:.2}° — gravity + bearing at ≥30° from the zenith, two\n\
         \x20  bearings ≥40° apart near it. Too coarse to point the {ka_half:.2}° Ka pencil\n\
         \x20  open-loop, fine for aiming a listening beam."
    );

    // ---- 2. a listening beam sized to the uncertainty ------------------------
    // The incoming satellite visits the served spot with a scheduled X dwell
    // (the lantern's 10 W). The box listens with a beam whose half-power
    // half-width covers the attitude bound, at the worst case: the rim slant,
    // the satellite steered 42°, a beam-edge loss of 3 dB, and the box's own
    // face leaned 65° + the landing tilt.
    let half = (bound + 0.5).ceil().to_radians();
    let d_listen = listen_aperture(half, X);
    let scan = (65.0 + LANDING_TILT_DEG).to_radians();
    let listen_gain = planar_array_gain_dbi(d_listen, X, EFFICIENCY, scan, ROLLOFF);
    let element_gain = 5.0 + scan_loss_db(scan, ROLLOFF);
    let slant = edge_slant_range(&planet, ALT, min_el);
    let sat_gain =
        planar_array_gain_dbi(SAT_APERTURE, X, EFFICIENCY, 42.4_f64.to_radians(), ROLLOFF);
    let noise = thermal_noise_dbw(290.0, 50e3);
    let snr = 10.0 + sat_gain - 3.0 - fspl_db(slant, X) + listen_gain - noise;
    let listen_bw = scanned_beamwidth_deg(d_listen, X, scan);
    let bearing = doa_rms(listen_bw, 10.0_f64.powf(snr / 10.0));
    let ka_half_scanned = scanned_beamwidth_deg(TERMINAL_APERTURE, KA, scan) / 2.0;
    println!(
        "\n2. The listening beam: half-width {:.0}° (a {:.2} m patch of the {TERMINAL_APERTURE} m face),\n\
         \x20  {listen_gain:+.1} dBi at the {:.0}° lean against {element_gain:+.1} dBi for a bare element.\n\
         \x20  At the rim slant ({:.0} km) the incoming satellite's X dwell lands at\n\
         \x20  {snr:.1} dB in 50 kHz, and its bearing reads to {bearing:.2}° rms — inside the\n\
         \x20  Ka pencil's {ka_half_scanned:.2}° half-width there ({:.1}x finer).",
        half.to_degrees(),
        d_listen,
        scan.to_degrees(),
        slant / 1e3,
        ka_half_scanned / bearing,
    );
    println!(
        "\n3. The Ka pencil points at that bearing and tracks the new satellite\n\
         \x20  closed-loop (monopulse); every tracked bearing refreshes the attitude\n\
         \x20  for the next handover."
    );

    // ---- the landing requirement -----------------------------------------------
    println!(
        "\nThe landing requirement: the coverage rule promises a satellite 25° up,\n\
         65° from the vertical; a box tilted τ may have to lean 65° + τ:"
    );
    for t in [0.0_f64, 5.0, 10.0, 15.0, 20.0] {
        let lean = (65.0 + t).to_radians();
        println!(
            "  tilt {t:>4.0}°: worst lean {:>3.0}°, scan loss {:>6.2} dB ({:+.2} dB against the −4.49 dB budget)",
            65.0 + t,
            scan_loss_db(lean, ROLLOFF),
            scan_loss_db(lean, ROLLOFF) - scan_loss_db(65.0_f64.to_radians(), ROLLOFF)
        );
    }
    let floor_lean = 63.78_f64;
    println!(
        "\n  at the {LANDING_TILT_DEG}° requirement the first-contact beacon's weakest point\n\
         \x20 (element leaned {floor_lean:.0}°) loses {:.1} dB more: 15.1 dB becomes {:.1} dB.\n\
         \x20 A box that cannot right itself to {LANDING_TILT_DEG}° says so, in words and light,\n\
         \x20 and asks to be set level.",
        scan_loss_db(floor_lean.to_radians(), ROLLOFF)
            - scan_loss_db((floor_lean + LANDING_TILT_DEG).to_radians(), ROLLOFF),
        15.15 + scan_loss_db((floor_lean + LANDING_TILT_DEG).to_radians(), ROLLOFF)
            - scan_loss_db(floor_lean.to_radians(), ROLLOFF),
    );
}
