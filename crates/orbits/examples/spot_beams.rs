//! What a spot beam buys: timing and Doppler uncertainty across the full
//! footprint of a 2,200 km access satellite versus across one narrow
//! phased-array spot, at Ka band, for a tidally locked reference planet.
//!
//! Also prints the field maps behind the footprint figures: signed received
//! Doppler along the ground track (blue ahead of the satellite, red behind)
//! and one-way delay by ground radius (iso-delay contours are concentric
//! circles).
//!
//! Run: cargo run -p terminus-orbits --example spot_beams

use terminus_orbits::beams::{
    delay_spread_across_spot, doppler_shift, doppler_spread_across_spot, nadir_spot_radius,
    range_rate, range_rate_at, received_doppler, slant_range,
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
    let df_nadir = doppler_spread_across_spot(&planet, ALT, 0.0, spot, KA);
    let dt_nadir = delay_spread_across_spot(&planet, ALT, 0.0, spot);
    let df_edge = doppler_spread_across_spot(&planet, ALT, edge, spot, KA);
    let dt_edge = delay_spread_across_spot(&planet, ALT, edge, spot);
    println!(
        "One phased-array spot (1° beam ⇒ {:.0} km spot radius) — the two\n\
         worst cases live at opposite ends of the footprint:",
        spot / 1e3
    );
    println!(
        "  nadir spot (Doppler worst case): {:.1} kHz spread, {:.1} µs delay spread",
        df_nadir / 1e3,
        dt_nadir.abs() * 1e6
    );
    println!(
        "  edge spot  (delay worst case):   {:.2} kHz spread, {:.0} µs delay spread",
        df_edge / 1e3,
        dt_edge * 1e6
    );
    println!(
        "\nPrecompensating each beam to its spot center, a terminal sees\n\
         residuals of at most ±{:.1} kHz (under the nadir spot) and ±{:.0} µs\n\
         (under the edge spot) — versus a blind search over ±{:.0} kHz and\n\
         {:.2} ms without spot beams: the network moves so the terminals\n\
         never search.",
        df_nadir / 2e3,
        dt_edge / 2.0 * 1e6,
        doppler_shift(max_rate, KA) / 1e3,
        (far - near) / SPEED_OF_LIGHT * 1e3
    );

    println!(
        "\nFootprint edge sits {:.0} km from the sub-satellite point.",
        planet.radius * edge / 1e3
    );
    println!("\nReceived Doppler along the ground track (+ ahead: blue; − behind: red):");
    for frac in [-1.0, -0.75, -0.5, -0.25, 0.25, 0.5, 0.75, 1.0] {
        println!(
            "  {:+.2} of edge: {:+.0} kHz",
            frac,
            received_doppler(range_rate_at(&planet, ALT, edge * frac, 0.0), KA) / 1e3
        );
    }
    println!("\nOne-way delay by ground radius (same at every azimuth):");
    for frac in [0.0, 0.25, 0.5, 0.75, 1.0] {
        println!(
            "  {:.2} of edge: {:.2} ms",
            frac,
            slant_range(&planet, ALT, edge * frac) / SPEED_OF_LIGHT * 1e3
        );
    }
}
