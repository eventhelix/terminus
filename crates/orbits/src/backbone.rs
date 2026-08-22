//! Inter-satellite backbone geometry: intra-ring neighbor links, LEO→MEO
//! feeder-link visibility, and the worst-case Doppler the feeder links must
//! precompensate.

use crate::circular::orbital_period;
use crate::CentralBody;

/// Distance (m) between adjacent satellites evenly spaced on one circular
/// ring: the chord 2r·sin(π/n). Constant for all time — satellites sharing
/// a circular orbit do not move relative to each other.
pub fn intra_plane_neighbor_range(body: &CentralBody, altitude: f64, sats_per_plane: usize) -> f64 {
    let r = body.radius + altitude;
    2.0 * r * (std::f64::consts::PI / sats_per_plane as f64).sin()
}

/// Largest central angle (rad) at which satellites on two shells still see
/// each other over the planet's limb: acos(R/r₁) + acos(R/r₂).
pub fn max_shell_separation(body: &CentralBody, alt1: f64, alt2: f64) -> f64 {
    let a1 = (body.radius / (body.radius + alt1)).acos();
    let a2 = (body.radius / (body.radius + alt2)).acos();
    a1 + a2
}

/// Fraction of shell 2 visible from a satellite on shell 1:
/// (1 − cos ψ_max)/2.
pub fn shell_visible_fraction(body: &CentralBody, alt1: f64, alt2: f64) -> f64 {
    (1.0 - max_shell_separation(body, alt1, alt2).cos()) / 2.0
}

/// Worst-case range rate (m/s) between satellites on two circular shells,
/// scanning the coplanar separation angle over the mutually visible range:
/// ρ̇(Δ) = r₁r₂·(ω₁−ω₂)·sinΔ/ρ(Δ). Fully deterministic for known orbits.
pub fn max_shell_range_rate(body: &CentralBody, alt1: f64, alt2: f64) -> f64 {
    let r1 = body.radius + alt1;
    let r2 = body.radius + alt2;
    let omega_rel = 2.0 * std::f64::consts::PI / orbital_period(body, alt1)
        - 2.0 * std::f64::consts::PI / orbital_period(body, alt2);
    let psi_max = max_shell_separation(body, alt1, alt2);
    let mut max_rate: f64 = 0.0;
    let steps = 10_000;
    for i in 1..=steps {
        let delta = psi_max * i as f64 / steps as f64;
        let rho = (r1 * r1 + r2 * r2 - 2.0 * r1 * r2 * delta.cos()).sqrt();
        let rate = (r1 * r2 * omega_rel * delta.sin() / rho).abs();
        max_rate = max_rate.max(rate);
    }
    max_rate
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::polar_sat_position;

    fn reference_planet() -> CentralBody {
        CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0)
    }

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn ring_neighbors_sit_4437_km_apart() {
        let p = reference_planet();
        assert_close(intra_plane_neighbor_range(&p, 2_200e3, 12), 4.4367e6, 1e-3);
    }

    #[test]
    fn ring_neighbors_never_move_relative_to_each_other() {
        // Two satellites of the same ring, 30° apart in phase: their
        // separation is constant at every sampled time.
        let p = reference_planet();
        let sep = 2.0 * std::f64::consts::PI / 12.0;
        let dist = |t: f64| {
            let a = polar_sat_position(&p, 2_200e3, 0.3, 0.0, t);
            let b = polar_sat_position(&p, 2_200e3, 0.3, sep, t);
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
        };
        let d0 = dist(0.0);
        for t in [100.0, 3_600.0, 86_400.0, 500_000.0] {
            assert_close(dist(t), d0, 1e-9);
        }
    }

    #[test]
    fn most_of_the_meo_shell_is_visible_from_leo() {
        let p = reference_planet();
        assert_close(max_shell_separation(&p, 2_200e3, 20_000e3), 2.0601, 1e-3);
        assert_close(shell_visible_fraction(&p, 2_200e3, 20_000e3), 0.7349, 1e-3);
    }

    #[test]
    fn worst_feeder_doppler_is_about_5_5_km_per_s() {
        let p = reference_planet();
        assert_close(max_shell_range_rate(&p, 2_200e3, 20_000e3), 5.555e3, 2e-3);
    }
}
