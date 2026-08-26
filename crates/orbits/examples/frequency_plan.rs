//! Band trade for the access link of a tidally locked reference planet
//! orbiting a radio-loud red dwarf (coherent emission ~1–3 GHz): free-space
//! path loss at the worst-case access slant, terminal beamwidth, and the
//! net two-dish advantage over L-band, per candidate band.
//!
//! Run: cargo run -p terminus-orbits --example frequency_plan

use terminus_orbits::coverage::edge_slant_range;
use terminus_orbits::radio::{beamwidth_deg, dish_gain_dbi, fspl_db};
use terminus_orbits::CentralBody;

const ACCESS_ALT: f64 = 2_200e3;
const DISH_M: f64 = 0.5;
const EFFICIENCY: f64 = 0.6;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let slant = edge_slant_range(&planet, ACCESS_ALT, 25.0_f64.to_radians());
    println!(
        "Access edge slant (2,200 km shell, 25° elevation): {:.0} km",
        slant / 1e3
    );
    println!("Terminal dish: {DISH_M} m, {EFFICIENCY} aperture efficiency\n");

    println!(
        "{:<6} {:>9} {:>11} {:>11} {:>15} {:>17}",
        "band", "f (GHz)", "FSPL (dB)", "beam (deg)", "vs L-band (dB)", "stellar 1-3 GHz?"
    );
    let l_band = 1.6e9;
    let link_figure = |f: f64| 2.0 * dish_gain_dbi(DISH_M, f, EFFICIENCY) - fspl_db(slant, f);
    for (band, f_ghz, in_stellar) in [
        ("L", 1.6, true),
        ("S", 2.5, true),
        ("X", 8.4, false),
        ("Ku", 14.0, false),
        ("Ka", 30.0, false),
    ] {
        let f = f_ghz * 1e9;
        println!(
            "{:<6} {:>9.1} {:>11.1} {:>11.1} {:>+15.1} {:>17}",
            band,
            f_ghz,
            fspl_db(slant, f),
            beamwidth_deg(DISH_M, f),
            link_figure(f) - link_figure(l_band),
            if in_stellar { "IN STELLAR BAND" } else { "clear" }
        );
    }

    println!(
        "\nFor fixed dishes on both ends, doubling frequency costs 6 dB of\n\
         path loss but buys 12 dB of combined dish gain: the higher band\n\
         wins — and the star owns 1-3 GHz outright.\n\
         Plan: Ka (30 GHz) primary, X (8.4 GHz) weather/flare diversity."
    );
}
