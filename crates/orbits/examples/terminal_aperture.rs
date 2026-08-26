//! What a fixed, electronically steered ground terminal pays for having no
//! moving parts: the scan loss and beam broadening of a face-up planar
//! aperture across the served elevation range, and the aperture growth that
//! buys the worst case back.
//!
//! Run: cargo run -p terminus-orbits --example terminal_aperture

use terminus_orbits::radio::{
    beamwidth_deg, dish_gain_dbi, fspl_db, planar_array_gain_dbi, scan_loss_db,
    scanned_beamwidth_deg,
};

const PANEL_M: f64 = 0.5;
const EFFICIENCY: f64 = 0.6;
const KA: f64 = 30e9;
const L_BAND: f64 = 1.6e9;
const EDGE_SLANT: f64 = 3.6418e6;
/// Minimum served elevation (ADR-0003 coverage rule), in degrees.
const MIN_ELEVATION_DEG: f64 = 25.0;
/// Element-pattern rolloff exponent; 1.0 is the ideal projected-aperture law.
const ROLLOFF: f64 = 1.2;

fn main() {
    println!(
        "Terminal aperture: {PANEL_M} m planar array, {EFFICIENCY} efficiency, \
         face-up, Ka {} GHz",
        KA / 1e9
    );
    println!(
        "Boresight (zenith) gain: {:.2} dBi, beamwidth {:.2}°\n",
        dish_gain_dbi(PANEL_M, KA, EFFICIENCY),
        beamwidth_deg(PANEL_M, KA)
    );

    println!(
        "{:>10} {:>7} {:>12} {:>12} {:>11} {:>11}",
        "elev (deg)", "scan θ", "cosθ loss", "×1.2 loss", "gain (dBi)", "beam (deg)"
    );
    for elev_deg in [90.0, 75.0, 60.0, 45.0, 30.0, MIN_ELEVATION_DEG] {
        let theta = (90.0 - elev_deg).to_radians();
        println!(
            "{:>10.0} {:>6.0}° {:>+12.2} {:>+12.2} {:>11.2} {:>11.2}",
            elev_deg,
            theta.to_degrees(),
            scan_loss_db(theta, 1.0),
            scan_loss_db(theta, ROLLOFF),
            planar_array_gain_dbi(PANEL_M, KA, EFFICIENCY, theta, ROLLOFF),
            scanned_beamwidth_deg(PANEL_M, KA, theta)
        );
    }

    let worst = (90.0 - MIN_ELEVATION_DEG).to_radians();
    let worst_loss = scan_loss_db(worst, ROLLOFF);
    let grown = PANEL_M * 10f64.powf(-worst_loss / 20.0);
    println!(
        "\nWorst case at the {MIN_ELEVATION_DEG:.0}° coverage floor: {:.2} dB.\n\
         Holding the boresight gain there costs {:.2} m of panel (a {:.0}% wider\n\
         face, {:.0}% more area) — or the same {:.2} dB taken from the link margin.",
        worst_loss,
        grown,
        100.0 * (grown / PANEL_M - 1.0),
        100.0 * ((grown / PANEL_M).powi(2) - 1.0),
        -worst_loss
    );

    // Scan loss has no frequency term, so it cannot disturb the band trade.
    let figure = |f: f64, th: f64| {
        2.0 * planar_array_gain_dbi(PANEL_M, f, EFFICIENCY, th, ROLLOFF) - fspl_db(EDGE_SLANT, f)
    };
    println!(
        "\nKa over L-band, terminal at boresight: {:+.1} dB\n\
         Ka over L-band, terminal at the {:.0}° coverage floor: {:+.1} dB\n\
         Scan loss carries no frequency term, so ADR-0005's band trade stands\n\
         unchanged; only the absolute margin moves.",
        figure(KA, 0.0) - figure(L_BAND, 0.0),
        MIN_ELEVATION_DEG,
        figure(KA, worst) - figure(L_BAND, worst)
    );
}
