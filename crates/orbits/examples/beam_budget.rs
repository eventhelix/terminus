// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! How many simultaneous Ka beams does one access satellite need?
//!
//! One terminal per settlement (it is the settlement's base station,
//! TER-REQ-010), settlements scattered evenly over the habitable band, and
//! the fleet's load shared by the satellites the activation plan keeps lit:
//! 23.1 on average (duty ring plus hole-fillers, `activation_plan`, tag
//! terminus-post-5b). Towns that fall in the same spot share a beam.
//!
//! Two satellites bracket the answer: the average lit satellite, serving
//! its 1/23.1 share of the band, and a duty-ring satellite riding the
//! terminator that serves every town in its footprint alone (95% of the
//! footprint lies on the band) — an upper bound, since neighbors overlap it.
//!
//! Run: cargo run -p terminus-orbits --example beam_budget

use terminus_orbits::acquisition::band_raster_fraction;
use terminus_orbits::beams::{
    band_area, beams_needed, footprint_area, mean_spot_area, nadir_spot_radius,
};
use terminus_orbits::CentralBody;

const ALT: f64 = 2_200e3;
const MASK_DEG: f64 = 25.0;
/// TER-REQ-001: the habitable band, ±20° about the terminator.
const BAND_DEG: f64 = 20.0;
/// Mean satellites lit by the flown policy (duty ring, patch holes, prune):
/// `activation_plan`, tag terminus-post-5b.
const MEAN_LIT: f64 = 23.1;
/// TER-REQ-005: terminals at first light and at the ceiling.
const FLEETS: [(&str, f64); 2] = [("first light", 10_000.0), ("ceiling", 1_000_000.0)];

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let mask = MASK_DEG.to_radians();
    let band_half = BAND_DEG.to_radians();
    let beam = 1.0_f64.to_radians();

    let band = band_area(&planet, band_half);
    let footprint = footprint_area(&planet, ALT, mask);
    let on_band = band_raster_fraction(&planet, ALT, mask, band_half, 0.0);
    let nadir = std::f64::consts::PI * nadir_spot_radius(ALT, beam).powi(2);
    let spot = mean_spot_area(&planet, ALT, mask, beam);

    println!("Beam budget for one access satellite at 2,200 km, 1° Ka beams\n");
    println!(
        "  habitable band (±{BAND_DEG:.0}°):        {:.3e} km²",
        band / 1e6
    );
    println!(
        "  one footprint:                {:.3e} km²  ({:.1}% of the band)",
        footprint / 1e6,
        100.0 * footprint / band
    );
    println!(
        "  spot: {:.0} km² at nadir, {:.0} km² on average over the footprint",
        nadir / 1e6,
        spot / 1e6
    );
    println!("  spots to tile one footprint:  {:.0}\n", footprint / spot);

    let mean_share = band / MEAN_LIT;
    let duty_share = footprint * on_band;
    println!(
        "  average lit satellite serves 1/{MEAN_LIT} of the band: {:.3e} km²",
        mean_share / 1e6
    );
    println!(
        "  duty-ring bound serves its whole on-band footprint ({:.0}%): {:.3e} km²\n",
        100.0 * on_band,
        duty_share / 1e6
    );

    println!(
        "  {:<12} {:>10} {:>9} {:>15}   {:>14} {:>14}",
        "fleet", "terminals", "spacing", "towns/footprint", "beams, average", "beams, bound"
    );
    for (label, towns) in FLEETS {
        let spacing = (band / towns).sqrt();
        let under = towns * duty_share / band;
        let avg = beams_needed(towns * mean_share / band, mean_share, spot);
        let bound = beams_needed(under, duty_share, spot);
        println!(
            "  {label:<12} {towns:>10.0} {:>6.0} km {under:>15.0}   {avg:>14.0} {bound:>14.0}",
            spacing / 1e3
        );
    }
    println!(
        "\n  At first light towns sit far wider apart than a spot, so beams track\n\
         \x20 towns: hundreds per satellite, about a thousand at the bound. At the\n\
         \x20 ceiling towns sit closer than a spot is wide; every spot is lit and\n\
         \x20 the count saturates at the tiling — thousands of beams, the regime\n\
         \x20 where real systems time-share fewer beams across spots (beam hopping)."
    );
}
