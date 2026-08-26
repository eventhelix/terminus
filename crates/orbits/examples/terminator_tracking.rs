//! Cost of actively rotating an orbital plane to track the terminator of a
//! tidally locked, Earth-sized reference planet (11.2-day rotation).
//!
//! Run: cargo run -p terminus-orbits --example terminator_tracking

use terminus_orbits::circular::orbital_velocity;
use terminus_orbits::plane_tracking::{
    cross_track_acceleration, ideal_plane_change_dv_per_day, propellant_fraction_per_day,
    remaining_mass_fraction, terminator_rate,
};
use terminus_orbits::CentralBody;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let spacecraft_mass_kg = 500.0;
    let isp_s = 3_000.0;

    let rate = terminator_rate(&planet);
    println!(
        "Terminator rotation: {:.4} deg/day ({:.4e} rad/s)\n",
        rate.to_degrees() * 86_400.0,
        rate
    );

    println!(
        "{:>10} {:>12} {:>16} {:>14} {:>12} {:>16}",
        "alt (km)", "v (km/s)", "dv/day (km/s)", "accel (m/s2)", "thrust (N)", "propellant %/day"
    );
    for altitude_km in [600.0, 1_200.0, 1_800.0, 2_000.0] {
        let altitude = altitude_km * 1e3;
        let v = orbital_velocity(&planet, altitude);
        let dv = ideal_plane_change_dv_per_day(&planet, altitude);
        let a = cross_track_acceleration(&planet, altitude);
        println!(
            "{:>10.0} {:>12.2} {:>16.2} {:>14.4} {:>12.1} {:>16.1}",
            altitude_km,
            v / 1e3,
            dv / 1e3,
            a,
            a * spacecraft_mass_kg,
            propellant_fraction_per_day(&planet, altitude, isp_s) * 100.0
        );
    }

    let alt = 1_800e3;
    println!(
        "\nCompounding at 1,800 km, Isp = {isp_s} s:\n\
         mass remaining after one local year (11.2 Earth days): {:.1}%\n\
         mass remaining after 30 Earth days: {:.1}%",
        remaining_mass_fraction(&planet, alt, isp_s, 11.2) * 100.0,
        remaining_mass_fraction(&planet, alt, isp_s, 30.0) * 100.0
    );

    println!(
        "\nAt Isp = {isp_s} s the best case burns >12% of spacecraft mass per day.\n\
         Continuous terminator tracking is not sustainable; use fixed planes\n\
         and hand service across them instead."
    );
}
