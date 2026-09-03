// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Cold-start budget for a terminal that knows nothing — no almanac, no
//! clock, no position — under the reference access constellation (2,200 km,
//! 25° min elevation), with the beacon lantern on X band: the same 0.7 m
//! array that throws a 1° pencil at Ka throws a 3.57° beam at X, and a
//! beam's Doppler spread is set by the aperture alone (v·k/D), so the wider
//! lantern keeps the same ±6 kHz residual while tiling the footprint with
//! 13× fewer positions. The whole handshake — beacon down, first reply up —
//! stays on X, which also rides through the storms that silence Ka.
//!
//! The receive side never searches either (ADR-0027): each element of the
//! terminal's panel hears the whole visible sky at once, the beacon closes
//! at bare-element gain because it is narrow and slow, and the phase tilt
//! of the arriving wavefront across the face *is* the direction — measured,
//! not found. The alternatives are priced below and both lose: a two-sided
//! raster search blows the budget 9x, and a nested fast receive scan nets
//! -2.2 dB against not scanning at all.
//!
//! Run: cargo run -p terminus-orbits --example first_contact

use terminus_orbits::acquisition::{
    band_raster_fraction, beacon_raster_period, doa_rms, sky_positions, spots_per_footprint,
};
use terminus_orbits::beams::{
    beam_doppler_spread, delay_spread_across_spot, nadir_spot_radius, spot_half_extent,
};
use terminus_orbits::coverage::{edge_slant_range, footprint_radius};
use terminus_orbits::placement::one_way_light_time;
use terminus_orbits::radio::{
    beamwidth_deg, dish_gain_dbi, fspl_db, planar_array_gain_dbi, scan_loss_db,
    scanned_beamwidth_deg, thermal_noise_dbw,
};
use terminus_orbits::CentralBody;

const ALT: f64 = 2_200e3;
const APERTURE: f64 = 0.7;
const KA: f64 = 30e9;
const X: f64 = 8.4e9;
const BEACON_DWELL: f64 = 0.010;
const REGISTRATION_ALLOWANCE: f64 = 30.0;
const REQUIREMENT: f64 = 15.0 * 60.0;

/// The terminal panel (ADR-0013): 0.5 m effective aperture, 0.6 efficiency,
/// face-up, electronically steered, scan rolloff 1.2.
const TERMINAL_APERTURE: f64 = 0.5;
const EFFICIENCY: f64 = 0.6;
const ROLLOFF: f64 = 1.2;

/// One radiating element of that panel — a patch over a ground plane —
/// hears the entire sky above it at about this gain. Stated, not derived:
/// the usual figure for such an element.
const ELEMENT_GAIN_DBI: f64 = 5.0;

/// The rendezvous contract (ADR-0028): the only facts frozen into terminal
/// firmware at the factory, chosen because physics and spectrum cannot go
/// stale the way almanacs do — the beacon's carrier, its channel width,
/// and the transmit power behind the satellite's X aperture.
const BEACON_TX_POWER_W: f64 = 10.0;
const BEACON_BANDWIDTH: f64 = 50e3;
const SYSTEM_NOISE_K: f64 = 290.0;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let min_elevation = 25.0_f64.to_radians();
    let ka_beam = beamwidth_deg(APERTURE, KA).to_radians();
    let x_beam = beamwidth_deg(APERTURE, X).to_radians();
    let spot = nadir_spot_radius(ALT, x_beam);
    let spots = spots_per_footprint(&planet, ALT, min_elevation, spot);
    let raster = beacon_raster_period(spots, BEACON_DWELL);
    let rtt = 2.0 * one_way_light_time(edge_slant_range(&planet, ALT, min_elevation));

    println!("Cold start: terminal with no almanac, no clock, no position\n");
    println!(
        "  sky is never empty (coverage minimum ≥ 1 satellite ≥ 25° up)\n\
         \x20 the lantern is X-band: the {APERTURE} m array that throws a {:.2}° pencil\n\
         \x20 at Ka throws a {:.2}° beam at X — and a beam's Doppler spread is set\n\
         \x20 by the aperture alone (v·k/D): ±{:.1} kHz at Ka, ±{:.1} kHz at X\n\
         \x20 footprint radius: {:.0} km; X spot radius: {:.1} km\n\
         \x20 spots to raster:  {:.0} ({} ms beacon dwell each)\n\
         \x20 full beacon raster: {:.1} s",
        ka_beam.to_degrees(),
        x_beam.to_degrees(),
        beam_doppler_spread(&planet, ALT, ka_beam, KA) / 2e3,
        beam_doppler_spread(&planet, ALT, x_beam, X) / 2e3,
        footprint_radius(&planet, ALT, min_elevation) / 1e3,
        spot / 1e3,
        spots,
        (BEACON_DWELL * 1e3) as u64,
        raster,
    );
    println!("\nWorst-case budget vs TER-REQ-008 (15 min):");
    println!(
        "  wait for beacon paint:      {:>6.1} s  (one full raster)",
        raster
    );
    println!(
        "  frequency search:           {:>6.1} s  (none — beam is precompensated)",
        0.0
    );
    println!(
        "  spatial search:             {:>6.1} s  (none — the face is the compass)",
        0.0
    );
    println!(
        "  timing alignment:           {:>6.3} s  (one round trip)",
        rtt
    );
    println!(
        "  registration allowance:     {:>6.1} s",
        REGISTRATION_ALLOWANCE
    );
    let total = raster + rtt + REGISTRATION_ALLOWANCE;
    println!(
        "  total:                      {:>6.1} s  ({:.1} min) — {:.0}x inside the {:.0} min requirement",
        total,
        total / 60.0,
        REQUIREMENT / total,
        REQUIREMENT / 60.0
    );

    let edge = footprint_radius(&planet, ALT, min_elevation) / planet.radius;
    let x_half = spot_half_extent(&planet, ALT, edge, x_beam);
    println!(
        "\nThe lantern's reply window: the X spot at the footprint rim\n\
         stretches to ±{:.0} km, so a first reply lands within ±{:.1} ms of\n\
         the satellite's expectation — a wide window, absorbed in orbit;\n\
         Ka service keeps its ±308 µs.",
        x_half / 1e3,
        delay_spread_across_spot(&planet, ALT, edge, x_half) / 2.0 * 1e3
    );

    // ---- the receive side: why the terminal never scans back (ADR-0027) ----
    let scan = std::f64::consts::FRAC_PI_2 - min_elevation;
    let x_terminal_beam = beamwidth_deg(TERMINAL_APERTURE, X).to_radians();
    let positions = sky_positions(min_elevation, x_terminal_beam);
    let array_gain = dish_gain_dbi(TERMINAL_APERTURE, X, EFFICIENCY);
    let nested_db = (array_gain - ELEMENT_GAIN_DBI) - 10.0 * positions.log10();
    let slant = edge_slant_range(&planet, ALT, min_elevation);
    let eirp = 10.0 * BEACON_TX_POWER_W.log10() + dish_gain_dbi(APERTURE, X, EFFICIENCY);
    let element_worst = ELEMENT_GAIN_DBI + scan_loss_db(scan, ROLLOFF);
    let snr_db = eirp - fspl_db(slant, X) + element_worst
        - thermal_noise_dbw(SYSTEM_NOISE_K, BEACON_BANDWIDTH);
    let listen_beam = scanned_beamwidth_deg(TERMINAL_APERTURE, X, scan);
    let compass = doa_rms(listen_beam, 10.0_f64.powf(snr_db / 10.0));
    let ka_pencil = scanned_beamwidth_deg(TERMINAL_APERTURE, KA, scan);
    let reply_gain = planar_array_gain_dbi(TERMINAL_APERTURE, X, EFFICIENCY, scan, ROLLOFF);

    println!(
        "\nThe receive side (ADR-0027): the box never forms a beam to search.\n\
         \x20 each element of its {TERMINAL_APERTURE} m panel hears the whole visible sky at\n\
         \x20 ~{ELEMENT_GAIN_DBI:.0} dBi; worst case — edge slant {:.0} km, element leaned {:.0}° —\n\
         \x20 the {:.0} W lantern still closes at {:.1} dB SNR in its {:.0} kHz\n\
         \x20 channel: detection by correlation against the hard-coded\n\
         \x20 waveform (ADR-0028), inside one {} ms dwell.\n\
         \x20 the face is the compass: the wavefront's phase tilt across the\n\
         \x20 panel fixes the beacon's direction to {compass:.2}° rms — {:.1}x finer\n\
         \x20 than the {ka_pencil:.2}° Ka pencil it must seed — and the reply returns\n\
         \x20 along the measured wavefront at {reply_gain:.1} dBi, {:.1} dB over the\n\
         \x20 bare element.",
        slant / 1e3,
        scan.to_degrees(),
        BEACON_TX_POWER_W,
        snr_db,
        BEACON_BANDWIDTH / 1e3,
        (BEACON_DWELL * 1e3) as u64,
        ka_pencil / compass,
        reply_gain - element_worst,
    );
    let rx_dbw = eirp - fspl_db(slant, X) + element_worst;
    let noise = thermal_noise_dbw(SYSTEM_NOISE_K, BEACON_BANDWIDTH);
    println!(
        "\nThe ledger, in decibels (worst case: edge slant, 65° lean):\n\
         \x20 lantern transmit power:      +{:.1} dBW   ({:.0} W)\n\
         \x20 satellite X aperture gain:   +{:.1} dBi   ({APERTURE} m, 60% efficient)\n\
         \x20 spreading loss:             -{:.1} dB    ({:.0} km at {:.1} GHz)\n\
         \x20 element gain, leaned 65°:     +{:.1} dBi   ({ELEMENT_GAIN_DBI:.0} dBi patch {:.1} dB lean)\n\
         \x20 power reaching the element: -{:.1} dBW   ({:.0} femtowatts)\n\
         \x20 thermal noise in 50 kHz:    -{:.1} dBW   (kTB at {SYSTEM_NOISE_K:.0} K)\n\
         \x20 signal over noise:           +{:.1} dB    (a {:.0}x power ratio)",
        10.0 * BEACON_TX_POWER_W.log10(),
        BEACON_TX_POWER_W,
        dish_gain_dbi(APERTURE, X, EFFICIENCY),
        fspl_db(slant, X),
        slant / 1e3,
        X / 1e9,
        element_worst,
        scan_loss_db(scan, ROLLOFF),
        -rx_dbw,
        10.0_f64.powf(rx_dbw / 10.0) * 1e15,
        -noise,
        snr_db,
        10.0_f64.powf(snr_db / 10.0),
    );

    println!(
        "\nWhy not scan? the alternatives, priced:\n\
         \x20 unsynchronized receive raster: {positions:.0} sky positions × the {raster:.1} s\n\
         \x20   lantern round = {:.0} minutes — {:.0}x OVER the 15 min requirement\n\
         \x20 nested fast raster ({positions:.0} looks inside each 10 ms dwell):\n\
         \x20   +{:.1} dB of beam gain − {:.1} dB of split integration = {nested_db:.1} dB —\n\
         \x20   worse than not scanning at all",
        positions * raster / 60.0,
        positions * raster / REQUIREMENT,
        array_gain - ELEMENT_GAIN_DBI,
        10.0 * positions.log10(),
    );

    println!(
        "\nThe raster region is one generic rule — footprint ∩ habitable band\n\
         (±20°) — evaluated per satellite; the {raster:.1} s full-footprint round\n\
         stays the ceiling:"
    );
    for (offset, role) in [
        (0.0_f64, " (duty ring)"),
        (10.0, ""),
        (20.0, ""),
        (30.0, " (hole-filler)"),
    ] {
        let frac = band_raster_fraction(
            &planet,
            ALT,
            min_elevation,
            20.0_f64.to_radians(),
            offset.to_radians(),
        );
        println!(
            "  {:>4.0}° off the band's center: {:>3.0}% of footprint  ->  {:>4.1} s round{}",
            offset,
            frac * 100.0,
            frac * raster,
            role
        );
    }

    println!(
        "\nStorms: the same raster is the all-weather lifeline. X loses\n\
         1.9 dB to the storm cell that takes 23.5 dB off Ka, so a terminal\n\
         whose Ka beam drowns waits at most one {raster:.1} s round, answers\n\
         the lantern, and requests sustained X service for its spot."
    );

    println!(
        "\nReacquisition (warm start): the terminal's spot is on the served\n\
         map with a scheduled beam; re-lock is bounded by one beam revisit —\n\
         seconds, against the 30 s requirement."
    );
}
