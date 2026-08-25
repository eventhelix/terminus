//! How round is a tidally locked planet, and what does its bulge do to the
//! rings?
//!
//! The access wheel's whole premise is that its planes stay pinned to the
//! stars while the terminator sweeps past them. A planet's equatorial bulge is
//! what would break that promise: it drags orbital nodes around. This example
//! estimates the reference planet's figure by calibrating the hydrostatic
//! response on Earth, then prices the node drift.
//!
//! Run: cargo run --release -p helixsim-orbits --example planet_figure

use helixsim_orbits::oblateness::{
    flattening, free_rotation_j2, polar_node_drift_rate, rotational_parameter, synchronous_c22,
    synchronous_j2, EARTH_FLUID_LOVE_NUMBER,
};
use helixsim_orbits::CentralBody;

const EARTH_J2: f64 = 1.0826e-3;
const YEAR: f64 = 86_400.0 * 365.25;

fn main() {
    let earth = CentralBody::from_earth_masses(1.0, 6.371e6, 86_164.090_5);
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);

    println!("A. Calibrating the hydrostatic response on Earth\n");
    println!(
        "   A spinning body's bulge is set by q = omega^2 R^3 / mu, the ratio of\n\
         \x20  centrifugal to gravitational pull at the equator, and by how centrally\n\
         \x20  condensed it is (the fluid Love number k2). For a free rotator\n\
         \x20  J2 = k2 q / 3, so Earth's own numbers give us k2.\n"
    );
    let q_e = rotational_parameter(&earth);
    println!("   Earth q                 = {q_e:.4e}");
    println!("   Earth J2 (measured)     = {EARTH_J2:.4e}");
    println!("   => fluid k2             = {:.4}", 3.0 * EARTH_J2 / q_e);
    println!(
        "   check: k2 q / 3         = {:.4e}  (vs measured {EARTH_J2:.4e})",
        free_rotation_j2(&earth, EARTH_FLUID_LOVE_NUMBER)
    );
    println!(
        "   check: flattening       = 1/{:.0}  (vs measured 1/298.3)",
        1.0 / flattening(&earth, EARTH_J2)
    );

    println!("\n\nB. The reference planet: same mass and radius, locked to 11.2 days\n");
    let q_p = rotational_parameter(&planet);
    println!(
        "   Spin is {:.1}x slower, and q goes as omega^2, so q falls {:.0}x:",
        planet.rotation_period / earth.rotation_period,
        q_e / q_p
    );
    println!("   planet q                = {q_p:.4e}");
    println!(
        "\n   But a locked world is not merely a slow rotator. It holds a permanent\n\
         \x20  tidal bulge facing its star, and synchronous rotation ties that bulge to\n\
         \x20  the same q. Hydrostatic equilibrium then gives J2 = 5 k2 q / 6 and\n\
         \x20  C22 = k2 q / 4 - the star's pull adds 2.5x the spin-only figure.\n"
    );
    let j2_rot = free_rotation_j2(&planet, EARTH_FLUID_LOVE_NUMBER);
    let j2 = synchronous_j2(&planet, EARTH_FLUID_LOVE_NUMBER);
    let c22 = synchronous_c22(&planet, EARTH_FLUID_LOVE_NUMBER);
    println!("   spin alone would give J2 = {j2_rot:.4e}");
    println!("   synchronous J2           = {j2:.4e}");
    println!("   synchronous C22          = {c22:.4e}   (J2/C22 = {:.2})", j2 / c22);
    println!("   => {:.0}x rounder than Earth", EARTH_J2 / j2);
    println!(
        "   flattening               = 1/{:.0}   (Earth 1/298)",
        1.0 / flattening(&planet, j2)
    );

    println!("\n\nC. What that does to a polar ring at 2,200 km\n");
    println!(
        "   Node drift goes as cos(inclination), so a perfectly polar ring does not\n\
         \x20  drift at all. What moves a ring is injection error - and on this planet\n\
         \x20  the bulge is too small for that to matter over a fleet's lifetime.\n"
    );
    println!(
        "{:>22} {:>18} {:>20}",
        "inclination error", "locked planet", "Earth's J2, for scale"
    );
    for err_deg in [0.05, 0.1, 0.5, 1.0] {
        let err = (err_deg as f64).to_radians();
        let here = polar_node_drift_rate(&planet, 2_200e3, err, j2);
        let earthlike = polar_node_drift_rate(&planet, 2_200e3, err, EARTH_J2);
        println!(
            "{:>19.2} deg {:>13.2} deg/dec {:>15.1} deg/dec",
            err_deg,
            here.to_degrees() * YEAR * 10.0,
            earthlike.to_degrees() * YEAR * 10.0
        );
    }
    println!(
        "\n   Against 30 deg of ring spacing, a tenth of a degree of injection error\n\
         \x20  costs under half a degree per decade here, and over twenty on an\n\
         \x20  Earth-like planet. The rings stay where they were put."
    );
}
