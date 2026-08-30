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
}
