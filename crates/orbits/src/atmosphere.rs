// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Sea-level atmospheric loss for band trade studies: clear-air gaseous
//! absorption and rain attenuation, as empirical fits from the ITU-R
//! propagation recommendations.
//!
//! Gaseous absorption is the simplified model of ITU-R P.676-3 Annex 2
//! (valid 1–350 GHz, within ~±15% of the line-by-line calculation away
//! from line centers): the 22.235 GHz water-vapor rotational line, the
//! 50–70 GHz oxygen complex — magnetic-dipole spin-flip transitions,
//! merged at sea-level pressure into a single wall — and the 118.75 GHz
//! oxygen line. Rain is the ITU-R P.838-3 power law `γ = k·R^α`, with
//! `k` and `α` from that recommendation's analytic coefficient fits,
//! combined here for circular polarization.
//!
//! Both models describe a nitrogen/oxygen atmosphere with water vapor,
//! parameterized by surface pressure, temperature, humidity and rain
//! rate; nothing planet-specific is hard-coded.
//!
//! Sources:
//! - ITU-R P.676-3, *Attenuation by atmospheric gases*, Annex 2:
//!   <https://www.itu.int/dms_pubrec/itu-r/rec/p/R-REC-P.676-3-199708-S!!PDF-E.pdf>
//! - ITU-R P.838-3, *Specific attenuation model for rain*:
//!   <https://www.itu.int/dms_pubrec/itu-r/rec/p/R-REC-P.838-3-200503-I!!PDF-E.pdf>

/// Surface air state for the gaseous-absorption model. SI units:
/// pressure in Pa, temperature in K, water-vapor density in kg/m³.
///
/// [`SurfaceAir::default`] is the ITU reference atmosphere the P.676
/// fits were made at: 1 013 hPa, 15 °C, 7.5 g/m³ of water vapor.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceAir {
    pub pressure: f64,
    pub temperature: f64,
    pub water_vapor_density: f64,
}

impl Default for SurfaceAir {
    fn default() -> Self {
        Self {
            pressure: 101_300.0,
            temperature: 288.15,
            water_vapor_density: 7.5e-3,
        }
    }
}

impl SurfaceAir {
    /// P.676's normalized pressure `r_p = p/1013` (p in hPa).
    fn r_p(&self) -> f64 {
        self.pressure / 101_300.0
    }

    /// P.676's normalized temperature `r_t = 288/(273 + t)` (t in °C).
    fn r_t(&self) -> f64 {
        288.0 / (self.temperature - 0.15)
    }

    /// Dry-air (oxygen) specific attenuation, dB/km, at `frequency` (Hz).
    /// ITU-R P.676-3 Annex 2 eq. (22a)–(22c); valid 1–350 GHz.
    pub fn dry_air_db_per_km(&self, frequency: f64) -> f64 {
        let f = frequency / 1e9;
        let (rp, rt) = (self.r_p(), self.r_t());
        if f <= 57.0 {
            dry_air_below_57(f, rp, rt)
        } else if f >= 63.0 {
            dry_air_above_63(f, rp, rt)
        } else {
            // Quadratic bridge through the merged 60 GHz oxygen complex:
            // exact at 57 and 63 GHz, peaking near 15 dB/km at 60.
            (f - 60.0) * (f - 63.0) / 18.0 * dry_air_below_57(57.0, rp, rt)
                - 1.66 * rp * rp * rt.powf(8.5) * (f - 57.0) * (f - 63.0)
                + (f - 57.0) * (f - 60.0) / 18.0 * dry_air_above_63(63.0, rp, rt)
        }
    }

    /// Water-vapor specific attenuation, dB/km, at `frequency` (Hz).
    /// ITU-R P.676-3 Annex 2 eq. (23); valid 1–350 GHz.
    pub fn water_vapor_db_per_km(&self, frequency: f64) -> f64 {
        let f = frequency / 1e9;
        let (rp, rt) = (self.r_p(), self.r_t());
        let rho = self.water_vapor_density * 1e3; // g/m³, the fit's unit
        let sum = 3.27e-2 * rt
            + 1.67e-3 * rho * rt.powi(7) / rp
            + 7.7e-4 * f.sqrt()
            + 3.79 / ((f - 22.235).powi(2) + 9.81 * rp * rp * rt)
            + 11.73 * rt / ((f - 183.31).powi(2) + 11.85 * rp * rp * rt)
            + 4.01 * rt / ((f - 325.153).powi(2) + 10.44 * rp * rp * rt);
        sum * f * f * rho * rp * rt * 1e-4
    }

    /// Total clear-air specific attenuation, dB/km, at `frequency` (Hz):
    /// dry air plus water vapor.
    pub fn gaseous_db_per_km(&self, frequency: f64) -> f64 {
        self.dry_air_db_per_km(frequency) + self.water_vapor_db_per_km(frequency)
    }
}

/// P.676-3 eq. (22a), f in GHz, f ≤ 57.
fn dry_air_below_57(f: f64, rp: f64, rt: f64) -> f64 {
    (7.27 * rt / (f * f + 0.351 * rp * rp * rt * rt)
        + 7.5 / ((f - 57.0).powi(2) + 2.44 * rp * rp * rt.powi(5)))
        * f
        * f
        * rp
        * rp
        * rt
        * rt
        * 1e-3
}

/// P.676-3 eq. (22b), f in GHz, 63 ≤ f ≤ 350.
fn dry_air_above_63(f: f64, rp: f64, rt: f64) -> f64 {
    (2e-4 * rt.powf(1.5) * (1.0 - 1.2e-5 * f.powf(1.5))
        + 4.0 / ((f - 63.0).powi(2) + 1.5 * rp * rp * rt.powi(5))
        + 0.28 * rt * rt / ((f - 118.75).powi(2) + 2.84 * rp * rp * rt * rt))
        * f
        * f
        * rp
        * rp
        * rt
        * rt
        * 1e-3
}

/// One polarization's power-law coefficient fit from ITU-R P.838-3
/// eq. (2)/(3): Gaussian terms in log₁₀ f plus a log-linear tail.
struct RainFit {
    terms: &'static [(f64, f64, f64)],
    m: f64,
    c: f64,
}

impl RainFit {
    fn eval(&self, log_f: f64) -> f64 {
        let sum: f64 = self
            .terms
            .iter()
            .map(|&(a, b, c)| a * (-((log_f - b) / c).powi(2)).exp())
            .sum();
        sum + self.m * log_f + self.c
    }
}

/// P.838-3 Table 1: log₁₀ k, horizontal polarization.
const K_H: RainFit = RainFit {
    terms: &[
        (-5.33980, -0.10008, 1.13098),
        (-0.35351, 1.26970, 0.45400),
        (-0.23789, 0.86036, 0.15354),
        (-0.94158, 0.64552, 0.16817),
    ],
    m: -0.18961,
    c: 0.71147,
};

/// P.838-3 Table 2: log₁₀ k, vertical polarization.
const K_V: RainFit = RainFit {
    terms: &[
        (-3.80595, 0.56934, 0.81061),
        (-3.44965, -0.22911, 0.51059),
        (-0.39902, 0.73042, 0.11899),
        (0.50167, 1.07319, 0.27195),
    ],
    m: -0.16398,
    c: 0.63297,
};

/// P.838-3 Table 3: α, horizontal polarization.
const ALPHA_H: RainFit = RainFit {
    terms: &[
        (-0.14318, 1.82442, -0.55187),
        (0.29591, 0.77564, 0.19822),
        (0.32177, 0.63773, 0.13164),
        (-5.37610, -0.96230, 1.47828),
        (16.1721, -3.29980, 3.43990),
    ],
    m: 0.67849,
    c: -1.95537,
};

/// P.838-3 Table 4: α, vertical polarization.
const ALPHA_V: RainFit = RainFit {
    terms: &[
        (-0.07771, 2.33840, -0.76284),
        (0.56727, 0.95545, 0.54039),
        (-0.20238, 1.14520, 0.26809),
        (-48.2991, 0.791669, 0.116226),
        (48.5833, 0.791459, 0.116479),
    ],
    m: -0.053739,
    c: 0.83433,
};

/// The `(k, α)` of `γ = k·R^α` at `frequency` (Hz) for circular
/// polarization: P.838-3 eq. (4)/(5) with tilt τ = 45°, where the
/// elevation dependence drops out and the coefficients reduce to
/// `k = (k_H + k_V)/2`, `α = (k_H·α_H + k_V·α_V)/2k`.
pub fn rain_coefficients(frequency: f64) -> (f64, f64) {
    let log_f = (frequency / 1e9).log10();
    let kh = 10f64.powf(K_H.eval(log_f));
    let kv = 10f64.powf(K_V.eval(log_f));
    let (ah, av) = (ALPHA_H.eval(log_f), ALPHA_V.eval(log_f));
    let k = (kh + kv) / 2.0;
    (k, (kh * ah + kv * av) / (2.0 * k))
}

/// Rain specific attenuation, dB/km, at `frequency` (Hz) for a rain
/// rate in mm/h (the power law's native unit — ~5 is steady rain, ~25 a
/// heavy storm, ~50 a downpour): ITU-R P.838-3, circular polarization.
pub fn rain_db_per_km(frequency: f64, rain_rate_mm_per_h: f64) -> f64 {
    let (k, alpha) = rain_coefficients(frequency);
    k * rain_rate_mm_per_h.powf(alpha)
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
    fn rain_fits_reproduce_the_recommendations_own_table() {
        // P.838-3 Table 5 rows at 8 and 30 GHz (kH, αH, kV, αV).
        let log8 = 8f64.log10();
        let log30 = 30f64.log10();
        assert_close(10f64.powf(K_H.eval(log8)), 0.004115, 2e-3);
        assert_close(ALPHA_H.eval(log8), 1.3905, 2e-3);
        assert_close(10f64.powf(K_V.eval(log8)), 0.003450, 2e-3);
        assert_close(ALPHA_V.eval(log8), 1.3797, 2e-3);
        assert_close(10f64.powf(K_H.eval(log30)), 0.2403, 2e-3);
        assert_close(ALPHA_H.eval(log30), 0.9485, 2e-3);
        assert_close(10f64.powf(K_V.eval(log30)), 0.2291, 2e-3);
        assert_close(ALPHA_V.eval(log30), 0.9129, 2e-3);
    }

    #[test]
    fn heavy_rain_at_ka_band_is_about_five_db_per_km() {
        // 25 mm/h at 30 GHz, circular pol: k=(0.2403+0.2291)/2, and the
        // blended α, give ~4.7 dB/km.
        assert_close(rain_db_per_km(30e9, 25.0), 4.70, 0.02);
    }

    #[test]
    fn x_band_shrugs_at_the_storm_that_hurts_ka() {
        let ka = rain_db_per_km(30e9, 25.0);
        let x = rain_db_per_km(8.4e9, 25.0);
        assert!(x < 0.5, "X-band in heavy rain: {x} dB/km");
        assert!(ka / x > 10.0, "Ka/X rain ratio: {}", ka / x);
    }

    #[test]
    fn water_vapor_peaks_at_its_rotational_line() {
        let air = SurfaceAir::default();
        let at_line = air.water_vapor_db_per_km(22.235e9);
        assert!(at_line > air.water_vapor_db_per_km(19e9));
        assert!(at_line > air.water_vapor_db_per_km(26e9));
        // P.676-3 Fig. 5 shows ~0.2 dB/km at the 22 GHz peak.
        assert_close(at_line, 0.16, 0.25);
    }

    #[test]
    fn the_oxygen_wall_peaks_near_fifteen_db_per_km() {
        let air = SurfaceAir::default();
        let peak = air.dry_air_db_per_km(60e9);
        assert!((14.0..16.0).contains(&peak), "60 GHz wall: {peak} dB/km");
    }

    #[test]
    fn the_oxygen_bridge_is_continuous_at_its_seams() {
        let air = SurfaceAir::default();
        assert_close(
            air.dry_air_db_per_km(57e9),
            air.dry_air_db_per_km(56.999e9),
            1e-3,
        );
        assert_close(
            air.dry_air_db_per_km(63e9),
            air.dry_air_db_per_km(63.001e9),
            1e-3,
        );
    }

    #[test]
    fn clear_air_at_the_plan_bands_is_a_rounding_error() {
        // The clear-sky columns the band trade quotes: ~0.09 dB/km at
        // Ka, ~0.01 at X — negligible against 190+ dB of path loss.
        let air = SurfaceAir::default();
        assert_close(air.gaseous_db_per_km(30e9), 0.087, 0.05);
        assert_close(air.gaseous_db_per_km(8.4e9), 0.011, 0.10);
    }
}
