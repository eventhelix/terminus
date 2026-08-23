//! Option C trade: could the MEO shell serve users directly, eliminating
//! the LEO access rings? Prices the aperture penalty, spot-reuse density,
//! and the (real) dynamical advantages of MEO-direct access.
//!
//! Run: cargo run -p helixsim-orbits --example access_trade

use helixsim_orbits::acquisition::spots_per_footprint;
use helixsim_orbits::beams::{doppler_shift, nadir_spot_radius, range_rate};
use helixsim_orbits::circular::orbital_period;
use helixsim_orbits::coverage::{edge_slant_range, footprint_radius, max_pass_duration};
use helixsim_orbits::radio::fspl_db;
use helixsim_orbits::CentralBody;

const LEO: f64 = 2_200e3;
const MEO: f64 = 20_000e3;
const KA: f64 = 30e9;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let e = 25.0_f64.to_radians();
    let beam = 1.0_f64.to_radians();

    let slant_leo = edge_slant_range(&planet, LEO, e);
    let slant_meo = edge_slant_range(&planet, MEO, e);
    let delta_db = fspl_db(slant_meo, KA) - fspl_db(slant_leo, KA);
    let dish_factor = 10f64.powf(delta_db / 20.0);

    println!("MEO-direct access (20,000 km) vs LEO access rings (2,200 km):\n");
    println!(
        "  worst-case slant: {:.0} km vs {:.0} km  ⇒  +{:.1} dB path loss",
        slant_meo / 1e3,
        slant_leo / 1e3,
        delta_db
    );
    println!(
        "  to recover at the terminal alone: 0.5 m dish → {:.1} m, or {:.0}x power",
        0.5 * dish_factor,
        10f64.powf(delta_db / 10.0)
    );
    println!(
        "  splitting recovery with the satellite: terminal still needs {:.2} m\n",
        0.5 * 10f64.powf(delta_db / 40.0)
    );

    let spot_leo = nadir_spot_radius(LEO, beam);
    let spot_meo = nadir_spot_radius(MEO, beam);
    println!(
        "  1° spot radius: {:.0} km vs {:.1} km  ⇒  {:.0}x less spatial reuse\n\
         \x20 (the 10^4 → 10^6 terminal scaling leans on small spots)\n",
        spot_meo / 1e3,
        spot_leo / 1e3,
        (spot_meo / spot_leo).powi(2)
    );

    let edge_meo = footprint_radius(&planet, MEO, e) / planet.radius;
    println!("  MEO-direct advantages, acknowledged:");
    // Passes and handovers are different clocks: a pass is the longest one
    // satellite could serve, while handovers come one in-plane spacing apart
    // (ADR-0015). Both comparisons favour MEO-direct; only the second one is
    // the rate the network actually pays.
    let leo_handover = orbital_period(&planet, LEO) / 12.0;
    let meo_handover = orbital_period(&planet, MEO) / 4.0;
    println!(
        "    best-case pass {:.1} h vs {:.1} min, and a handover every {:.1} h
             instead of every {:.1} min; Doppler window ±{:.0} kHz vs ±460 kHz;",
        max_pass_duration(&planet, MEO, e) / 3_600.0,
        max_pass_duration(&planet, LEO, e) / 60.0,
        meo_handover / 3_600.0,
        leo_handover / 60.0,
        doppler_shift(range_rate(&planet, MEO, edge_meo), KA) / 1e3
    );
    println!(
        "    no feeder hop; beacon raster over {:.0} spots ≈ {:.0} s.\n",
        spots_per_footprint(&planet, MEO, e, spot_meo),
        spots_per_footprint(&planet, MEO, e, spot_meo) * 0.010
    );

    println!(
        "Verdict (ADR-0012): the terminal pays for MEO-direct — a {:.1} m dish\n\
         or {:.0}x the power on ten thousand unattended boxes — and spot reuse\n\
         coarsens {:.0}x. LEO access is retained; MEO-direct remains the\n\
         economics post's contingency case.",
        0.5 * dish_factor,
        10f64.powf(delta_db / 10.0),
        (spot_meo / spot_leo).powi(2)
    );
}
