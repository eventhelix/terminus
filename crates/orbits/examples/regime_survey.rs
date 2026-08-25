//! Survey of circular-orbit altitude regimes for a tidally locked,
//! Earth-sized reference planet (11.2-day period, 0.122-solar-mass star at
//! 0.0485 AU): coverage footprint, edge latency, and pass duration per
//! shelf, then the Hill-sphere verdict on a stationary orbit.
//!
//! Run: cargo run -p helixsim-orbits --example regime_survey

use helixsim_orbits::circular::{orbital_period, synchronous_radius};
use helixsim_orbits::coverage::{
    edge_slant_range, footprint_radius, footprint_radius_limit, max_pass_duration,
};
use helixsim_orbits::hill::{hill_radius, prograde_stability_limit, SUN_MU};
use helixsim_orbits::placement::SPEED_OF_LIGHT;
use helixsim_orbits::{CentralBody, EARTH_MU};

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let min_elevation = 25.0_f64.to_radians();
    let star_mu = 0.122 * SUN_MU;
    let orbital_distance = 7.2555e9;

    println!("Planet mu = GM = {:.4e} m^3/s^2 (Earth mass)", EARTH_MU);
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

    // What the edge user actually pays, and how much of a lap a pass is.
    println!("
Path length and dwell:");
    println!(
        "{:>10} {:>13} {:>13} {:>10} {:>14}",
        "alt (km)", "slant (km)", "vs overhead", "lambda", "% of a lap"
    );
    for altitude_km in [300.0, 1_200.0, 1_800.0, 10_000.0, 20_000.0, 50_000.0] {
        let altitude = altitude_km * 1e3;
        let slant = edge_slant_range(&planet, altitude, min_elevation);
        let lambda = footprint_radius(&planet, altitude, min_elevation) / planet.radius;
        println!(
            "{:>10.0} {:>13.0} {:>12.2}x {:>9.2}° {:>13.1}%",
            altitude_km,
            slant / 1e3,
            slant / altitude,
            lambda.to_degrees(),
            lambda / std::f64::consts::PI * 100.0
        );
    }
    let (lo, hi) = (300e3, 50_000e3);
    let period_ratio = orbital_period(&planet, hi) / orbital_period(&planet, lo);
    let angle_ratio = footprint_radius(&planet, hi, min_elevation)
        / footprint_radius(&planet, lo, min_elevation);
    println!(
        "  dwell grows {:.0}x from 300 to 50,000 km = {:.1}x period x {:.1}x half-angle",
        max_pass_duration(&planet, hi, min_elevation) / max_pass_duration(&planet, lo, min_elevation),
        period_ratio,
        angle_ratio
    );

    println!("
What the 25 deg mask costs (footprint radius):");
    println!(
        "{:>10} {:>14} {:>16} {:>12}",
        "alt (km)", "masked (km)", "horizon (km)", "shrink"
    );
    for altitude_km in [300.0, 1_200.0, 1_800.0, 10_000.0, 20_000.0, 50_000.0] {
        let altitude = altitude_km * 1e3;
        let masked = footprint_radius(&planet, altitude, min_elevation);
        let horizon = footprint_radius(&planet, altitude, 0.0);
        println!(
            "{:>10.0} {:>14.0} {:>16.0} {:>11.2}x",
            altitude_km,
            masked / 1e3,
            horizon / 1e3,
            horizon / masked
        );
    }
    let ceiling = footprint_radius_limit(&planet, min_elevation);
    println!(
        "  ceiling as altitude -> infinity: {:.0} km ({:.0} deg of arc);",
        ceiling / 1e3,
        (std::f64::consts::FRAC_PI_2 - min_elevation).to_degrees()
    );
    println!(
        "  the 50,000 km shelf already holds {:.0}% of it.",
        footprint_radius(&planet, 50_000e3, min_elevation) / ceiling * 100.0
    );

    // Where the stationary shelf comes from, and why a slow spin exiles it.
    // r_sync scales as rotation_period^(2/3), so Earth's own shelf is the
    // natural yardstick.
    let earth = CentralBody::from_earth_masses(1.0, 6.371e6, 23.9344696 * 3_600.0);
    let r_sync_earth = synchronous_radius(&earth);
    let spin_ratio = planet.rotation_period / earth.rotation_period;
    println!("
Stationary shelf, scaled from Earth's:");
    println!(
        "  Earth synchronous radius:  {:>9.0} km (altitude {:.0} km)",
        r_sync_earth / 1e3,
        (r_sync_earth - earth.radius) / 1e3
    );
    println!("  this planet spins         {:>10.2}x slower", spin_ratio);
    println!(
        "  radius grows as T^(2/3):  {:>10.2}x  =>  {:.0} km",
        spin_ratio.powf(2.0 / 3.0),
        r_sync_earth * spin_ratio.powf(2.0 / 3.0) / 1e3
    );

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
    println!(
        "  planet / star mass ratio:  {:>9.3e} (1 : {:.0})",
        planet.mu / star_mu,
        star_mu / planet.mu
    );
    println!(
        "  Hill radius:               {:>9.0} km ({:.2}% of the way to the star)",
        r_hill / 1e3,
        r_hill / orbital_distance * 100.0
    );
    println!("  prograde stability limit:  {:>9.0} km", limit / 1e3);
    println!(
        "  verdict: synchronous radius is {:.1}x the stability limit —\n\
         \x20          no stationary orbit exists around this planet.",
        r_sync / limit
    );
}
