//! What a spot beam buys: timing and Doppler uncertainty across the full
//! footprint of a 2,200 km access satellite versus across one narrow
//! phased-array spot, at Ka band, for a tidally locked reference planet.
//!
//! A leaning beam's spot elongates — farther (slant), flatter (oblique
//! incidence), fatter (scan broadening) — which makes the delay spread grow
//! toward the rim while the Doppler spread stays exactly (f/c)·v·β for every
//! beam. Also prints the field maps behind the footprint figures: signed
//! received Doppler along the ground track (blue ahead of the satellite, red
//! behind) and one-way delay by ground radius (iso-delay contours are
//! concentric circles).
//!
//! Run: cargo run -p terminus-orbits --example spot_beams

use terminus_orbits::beams::{
    beam_doppler_spread, delay_spread_across_spot, doppler_shift, nadir_angle, nadir_spot_radius,
    range_rate, range_rate_at, received_doppler, slant_range, spot_cross_half_extent,
    spot_half_extent,
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
    let beam = 1.0_f64.to_radians();

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

    let nadir_half = spot_half_extent(&planet, ALT, 0.0, beam);
    let edge_half = spot_half_extent(&planet, ALT, edge, beam);
    let edge_cross = spot_cross_half_extent(&planet, ALT, edge, beam);
    println!("One 1° phased-array beam paints:");
    println!(
        "  straight down: a circle of {:.0} km radius",
        nadir_spot_radius(ALT, beam) / 1e3
    );
    println!(
        "  at the footprint edge: an ellipse ±{:.0} km radial × ±{:.0} km cross —",
        edge_half / 1e3,
        edge_cross / 1e3
    );
    println!(
        "    farther (slant {:.2}×) · flatter (1/sin 25° = {:.2}×) ·\n\
         \x20   fatter (scan broadening 1/cos η = {:.2}×)  =  {:.1}× elongation\n",
        far / ALT,
        1.0 / min_elevation.sin(),
        1.0 / nadir_angle(&planet, ALT, edge).cos(),
        edge_half / nadir_half
    );

    let df = beam_doppler_spread(&planet, ALT, beam, KA);
    println!(
        "Doppler spread across any beam's spot: (f/c)·v·β = {:.1} kHz — the\n\
         same for every beam in the footprint: the shift's slope per beam\n\
         angle, (f/c)·v·cos η, and the broadening, 1/cos η, cancel exactly.\n",
        df / 1e3
    );

    println!("Delay spread across a beam's spot grows toward the rim:");
    for frac in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let center = edge * frac;
        let half = spot_half_extent(&planet, ALT, center, beam);
        let us = delay_spread_across_spot(&planet, ALT, center, half).abs() * 1e6;
        let shown = if us < 1.0 {
            format!("{us:.1}")
        } else {
            format!("{us:.0}")
        };
        println!("  {frac:.2} of edge: {shown:>3} µs");
    }
    let dt_edge = delay_spread_across_spot(&planet, ALT, edge, edge_half);
    println!(
        "\nPrecompensating each beam to its spot center, a terminal sees\n\
         residuals of at most ±{:.1} kHz (every spot alike) and ±{:.0} µs\n\
         (under the rim beam) — versus a blind search over ±{:.0} kHz and\n\
         {:.2} ms without spot beams: the network moves so the terminals\n\
         never search.",
        df / 2e3,
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
