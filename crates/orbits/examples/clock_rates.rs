//! Relativistic clock rates for the MEO service shell of a tidally locked,
//! Earth-sized reference planet, and the failover arithmetic of the
//! keep-alive layer that guards the routing timetable.
//!
//! Run: cargo run -p terminus-orbits --example clock_rates

use terminus_orbits::hill::SUN_MU;
use terminus_orbits::relativity::{
    fractional_gravitational_blueshift, fractional_velocity_dilation, net_clock_rate_per_day,
    stellar_tidal_rate_per_day,
};
use terminus_orbits::CentralBody;

const MEO_ALT: f64 = 20_000e3;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);

    println!("MEO clock rates vs a surface clock (20,000 km shell):");
    println!(
        "  velocity time dilation:      -{:.2} µs/day (special relativity)",
        fractional_velocity_dilation(&planet, MEO_ALT) * 86_400.0 * 1e6
    );
    println!(
        "  gravitational blueshift:     +{:.2} µs/day (general relativity)",
        fractional_gravitational_blueshift(&planet, MEO_ALT) * 86_400.0 * 1e6
    );
    println!(
        "  net:                         +{:.2} µs/day (Earth GPS: +38.6)",
        net_clock_rate_per_day(&planet, MEO_ALT) * 1e6
    );
    println!(
        "  uncorrected ranging error:   ~{:.1} km/day\n",
        net_clock_rate_per_day(&planet, MEO_ALT) * 299_792_458.0 / 1e3
    );

    let local = stellar_tidal_rate_per_day(0.122 * SUN_MU, 7.2555e9, planet.radius + MEO_ALT);
    let earth = stellar_tidal_rate_per_day(SUN_MU, 1.495979e11, 2.656e7);
    println!("Stellar tidal clock modulation (the non-GPS term):");
    println!("  this system:  ~{:.1} ns/day (periodic)", local * 1e9);
    println!("  Earth GPS:    ~{:.3} ns/day — ignored", earth * 1e9);
    println!(
        "  ratio: {:.0}x — must be modeled against the 100 ns timing budget\n",
        local / earth
    );

    let keepalive = 0.100;
    let missed = 3.0;
    println!("Keep-alive failover budget (timetable alternate switching):");
    println!(
        "  {:.0} ms keep-alives, {:.0} missed ⇒ failure declared in {:.0} ms;\n\
         \x20 switch to the timetable's pre-assigned alternate is immediate —\n\
         \x20 orders of magnitude inside the 60 s single-failure bound.",
        keepalive * 1e3,
        missed,
        missed * keepalive * 1e3
    );
}
