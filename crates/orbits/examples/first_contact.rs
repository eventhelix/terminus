// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Cold-start budget for a terminal that knows nothing — no almanac, no
//! clock, no position — under the reference access constellation (2,200 km,
//! 25° min elevation), with the beacon lantern on X band: the same 0.7 m
//! array that throws a 1° pencil at Ka throws a 3.57° beam at X, and a
//! beam's Doppler spread is set by the aperture alone (v·k/D), so the wider
//! lantern keeps the same ±6 kHz residual while each position covers 13×
//! the ground. The raster is a gap-free covering that follows each beam's
//! true, elongated size: 925 positions, a 9.2 s round. The whole handshake — beacon down, first reply up —
//! stays on X, which also rides through the storms that silence Ka.
//!
//! The receive side never searches either (ADR-0027): each element of the
//! terminal's panel hears the whole visible sky at once, the beacon closes
//! at bare-element gain because it is narrow and slow, and the phase tilt
//! of the arriving wavefront across the face *is* the direction — measured,
//! not found. The alternatives are priced below and both lose: a two-sided
//! raster search blows the budget 6x, and a nested fast receive scan nets
//! -2.2 dB against not scanning at all.
//!
//! Run: cargo run -p terminus-orbits --example first_contact

use terminus_orbits::acquisition::{
    beacon_raster_period, beam_edge_loss_db, covering_raster, doa_rms, raster_in_band,
    raster_link_floor, sky_positions, spots_per_footprint,
};
use terminus_orbits::beams::{
    beam_delay_spread, beam_doppler_spread, doppler_shift, nadir_angle, nadir_spot_radius,
    precompensated_delay_residual, precompensated_doppler_residual, range_rate, ray_ground_angle,
    slant_range, spot_edges, worst_precompensation_residuals,
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
    let edge_angle = footprint_radius(&planet, ALT, min_elevation) / planet.radius;
    let eta_max = nadir_angle(&planet, ALT, edge_angle);
    // The area estimate: footprint cap over a nadir spot's cap. Circles
    // cannot tile, so it leaves gaps; a gap-free hexagonal covering of
    // nadir-sized spots needs 2π/√27 ≈ 1.209x as many.
    let area_spots = spots_per_footprint(&planet, ALT, min_elevation, spot);
    let hex_nadir = area_spots * 2.0 * std::f64::consts::PI / 27.0_f64.sqrt();
    // The raster the lantern walks: a gap-free covering that follows each
    // beam's true, elongated size (ADR-0007).
    let (lattice, (worst_u, worst_eta, _)) = covering_raster(eta_max, x_beam);
    let raster = beacon_raster_period(lattice.len() as f64, BEACON_DWELL);
    let rtt = 2.0 * one_way_light_time(edge_slant_range(&planet, ALT, min_elevation));

    println!("Cold start: terminal with no almanac, no clock, no position\n");
    println!(
        "  sky is never empty (coverage minimum ≥ 1 satellite ≥ 25° up)\n\
         \x20 the lantern is X-band: the {APERTURE} m array that throws a {:.2}° pencil\n\
         \x20 at Ka throws a {:.2}° beam at X — and a beam's Doppler spread is set\n\
         \x20 by the aperture alone (v·k/D): ±{:.1} kHz at Ka, ±{:.1} kHz at X\n\
         \x20 footprint radius: {:.0} km; X spot radius at nadir: {:.1} km",
        ka_beam.to_degrees(),
        x_beam.to_degrees(),
        beam_doppler_spread(&planet, ALT, ka_beam, KA) / 2e3,
        beam_doppler_spread(&planet, ALT, x_beam, X) / 2e3,
        footprint_radius(&planet, ALT, min_elevation) / 1e3,
        spot / 1e3,
    );
    println!(
        "\nTiling the footprint ({} ms beacon dwell per position):\n\
         \x20 area estimate, nadir-sized spots:      {:>5.0} positions  {:>5.1} s  (leaves gaps)\n\
         \x20 gap-free hexagonal, nadir-sized spots: {:>5.0} positions  {:>5.1} s\n\
         \x20 gap-free rings, true elongated spots:  {:>5} positions  {:>5.1} s  <- the raster\n\
         \x20 every direction within {:.3} of a beam's half-power contour (worst at\n\
         \x20 {:.1}° off nadir): beam-edge loss never more than {:.2} dB",
        (BEACON_DWELL * 1e3) as u64,
        area_spots,
        beacon_raster_period(area_spots, BEACON_DWELL),
        hex_nadir,
        beacon_raster_period(hex_nadir, BEACON_DWELL),
        lattice.len(),
        raster,
        worst_u,
        worst_eta.to_degrees(),
        -beam_edge_loss_db(worst_u),
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

    // The Ka column is the 1° reference pencil that spot_beams and ADR-0006
    // price (the 0.7 m face's 70·λ/D gives 0.9993°), so its residuals match
    // the ones published for the service beams.
    let ka_pencil = 1.0_f64.to_radians();
    let edge = footprint_radius(&planet, ALT, min_elevation) / planet.radius;
    let rim_sides = |beam: f64| {
        let (near, far) = spot_edges(&planet, ALT, edge, beam);
        (
            (edge - near) * planet.radius / 1e3,
            (far - edge) * planet.radius / 1e3,
        )
    };
    let (ka_near, ka_far) = rim_sides(ka_pencil);
    let (x_near, x_far) = rim_sides(x_beam);
    let (ka_hz, ka_s) =
        worst_precompensation_residuals(&planet, ALT, min_elevation, ka_pencil, KA, 2_000);
    let (x_hz, x_s) =
        worst_precompensation_residuals(&planet, ALT, min_elevation, x_beam, X, 2_000);
    let blanket_rate = range_rate(&planet, ALT, edge);
    println!(
        "\nOne beam, two bands (worst terminal, swept over a pass):\n\
         \x20                          Ka service    X beacon\n\
         \x20 beamwidth                {:>7.2}°     {:>6.2}°\n\
         \x20 spot radius, nadir       {:>6.1} km   {:>6.1} km\n\
         \x20 rim spot, near · far    {:>3.0} · {:>3.0} km  {:>3.0} · {:>3.0} km\n\
         \x20 Doppler window, blanket   ±{:.0} kHz    ±{:.0} kHz\n\
         \x20 Doppler residual        ±{:>4.2} kHz  ±{:>4.2} kHz\n\
         \x20 delay spread, rim spot     {:.0} µs    {:.2} ms\n\
         \x20 delay residual            ±{:.0} µs   ±{:.2} ms\n\
         The rim spot is lopsided, yet Doppler follows the look angle, not\n\
         the ground: both bands keep the same residual. Delay grows with the\n\
         longer X spot; the satellite opens its reply window to match — a\n\
         wide window, absorbed in orbit.",
        ka_pencil.to_degrees(),
        x_beam.to_degrees(),
        nadir_spot_radius(ALT, ka_pencil) / 1e3,
        spot / 1e3,
        ka_near,
        ka_far,
        x_near,
        x_far,
        doppler_shift(blanket_rate, KA) / 1e3,
        doppler_shift(blanket_rate, X) / 1e3,
        ka_hz / 1e3,
        x_hz / 1e3,
        beam_delay_spread(&planet, ALT, edge, ka_pencil) * 1e6,
        beam_delay_spread(&planet, ALT, edge, x_beam) * 1e3,
        ka_s * 1e6,
        x_s * 1e3,
    );

    // The same two beams straight down: round spots, the same Doppler, and
    // almost no delay spread — the far edge is barely farther than nadir.
    let nadir = |beam: f64, f: f64| {
        let (_, far) = spot_edges(&planet, ALT, 0.0, beam);
        (
            precompensated_doppler_residual(&planet, ALT, 0.0, far, beam, f).abs(),
            beam_delay_spread(&planet, ALT, 0.0, beam),
            precompensated_delay_residual(&planet, ALT, 0.0, far, beam).abs(),
        )
    };
    let (ka_n_hz, ka_n_spread, ka_n_s) = nadir(ka_pencil, KA);
    let (x_n_hz, x_n_spread, x_n_s) = nadir(x_beam, X);
    println!(
        "\nStraight down (the nadir spot):\n\
         \x20                          Ka service    X beacon\n\
         \x20 Doppler residual        ±{:>4.2} kHz  ±{:>4.2} kHz\n\
         \x20 delay spread             {:>5.2} µs   {:>5.2} µs\n\
         \x20 delay residual           ±{:.2} µs    ±{:.2} µs\n\
         Doppler is steepest here and the spots smallest; the two cancel, as\n\
         everywhere. Delay is flattest here: both spots sit inside microseconds.",
        ka_n_hz / 1e3,
        x_n_hz / 1e3,
        ka_n_spread * 1e6,
        x_n_spread * 1e6,
        ka_n_s * 1e6,
        x_n_s * 1e6,
    );

    // ---- the receive side: why the terminal never scans back (ADR-0027) ----
    let x_terminal_beam = beamwidth_deg(TERMINAL_APERTURE, X).to_radians();
    let positions = sky_positions(min_elevation, x_terminal_beam);
    let array_gain = dish_gain_dbi(TERMINAL_APERTURE, X, EFFICIENCY);
    let nested_db = (array_gain - ELEMENT_GAIN_DBI) - 10.0 * positions.log10();
    let noise = thermal_noise_dbw(SYSTEM_NOISE_K, BEACON_BANDWIDTH);
    // The link toward any look direction, at a beam's peak: the satellite's
    // face steered `eta` off nadir (scan loss, as on the terminal), the slant
    // to where that ray lands, and the terminal's bare element leaned to the
    // satellite's zenith angle there (η + γ).
    let tx_dbw = 10.0 * BEACON_TX_POWER_W.log10();
    let link = |eta: f64| {
        let gamma = ray_ground_angle(&planet, ALT, eta);
        let slant = slant_range(&planet, ALT, gamma);
        let sat = planar_array_gain_dbi(APERTURE, X, EFFICIENCY, eta, ROLLOFF);
        let element = ELEMENT_GAIN_DBI + scan_loss_db(eta + gamma, ROLLOFF);
        (sat, slant, element, eta + gamma)
    };
    let peak_db = |eta: f64| {
        let (sat, slant, element, _) = link(eta);
        tx_dbw + sat - fspl_db(slant, X) + element - noise
    };
    // The weakest point anywhere in the footprint: every direction, at its
    // nearest raster beam's edge loss.
    let (snr_db, floor_eta, edge_db) = raster_link_floor(&lattice, eta_max, x_beam, peak_db);
    let (sat_db, slant, element_db, zenith) = link(floor_eta);
    let rx_dbw = snr_db + noise;
    let listen_beam = scanned_beamwidth_deg(TERMINAL_APERTURE, X, zenith);
    let compass = doa_rms(listen_beam, 10.0_f64.powf(snr_db / 10.0));
    let scan = std::f64::consts::FRAC_PI_2 - min_elevation;
    let ka_pencil = scanned_beamwidth_deg(TERMINAL_APERTURE, KA, scan);
    let reply_gain = planar_array_gain_dbi(TERMINAL_APERTURE, X, EFFICIENCY, scan, ROLLOFF);
    let element_rim = ELEMENT_GAIN_DBI + scan_loss_db(scan, ROLLOFF);

    println!(
        "\nThe receive side (ADR-0027): the box never forms a beam to search.\n\
         \x20 each element of its {TERMINAL_APERTURE} m panel hears the whole visible sky at\n\
         \x20 ~{ELEMENT_GAIN_DBI:.0} dBi; at the weakest point anywhere in the footprint —\n\
         \x20 {:.1}° off nadir, slant {:.0} km, element leaned {:.0}°, between raster\n\
         \x20 beams — the {:.0} W lantern still closes at {:.1} dB SNR in its {:.0} kHz\n\
         \x20 channel: detection by correlation against the hard-coded\n\
         \x20 waveform (ADR-0028), inside one {} ms dwell.\n\
         \x20 the face is the compass: the wavefront's phase tilt across the\n\
         \x20 panel fixes the beacon's direction to {compass:.2}° rms there — {:.1}x\n\
         \x20 finer than the {ka_pencil:.2}° Ka pencil it must seed — and the reply\n\
         \x20 returns along the measured wavefront at {reply_gain:.1} dBi at the 65° lean,\n\
         \x20 {:.1} dB over the bare element.",
        floor_eta.to_degrees(),
        slant / 1e3,
        zenith.to_degrees(),
        BEACON_TX_POWER_W,
        snr_db,
        BEACON_BANDWIDTH / 1e3,
        (BEACON_DWELL * 1e3) as u64,
        ka_pencil / compass,
        reply_gain - element_rim,
    );
    println!(
        "\nThe ledger, in decibels (the weakest point in the footprint):\n\
         \x20 lantern transmit power:      +{:.1} dBW   ({:.0} W)\n\
         \x20 satellite X aperture gain:   +{:.1} dBi   ({APERTURE} m, 60% efficient, steered {:.0}°)\n\
         \x20 beam-edge loss:              {:.1} dB    (between raster beams)\n\
         \x20 spreading loss:             -{:.1} dB    ({:.0} km at {:.1} GHz)\n\
         \x20 element gain, leaned {:.0}°:   {:+.1} dBi   ({ELEMENT_GAIN_DBI:.0} dBi patch {:.1} dB lean)\n\
         \x20 power reaching the element: -{:.1} dBW   ({:.0} femtowatts)\n\
         \x20 thermal noise in 50 kHz:    -{:.1} dBW   (kTB at {SYSTEM_NOISE_K:.0} K)\n\
         \x20 signal over noise:           +{:.1} dB    (a {:.0}x power ratio)\n\
         For reference, a beam's peak at the rim: {:.1} dB.",
        tx_dbw,
        BEACON_TX_POWER_W,
        sat_db,
        floor_eta.to_degrees(),
        edge_db,
        fspl_db(slant, X),
        slant / 1e3,
        X / 1e9,
        zenith.to_degrees(),
        element_db,
        element_db - ELEMENT_GAIN_DBI,
        -rx_dbw,
        10.0_f64.powf(rx_dbw / 10.0) * 1e15,
        -noise,
        snr_db,
        10.0_f64.powf(snr_db / 10.0),
        peak_db(eta_max),
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
         stays the ceiling. Rim positions are few and long, so a trimmed round\n\
         counts positions, not area:"
    );
    for (offset, role) in [
        (0.0_f64, " (duty ring)"),
        (10.0, ""),
        (20.0, ""),
        (30.0, " (hole-filler)"),
    ] {
        let n = raster_in_band(
            &planet,
            ALT,
            &lattice,
            20.0_f64.to_radians(),
            offset.to_radians(),
        );
        println!(
            "  {:>4.0}° off the band's center: {:>4} of {} positions ({:>3.0}%)  ->  {:>4.1} s round{}",
            offset,
            n,
            lattice.len(),
            100.0 * n as f64 / lattice.len() as f64,
            beacon_raster_period(n as f64, BEACON_DWELL),
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
