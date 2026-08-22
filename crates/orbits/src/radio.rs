//! First-order radio-link arithmetic: free-space path loss, parabolic-dish
//! gain, and beamwidth, for band trade studies.

use crate::placement::SPEED_OF_LIGHT;

/// Free-space path loss in dB over `distance` (m) at `frequency` (Hz):
/// 20·log₁₀(4πdf/c).
pub fn fspl_db(distance: f64, frequency: f64) -> f64 {
    20.0 * (4.0 * std::f64::consts::PI * distance * frequency / SPEED_OF_LIGHT).log10()
}

/// Gain (dBi) of a parabolic dish of `diameter` (m) at `frequency` (Hz)
/// with aperture `efficiency` (0..1): 10·log₁₀(η·(πD/λ)²).
pub fn dish_gain_dbi(diameter: f64, frequency: f64, efficiency: f64) -> f64 {
    let lambda = SPEED_OF_LIGHT / frequency;
    let x = std::f64::consts::PI * diameter / lambda;
    10.0 * (efficiency * x * x).log10()
}

/// Half-power beamwidth (degrees) of a dish of `diameter` (m) at
/// `frequency` (Hz), using the ~70·λ/D rule of thumb.
pub fn beamwidth_deg(diameter: f64, frequency: f64) -> f64 {
    70.0 * (SPEED_OF_LIGHT / frequency) / diameter
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
    fn fspl_at_access_edge_slant_ka_band() {
        // 3,642 km edge slant (2,200 km shell, 25° elevation) at 30 GHz.
        assert_close(fspl_db(3.6418e6, 30e9), 193.22, 1e-3);
    }

    #[test]
    fn fspl_doubles_by_six_db_per_octave() {
        let delta = fspl_db(3.6418e6, 30e9) - fspl_db(3.6418e6, 15e9);
        assert_close(delta, 6.0206, 1e-3);
    }

    #[test]
    fn half_meter_dish_at_ka_band() {
        assert_close(dish_gain_dbi(0.5, 30e9, 0.6), 41.71, 1e-3);
        assert_close(beamwidth_deg(0.5, 30e9), 1.399, 1e-3);
    }

    #[test]
    fn half_meter_dish_at_l_band_floods() {
        assert_close(beamwidth_deg(0.5, 1.6e9), 26.23, 1e-3);
    }

    #[test]
    fn meo_direct_access_pays_16_db() {
        // Option C trade (ADR-0012): serving users straight from the MEO
        // shell lengthens the worst-case slant from 3,642 km to 23,039 km —
        // +16.0 dB of path loss at any frequency.
        use crate::coverage::edge_slant_range;
        use crate::CentralBody;
        let p = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        let e = 25.0_f64.to_radians();
        let delta = fspl_db(edge_slant_range(&p, 20_000e3, e), 30e9)
            - fspl_db(edge_slant_range(&p, 2_200e3, e), 30e9);
        assert_close(delta, 16.02, 1e-3);
    }
}
