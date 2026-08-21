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
}
