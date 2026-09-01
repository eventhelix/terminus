//! Cold-start acquisition arithmetic: how long a just-landed terminal waits
//! for a beacon when satellites raster their footprint spot by spot — and
//! why the terminal's own receive side never rasters back (ADR-0027): each
//! element of its panel hears the whole visible sky at once, and the phase
//! tilt of the arriving wavefront across the face measures the beacon's
//! direction without ever forming a search beam.

use crate::coverage::footprint_radius;
use crate::CentralBody;

/// Number of spot-beam positions needed to tile one satellite footprint:
/// the ratio of the footprint's spherical-cap area to a spot's cap area.
///
/// Deliberately conservative: every position is priced at the given (nadir)
/// spot size, while a leaning beam's real spot stretches up to 5.3× radially
/// (`beams::spot_half_extent` — farther, flatter, fatter). A raster stepped
/// at nadir pitch therefore over-tiles the rim; the count is an upper bound
/// and the raster period built from it is a ceiling, bought in exchange for
/// a uniform pitch and generous overlap where the link is weakest.
pub fn spots_per_footprint(
    body: &CentralBody,
    altitude: f64,
    min_elevation: f64,
    spot_radius: f64,
) -> f64 {
    let lambda = footprint_radius(body, altitude, min_elevation) / body.radius;
    let s = spot_radius / body.radius;
    (1.0 - lambda.cos()) / (1.0 - s.cos())
}

/// Time (s) for one full beacon raster over `spots` positions at `dwell`
/// seconds each — the worst-case wait for a terminal that knows nothing.
pub fn beacon_raster_period(spots: f64, dwell: f64) -> f64 {
    spots * dwell
}

/// Number of receive-beam positions needed to tile the sky a terminal must
/// watch — the spherical cap above `min_elevation` (rad) — with beams of
/// `beamwidth` (rad): the same cap-area ratio as [`spots_per_footprint`],
/// pointed up instead of down. This is the price a terminal would pay to
/// search for the beacon with a directional beam; ADR-0027 exists because
/// that price, either way you pay it, is worse than not searching at all.
pub fn sky_positions(min_elevation: f64, beamwidth: f64) -> f64 {
    let cap = std::f64::consts::FRAC_PI_2 - min_elevation;
    (1.0 - cap.cos()) / (1.0 - (beamwidth / 2.0).cos())
}

/// Root-mean-square direction-of-arrival error — same angular unit as
/// `beamwidth` — when an aperture reads a wavefront's direction from the
/// phase tilt across its own face: the classic monopulse/interferometer
/// rule of thumb `θ_bw / (k·√(2·SNR))` with slope constant k = 1.6.
///
/// The physics, in one breath: a plane wave arriving off boresight reaches
/// one edge of the aperture a fraction of a wavelength before the other,
/// so every element sees the same signal at a slightly different carrier
/// phase, and fitting the tilt of that phase plane *is* the direction
/// measurement. No beam is formed and nothing is scanned; accuracy is set
/// by the aperture size (through `beamwidth`) and by how cleanly each
/// phase is read (through `snr`, a linear power ratio).
pub fn doa_rms(beamwidth: f64, snr: f64) -> f64 {
    beamwidth / (1.6 * (2.0 * snr).sqrt())
}

/// Fraction of a satellite's footprint that lies within `band_half_angle`
/// (rad) of the habitable band's central great circle, for a sub-satellite
/// point `track_offset` (rad) off that circle.
///
/// The raster region is this intersection — one generic rule for every
/// active satellite: a duty-ring satellite riding the terminator keeps ~95%
/// of its footprint, while a hole-filler lit from a displaced ring keeps
/// only the sliver that clips the band. Trimming only ever shortens rounds,
/// so the full-footprint raster period stays the worst case.
pub fn band_raster_fraction(
    body: &CentralBody,
    altitude: f64,
    min_elevation: f64,
    band_half_angle: f64,
    track_offset: f64,
) -> f64 {
    let lambda = footprint_radius(body, altitude, min_elevation) / body.radius;
    let (n_rho, n_psi) = (256, 512);
    let (mut inside, mut total) = (0.0, 0.0);
    for i in 0..n_rho {
        let rho: f64 = lambda * (i as f64 + 0.5) / n_rho as f64;
        let weight = rho.sin();
        for j in 0..n_psi {
            let psi = 2.0 * std::f64::consts::PI * (j as f64 + 0.5) / n_psi as f64;
            // Spherical law of cosines: the latitude (off the band's central
            // great circle) of the cap point at polar coords (rho, psi).
            let sin_lat =
                track_offset.sin() * rho.cos() + track_offset.cos() * rho.sin() * psi.cos();
            total += weight;
            if sin_lat.abs() <= band_half_angle.sin() {
                inside += weight;
            }
        }
    }
    inside / total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn about_17_000_spots_tile_a_footprint() {
        // 2,200 km shell, 25° min elevation (footprint radius 2,519 km),
        // 19.2 km spot radius.
        let p = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        let spots = spots_per_footprint(&p, 2_200e3, 25.0_f64.to_radians(), 1.92e4);
        assert_close(spots, 1.6996e4, 1e-3);
    }

    #[test]
    fn full_raster_takes_under_three_minutes() {
        assert_close(beacon_raster_period(1.6996e4, 0.010), 169.96, 1e-3);
    }

    #[test]
    fn six_hundred_receive_beam_positions_tile_the_visible_sky() {
        // A 0.5 m terminal panel throws a 5.0° beam at X (8.4 GHz); the sky
        // it must watch is the cap above the 25° elevation floor.
        use crate::radio::beamwidth_deg;
        let bw = beamwidth_deg(0.5, 8.4e9).to_radians();
        assert_close(sky_positions(25.0_f64.to_radians(), bw), 607.47, 1e-3);
    }

    #[test]
    fn an_unsynchronized_two_sided_search_blows_the_budget() {
        // If the terminal searched with a directional receive beam, each of
        // its 607 sky positions must be held for one full lantern round
        // before moving on — 2.2 hours against TER-REQ-008's 15 minutes.
        let round = beacon_raster_period(1.333e3, 0.010);
        let search = 607.47 * round;
        assert_close(search, 8_098.0, 1e-2);
        assert!(search / (15.0 * 60.0) > 8.9, "must be ~9x over the budget");
    }

    #[test]
    fn a_nested_receive_raster_is_worse_than_not_scanning() {
        // The alternative: sweep all 607 positions electronically inside
        // each 10 ms beacon dwell. The pointed beam buys array-over-element
        // gain, but splitting the dwell 607 ways costs more integration
        // time than the gain repays — scanning nets ~-2.2 dB.
        use crate::radio::{beamwidth_deg, dish_gain_dbi};
        let bw = beamwidth_deg(0.5, 8.4e9).to_radians();
        let n = sky_positions(25.0_f64.to_radians(), bw);
        let element_gain_dbi = 5.0;
        let net = (dish_gain_dbi(0.5, 8.4e9, 0.6) - element_gain_dbi) - 10.0 * n.log10();
        assert_close(net, -2.182, 1e-2);
        assert!(net < 0.0);
    }

    #[test]
    fn beacon_closes_at_bare_element_gain_on_the_worst_slant() {
        // Wide-listen budget, worst case twice over: edge slant (3,642 km)
        // and the element's own pattern leaned 65° off boresight. 10 W
        // behind the satellite's 0.7 m X aperture, 50 kHz beacon channel,
        // 290 K system temperature.
        use crate::coverage::edge_slant_range;
        use crate::radio::{dish_gain_dbi, fspl_db, scan_loss_db, thermal_noise_dbw};
        let p = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        let slant = edge_slant_range(&p, 2_200e3, 25.0_f64.to_radians());
        let eirp = 10.0 * 10.0_f64.log10() + dish_gain_dbi(0.7, 8.4e9, 0.6);
        let element = 5.0 + scan_loss_db(65.0_f64.to_radians(), 1.2);
        let snr =
            eirp - fspl_db(slant, 8.4e9) + element - thermal_noise_dbw(290.0, 50e3);
        assert_close(snr, 18.91, 1e-2);
    }

    #[test]
    fn the_wavefront_compass_reads_finer_than_the_ka_pencil_needs() {
        // At the worst slant the X receive pattern broadens to 11.8° and
        // the SNR is 18.9 dB; the phase-tilt fit still fixes the beacon's
        // direction to 0.59° rms — 5.6x finer than the 3.31° Ka pencil
        // (itself broadened at that scan angle) the box must seed with it.
        use crate::radio::scanned_beamwidth_deg;
        let theta = 65.0_f64.to_radians();
        let bw = scanned_beamwidth_deg(0.5, 8.4e9, theta);
        let doa = doa_rms(bw, 10.0_f64.powf(18.91 / 10.0));
        assert_close(doa, 0.592, 1e-2);
        assert!(doa < scanned_beamwidth_deg(0.5, 30e9, theta) / 2.0);
    }

    #[test]
    fn doa_improves_as_the_square_root_of_snr() {
        assert_close(doa_rms(1.0, 400.0), 0.0221, 1e-2);
        let four_times = doa_rms(5.0, 100.0) / doa_rms(5.0, 400.0);
        assert_close(four_times, 2.0, 1e-9);
    }

    #[test]
    fn raster_region_is_the_footprint_band_intersection() {
        // One generic rule: footprint ∩ ±20° band. On the terminator the
        // 22.65°-radius footprint keeps ~95%; the share falls monotonically
        // as the sub-satellite point moves off the band's central circle,
        // and vanishes once the footprint no longer reaches the band.
        let p = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        let eps = 25.0_f64.to_radians();
        let band = 20.0_f64.to_radians();
        let frac = |t: f64| band_raster_fraction(&p, 2_200e3, eps, band, t.to_radians());
        assert_close(frac(0.0), 0.95, 2e-2);
        let mut last = 1.0;
        for t in [0.0, 10.0, 20.0, 30.0, 40.0] {
            let f = frac(t);
            assert!(f < last, "not monotone at {t}");
            last = f;
        }
        assert_close(frac(20.0), 0.51, 5e-2);
        assert!(frac(45.0) < 1e-9);
    }
}
