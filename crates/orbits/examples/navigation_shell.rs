//! Sizing the service shell as a navigation constellation: how many
//! satellites the shell needs before four are visible from every point of
//! the inhabited band at every instant, and what that costs on top of the
//! handful the compute anchors need.
//!
//! Run: cargo run -p terminus-orbits --example navigation_shell

use terminus_orbits::constellation::CoverageStats;
use terminus_orbits::walker::{band_coverage, WalkerShell};
use terminus_orbits::CentralBody;

const ALTITUDE: f64 = 20_000e3; // service shell (ADR-0004)
const BAND: f64 = 20.0; // degrees either side of the terminator
const NAV_MASK: f64 = 10.0; // degrees; navigation works a lower mask than access
const INCLINATION: f64 = 55.0; // degrees
const REQUIRED_VISIBLE: usize = 4; // TER-REQ-015

fn shell(planes: usize, sats_per_plane: usize) -> WalkerShell {
    WalkerShell {
        altitude: ALTITUDE,
        planes,
        sats_per_plane,
        inclination: INCLINATION.to_radians(),
        phase_factor: 1.0,
    }
}

fn survey(body: &CentralBody, s: &WalkerShell) -> CoverageStats {
    band_coverage(
        body,
        s,
        BAND.to_radians(),
        NAV_MASK.to_radians(),
        86_400.0, // one Earth day of geometry
        300.0,
        36,
    )
}

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);

    println!(
        "Service shell at {:.0} km, {INCLINATION:.0}° inclined planes, nodes spread over 360°.\n\
         Ground mask {NAV_MASK:.0}°; band ±{BAND:.0}°; one day of geometry sampled every 300 s\n\
         at 36 azimuths × 3 band offsets.\n",
        ALTITUDE / 1e3
    );

    println!("  shell     sats   min visible   mean visible   4 always in view?");
    for sats_per_plane in 1..=6 {
        let s = shell(6, sats_per_plane);
        let stats = survey(&planet, &s);
        println!(
            "  6 × {sats_per_plane}      {:>3}       {:>3}          {:>5.2}         {}",
            s.total(),
            stats.min_visible,
            stats.mean_visible,
            if stats.min_visible >= REQUIRED_VISIBLE {
                "yes"
            } else {
                "no"
            }
        );
    }

    println!();
    for (planes, per_plane) in [(4, 6), (8, 3), (3, 8)] {
        let s = shell(planes, per_plane);
        let stats = survey(&planet, &s);
        println!(
            "  {planes} × {per_plane}      {:>3}       {:>3}          {:>5.2}         {}",
            s.total(),
            stats.min_visible,
            stats.mean_visible,
            if stats.min_visible >= REQUIRED_VISIBLE {
                "yes"
            } else {
                "no"
            }
        );
    }

    let baseline = shell(6, 4);
    let stats = survey(&planet, &baseline);
    println!(
        "
         Twenty-four satellites is where the floor is met: eighteen leave band
         points with three in view, twenty-four never drop below {REQUIRED_VISIBLE}. Six planes of
         four is the GPS-like split — worst case {}, typical {:.1}. At the same
         count the arrangement matters less than the number: 4 × 6 and 8 × 3 clear
         the floor too, with 4 × 6 holding one more in the worst case. Choosing
         between them belongs to the geometry work, not to this sizing.
         
         The same six planes carrying one satellite each — enough to anchor every
         session on the band — leave band points with no fix at all. Navigation is
         what sizes this shell; the minds alone would have been happy with six.
         
         One caution: this is a count, not a geometry quality. Four satellites
         bunched in one quarter of the sky dilute precision, and the GDOP and
         integrity work that turns this count into the 10 m / 100 ns of
         TER-REQ-015 is Series 3.",
        stats.min_visible,
        stats.mean_visible
    );
}
