//! Coverage of the ±20° terminator band by candidate access
//! constellations: polar rings, minimum user elevation 25°, evaluated over
//! one full 11.2-day rotation of a tidally locked, Earth-sized reference
//! planet. Every band point is sampled every 30 s at 72 azimuths × 3
//! band offsets.
//!
//! Run: cargo run --release -p helixsim-orbits --example access_constellation

use helixsim_orbits::constellation::{band_coverage, PolarConstellation};
use helixsim_orbits::CentralBody;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let min_elevation = 25.0_f64.to_radians();
    let band_halfwidth = 20.0_f64.to_radians();
    let duration = 11.2 * 86_400.0;
    let step = 30.0;
    let azimuth_samples = 72;

    println!("Band ±20°, min elevation 25°, polar rings, full 11.2-day rotation\n");
    println!(
        "{:>10} {:>7} {:>10} {:>7} {:>13} {:>14}",
        "alt (km)", "rings", "sats/ring", "total", "min visible", "mean visible"
    );
    for (alt_km, planes, sats_per_plane) in [
        (1_800.0, 6, 12),
        (1_800.0, 6, 14),
        (1_800.0, 7, 12),
        (2_000.0, 6, 12),
        (2_200.0, 6, 12),
        (2_400.0, 6, 12),
    ] {
        let c = PolarConstellation {
            altitude: alt_km * 1e3,
            planes,
            sats_per_plane,
            interplane_phase: 0.0,
        };
        let stats = band_coverage(
            &planet,
            &c,
            band_halfwidth,
            min_elevation,
            duration,
            step,
            azimuth_samples,
        );
        println!(
            "{:>10.0} {:>7} {:>10} {:>7} {:>13} {:>14.2}",
            alt_km,
            planes,
            sats_per_plane,
            planes * sats_per_plane,
            stats.min_visible,
            stats.mean_visible
        );
    }

    println!(
        "\nBaseline: 6 rings x 12 at 2,200 km (72 satellites, continuous\n\
         coverage with margin). Preferred-ring handoff cadence: 180°/6 rings\n\
         = 30° of terminator rotation at 32.14°/day ≈ every 22.4 hours."
    );
}
