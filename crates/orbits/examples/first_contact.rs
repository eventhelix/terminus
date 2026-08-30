//! Cold-start budget for a terminal that knows nothing — no almanac, no
//! clock, no position — under the reference access constellation (2,200 km,
//! 25° min elevation), with the beacon lantern on X band: the same 0.7 m
//! array that throws a 1° pencil at Ka throws a 3.57° beam at X, and a
//! beam's Doppler spread is set by the aperture alone (v·k/D), so the wider
//! lantern keeps the same ±6 kHz residual while tiling the footprint with
//! 13× fewer positions. The whole handshake — beacon down, first reply up —
//! stays on X, which also rides through the storms that silence Ka.
//!
//! Run: cargo run -p terminus-orbits --example first_contact

use terminus_orbits::acquisition::{beacon_raster_period, spots_per_footprint};
use terminus_orbits::beams::{
    beam_doppler_spread, delay_spread_across_spot, nadir_spot_radius, spot_half_extent,
};
use terminus_orbits::coverage::{edge_slant_range, footprint_radius};
use terminus_orbits::placement::one_way_light_time;
use terminus_orbits::radio::beamwidth_deg;
use terminus_orbits::CentralBody;

const ALT: f64 = 2_200e3;
const APERTURE: f64 = 0.7;
const KA: f64 = 30e9;
const X: f64 = 8.4e9;
const BEACON_DWELL: f64 = 0.010;
const REGISTRATION_ALLOWANCE: f64 = 30.0;
const REQUIREMENT: f64 = 15.0 * 60.0;

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
