//! Survey of circular-orbit altitude regimes for a tidally locked,
//! Earth-sized reference planet (11.2-day period, 0.122-solar-mass star at
//! 0.0485 AU): coverage footprint, edge latency, and pass duration per
//! shelf, then the Hill-sphere verdict on a stationary orbit.
//!
//! Run: cargo run -p helixsim-orbits --example regime_survey

use helixsim_orbits::circular::{orbital_period, synchronous_radius};
use helixsim_orbits::coverage::{edge_slant_range, footprint_radius, max_pass_duration};
use helixsim_orbits::hill::{hill_radius, prograde_stability_limit, SUN_MU};
use helixsim_orbits::CentralBody;

const SPEED_OF_LIGHT: f64 = 299_792_458.0;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let min_elevation = 25.0_f64.to_radians();
    let star_mu = 0.122 * SUN_MU;
    let orbital_distance = 7.2555e9;

    println!("Minimum user elevation: 25 deg\n");
    println!(
        "{:>10} {:>12} {:>16} {:>18} {:>16}",
        "alt (km)", "period (h)", "footprint (km)", "edge latency (ms)", "max pass (min)"
    );
    for altitude_km in [300.0, 1_200.0, 1_800.0, 10_000.0, 20_000.0, 50_000.0] {
        let altitude = altitude_km * 1e3;
        println!(
            "{:>10.0} {:>12.2} {:>16.0} {:>18.1} {:>16.1}",
            altitude_km,
            orbital_period(&planet, altitude) / 3_600.0,
            footprint_radius(&planet, altitude, min_elevation) / 1e3,
            edge_slant_range(&planet, altitude, min_elevation) / SPEED_OF_LIGHT * 1e3,
            max_pass_duration(&planet, altitude, min_elevation) / 60.0
        );
    }

    let r_sync = synchronous_radius(&planet);
    let r_hill = hill_radius(&planet, star_mu, orbital_distance);
    let limit = prograde_stability_limit(&planet, star_mu, orbital_distance);
    let sync_altitude = r_sync - planet.radius;
    println!("\nStationary orbit check:");
    println!(
        "  synchronous radius:        {:>9.0} km (altitude {:.0} km)",
        r_sync / 1e3,
        sync_altitude / 1e3
    );
    println!(
        "  edge latency if it existed: {:>8.0} ms one way",
        edge_slant_range(&planet, sync_altitude, min_elevation) / SPEED_OF_LIGHT * 1e3
    );
    println!("  Hill radius:               {:>9.0} km", r_hill / 1e3);
    println!("  prograde stability limit:  {:>9.0} km", limit / 1e3);
    println!(
        "  verdict: synchronous radius is {:.1}x the stability limit —\n\
         \x20          no stationary orbit exists around this planet.",
        r_sync / limit
    );
}
