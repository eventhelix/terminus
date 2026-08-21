//! Cold-start budget for a terminal that knows nothing — no almanac, no
//! clock, no position — under the reference access constellation (2,200 km,
//! 25° min elevation, 1° spot beams, beacon raster at 10 ms per spot).
//!
//! Run: cargo run -p helixsim-orbits --example first_contact

use helixsim_orbits::acquisition::{beacon_raster_period, spots_per_footprint};
use helixsim_orbits::beams::nadir_spot_radius;
use helixsim_orbits::coverage::{edge_slant_range, footprint_radius};
use helixsim_orbits::placement::one_way_light_time;
use helixsim_orbits::CentralBody;

const ALT: f64 = 2_200e3;
const BEACON_DWELL: f64 = 0.010;
const REGISTRATION_ALLOWANCE: f64 = 30.0;
const REQUIREMENT: f64 = 15.0 * 60.0;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let min_elevation = 25.0_f64.to_radians();
    let spot = nadir_spot_radius(ALT, 1.0_f64.to_radians());
    let spots = spots_per_footprint(&planet, ALT, min_elevation, spot);
    let raster = beacon_raster_period(spots, BEACON_DWELL);
    let rtt = 2.0 * one_way_light_time(edge_slant_range(&planet, ALT, min_elevation));

    println!("Cold start: terminal with no almanac, no clock, no position\n");
    println!(
        "  sky is never empty (coverage minimum ≥ 1 satellite ≥ 25° up)\n\
         \x20 footprint radius: {:.0} km; spot radius: {:.1} km\n\
         \x20 spots to raster:  {:.0} ({} ms beacon dwell each)\n\
         \x20 full beacon raster: {:.0} s ({:.1} min)",
        footprint_radius(&planet, ALT, min_elevation) / 1e3,
        spot / 1e3,
        spots,
        (BEACON_DWELL * 1e3) as u64,
        raster,
        raster / 60.0
    );
    println!("\nWorst-case budget vs TER-REQ-008 (15 min):");
    println!("  wait for beacon paint:      {:>6.1} s  (one full raster)", raster);
    println!("  frequency search:           {:>6.1} s  (none — beam is precompensated)", 0.0);
    println!("  timing alignment:           {:>6.3} s  (one round trip)", rtt);
    println!("  registration allowance:     {:>6.1} s", REGISTRATION_ALLOWANCE);
    let total = raster + rtt + REGISTRATION_ALLOWANCE;
    println!(
        "  total:                      {:>6.1} s  ({:.1} min) — {:.1}x inside the {:.0} min requirement",
        total,
        total / 60.0,
        REQUIREMENT / total,
        REQUIREMENT / 60.0
    );

    println!(
        "\nReacquisition (warm start): the terminal's spot is on the served\n\
         map with a scheduled beam; re-lock is bounded by one beam revisit —\n\
         seconds, against the 30 s requirement."
    );
}
