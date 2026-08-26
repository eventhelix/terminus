//! What a spot beam buys: timing and Doppler uncertainty across the full
//! footprint of a 2,200 km access satellite versus across one narrow
//! phased-array spot, at Ka band, for a tidally locked reference planet.
//!
//! Run: cargo run -p terminus-orbits --example spot_beams

use terminus_orbits::beams::{
    delay_spread_across_spot, doppler_shift, doppler_spread_across_spot, nadir_spot_radius,
    range_rate, slant_range,
};
use terminus_orbits::coverage::footprint_radius;
use terminus_orbits::placement::SPEED_OF_LIGHT;
use terminus_orbits::CentralBody;

const ALT: f64 = 2_200e3;
const KA: f64 = 30e9;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let min_elevation = 25.0_f64.to_radians();
    let edge = footprint_radius(&planet, ALT, min_elevation) / planet.radius;

    println!("Access satellite at 2,200 km, Ka band (30 GHz), 25° min elevation\n");

    let max_rate = range_rate(&planet, ALT, edge);
    let near = slant_range(&planet, ALT, 0.0);
    let far = slant_range(&planet, ALT, edge);
    println!("Across the full footprint (a blanket beam):");
    println!(
        "  range rate at the edges: ±{:.2} km/s  ⇒  Doppler window ±{:.0} kHz",
        max_rate / 1e3,
        doppler_shift(max_rate, KA) / 1e3
    );
    println!(
        "  slant range {:.0}-{:.0} km  ⇒  delay window {:.2} ms wide\n",
        near / 1e3,
        far / 1e3,
        (far - near) / SPEED_OF_LIGHT * 1e3
    );

    let beamwidth = 1.0_f64.to_radians();
    let spot = nadir_spot_radius(ALT, beamwidth);
    println!(
        "One phased-array spot (1° beam ⇒ {:.0} km spot radius), worst case\n\
         at the footprint edge:",
        spot / 1e3
    );
    let df = doppler_spread_across_spot(&planet, ALT, edge, spot, KA);
    let dt = delay_spread_across_spot(&planet, ALT, edge, spot);
    println!("  Doppler spread across the spot: {:.2} kHz", df / 1e3);
    println!("  delay spread across the spot:   {:.0} µs", dt * 1e6);
    println!(
        "\nPrecompensating each beam to its spot center, a terminal sees\n\
         residuals of at most ±{:.1} kHz and ±{:.0} µs — versus a blind\n\
         search over ±{:.0} kHz and {:.2} ms without spot beams: the\n\
         network moves so the terminals never search.",
        df / 2e3,
        dt / 2.0 * 1e6,
        doppler_shift(max_rate, KA) / 1e3,
        (far - near) / SPEED_OF_LIGHT * 1e3
    );
}
