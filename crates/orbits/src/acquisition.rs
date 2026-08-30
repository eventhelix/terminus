//! Cold-start acquisition arithmetic: how long a just-landed terminal waits
//! for a beacon when satellites raster their footprint spot by spot.

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
