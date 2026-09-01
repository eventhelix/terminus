//! First-order radio-link arithmetic: free-space path loss, aperture gain
//! and beamwidth, and the scan loss a planar (electronically steered)
//! aperture pays away from boresight, for band and terminal trade studies.
//!
//! The gain law `10·log₁₀(η·(πD/λ)²)` is aperture-area physics: it holds for
//! a parabolic reflector and for a planar array of the same effective area
//! alike. The two differ in how they point — a reflector is aimed
//! mechanically and always looks along its own axis, while a fixed planar
//! array steers electronically and loses the cosine of the steering angle
//! off its face. [`scan_loss_db`] is that difference.

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

/// Thermal noise power (dBW) collected in `bandwidth` (Hz) at system
/// temperature `temperature` (K): 10·log₁₀(k·T·B), k the Boltzmann
/// constant. The narrower the channel, the quieter the floor — which is
/// why a slow, narrow beacon can be heard by an antenna far too humble to
/// carry traffic.
pub fn thermal_noise_dbw(temperature: f64, bandwidth: f64) -> f64 {
    const BOLTZMANN: f64 = 1.380_649e-23;
    10.0 * (BOLTZMANN * temperature * bandwidth).log10()
}

/// Scan loss (dB, ≤ 0) of a planar aperture steered `scan_angle` (rad) away
/// from boresight — the normal to its own face: `10·rolloff·log₁₀(cos θ)`.
///
/// `rolloff` = 1.0 is the ideal projected-aperture law: a face tilted by θ
/// presents only `cos θ` of its area to the far end, so it both radiates and
/// collects that much less. Real arrays fall off faster, because each
/// element has its own pattern that dims off-axis on top of the projection;
/// `rolloff` in 1.2..1.5 is the usual fitted range. Nothing here is
/// frequency-dependent, so scan loss cancels out of any same-geometry
/// comparison between bands.
///
/// Returns [`f64::NEG_INFINITY`] at or beyond 90°, where a planar face can
/// no longer see the far end at all.
pub fn scan_loss_db(scan_angle: f64, rolloff: f64) -> f64 {
    if scan_angle.abs() >= std::f64::consts::FRAC_PI_2 {
        return f64::NEG_INFINITY;
    }
    10.0 * rolloff * scan_angle.cos().log10()
}

/// Gain (dBi) of a planar aperture of effective `diameter` (m) at
/// `frequency` (Hz) and aperture `efficiency` (0..1), steered `scan_angle`
/// (rad) off boresight with the given `rolloff`: boresight aperture gain
/// plus [`scan_loss_db`].
pub fn planar_array_gain_dbi(
    diameter: f64,
    frequency: f64,
    efficiency: f64,
    scan_angle: f64,
    rolloff: f64,
) -> f64 {
    dish_gain_dbi(diameter, frequency, efficiency) + scan_loss_db(scan_angle, rolloff)
}

/// Half-power beamwidth (degrees) of a planar aperture of `diameter` (m) at
/// `frequency` (Hz) steered `scan_angle` (rad) off boresight: the boresight
/// beamwidth broadened by 1/cos θ, since the aperture foreshortens in the
/// scan plane. Returns [`f64::INFINITY`] at or beyond 90°.
pub fn scanned_beamwidth_deg(diameter: f64, frequency: f64, scan_angle: f64) -> f64 {
    if scan_angle.abs() >= std::f64::consts::FRAC_PI_2 {
        return f64::INFINITY;
    }
    beamwidth_deg(diameter, frequency) / scan_angle.cos()
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
    fn scan_loss_is_zero_at_boresight() {
        assert!(scan_loss_db(0.0, 1.0).abs() < 1e-12);
        assert!(scan_loss_db(0.0, 1.4).abs() < 1e-12);
    }

    #[test]
    fn scan_loss_at_the_minimum_elevation_of_a_flat_panel() {
        // A panel lying face-up serving a satellite 25° above the horizon
        // steers 65° off boresight.
        let theta = 65.0_f64.to_radians();
        assert_close(scan_loss_db(theta, 1.0), -3.742, 1e-3);
        assert_close(scan_loss_db(theta, 1.2), -4.490, 1e-3);
    }

    #[test]
    fn scan_loss_is_frequency_independent_so_band_trades_are_unaffected() {
        // The Ka-vs-L link advantage is identical at boresight and at the
        // 65° worst-case scan: scan loss is common to both bands.
        let theta = 65.0_f64.to_radians();
        let figure = |f: f64, th: f64| {
            2.0 * planar_array_gain_dbi(0.5, f, 0.6, th, 1.2) - fspl_db(3.6418e6, f)
        };
        let at_boresight = figure(30e9, 0.0) - figure(1.6e9, 0.0);
        let at_scan = figure(30e9, theta) - figure(1.6e9, theta);
        assert_close(at_scan, at_boresight, 1e-9);
    }

    #[test]
    fn planar_gain_matches_dish_gain_at_boresight() {
        assert_close(
            planar_array_gain_dbi(0.5, 30e9, 0.6, 0.0, 1.2),
            dish_gain_dbi(0.5, 30e9, 0.6),
            1e-12,
        );
    }

    #[test]
    fn beam_broadens_by_one_over_cosine_when_scanned() {
        let theta = 65.0_f64.to_radians();
        // 1.399° at boresight broadens to 3.31° at the horizon-most scan.
        assert_close(scanned_beamwidth_deg(0.5, 30e9, theta), 3.310, 1e-3);
    }

    #[test]
    fn a_planar_face_sees_nothing_past_ninety_degrees() {
        let theta = 90.0_f64.to_radians();
        assert!(scan_loss_db(theta, 1.2).is_infinite());
        assert!(scanned_beamwidth_deg(0.5, 30e9, theta).is_infinite());
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
