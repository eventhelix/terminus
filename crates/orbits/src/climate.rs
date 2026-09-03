// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Radiative screening for a tidally locked planet: what the star delivers,
//! how warm that makes the ground, how deep the weather layer is, and whether
//! the night side is a cold trap that will strip the sky.
//!
//! This is deliberately the *thin* end of climate. Everything here is a
//! closed-form energy or gas-law statement that can be checked by hand:
//! instellation from luminosity and distance, radiative equilibrium at a
//! chosen heat-redistribution factor, the pressure scale height, and vapour
//! pressure over ice. Nothing here is a circulation model.
//!
//! **What this module deliberately does not compute: wind speed.** The
//! overturning velocity of a tidally locked atmosphere comes out of a general
//! circulation model, not out of any expression that fits on a page. The
//! series quotes 5-15 m/s at the surface from the GCM literature for planets
//! of this class and says so; a closed-form number here would be false
//! precision wearing a citation's clothes, which is worse than a citation.
//!
//! What it *is* enough for is the load-bearing claim. A locked planet whose
//! air does not circulate turns its night side into a cold trap: the surface
//! there falls to the radiative floor, far below the point at which the
//! atmosphere's own constituents freeze, so the sky migrates to the dark and
//! stays. That the band is habitable at all is a statement about circulation
//! defeating this, and [`night_side_collapses`] is the test it has to pass.
//!
//! Sources:
//! - Stefan-Boltzmann equilibrium with a redistribution factor is the
//!   standard exoplanet formulation (e.g. Selsis et al. 2007).
//! - CO2 vapour pressure over ice: the Mars-community fit
//!   `P = 1.382e12 · exp(-3182.48/T)` Pa (James et al. 1992).
//! - N2 sublimation: Antoine coefficients from the NIST Chemistry WebBook.

/// Stefan-Boltzmann constant, W m^-2 K^-4.
pub const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;

/// Molar gas constant, J mol^-1 K^-1.
pub const GAS_CONSTANT: f64 = 8.314_462_618;

/// Astronomical unit, m.
pub const AU: f64 = 1.495_978_707e11;

/// Solar bolometric luminosity, W.
pub const SOLAR_LUMINOSITY: f64 = 3.828e26;

/// Solar constant at 1 AU, W m^-2 — quoted for scale, not used in the math.
pub const SOLAR_CONSTANT: f64 = 1361.0;

/// Mean molar mass of Earth-like air, kg/mol.
pub const AIR_MOLAR_MASS: f64 = 0.028_964_6;

/// Earth's mean geothermal heat flux, W m^-2. The floor a surface radiates
/// against when no starlight and no atmosphere reach it.
pub const EARTH_INTERNAL_FLUX: f64 = 0.087;

/// Heat-redistribution factor for a surface under a star that never moves,
/// with no transport at all: the substellar point takes the full normal beam.
pub const NO_REDISTRIBUTION: f64 = 1.0;

/// Redistribution over the lit hemisphere only — the classic "day side keeps
/// everything" case, half the sphere radiating what the disc intercepts.
pub const DAYSIDE_REDISTRIBUTION: f64 = 2.0 / 3.0;

/// Redistribution over the whole sphere: the planet radiates from 4πR² what
/// it intercepts on πR². This is the coldest the ground can average.
pub const FULL_REDISTRIBUTION: f64 = 0.25;

/// Starlight arriving at the top of the atmosphere, W m^-2, for a star of
/// `luminosity` (W) at `distance` (m).
pub fn instellation(luminosity: f64, distance: f64) -> f64 {
    luminosity / (4.0 * std::f64::consts::PI * distance * distance)
}

/// Radiative equilibrium temperature, K, for an absorbed flux.
///
/// `redistribution` is the fraction of the intercepted beam each radiating
/// square meter has to shed — [`NO_REDISTRIBUTION`] at the substellar point,
/// [`FULL_REDISTRIBUTION`] for a planet that spreads its heat everywhere, and
/// [`DAYSIDE_REDISTRIBUTION`] between them. It is the whole climate model in
/// one number, which is why it is an argument and not a constant: this
/// function brackets the answer, it does not choose it.
pub fn equilibrium_temperature(instellation: f64, albedo: f64, redistribution: f64) -> f64 {
    (instellation * (1.0 - albedo) * redistribution / STEFAN_BOLTZMANN).powf(0.25)
}

/// The temperature a surface settles at when the only thing warming it is the
/// planet's own interior — the night side of a world that has lost both its
/// starlight and the air that used to carry heat there.
pub fn radiative_floor(internal_flux: f64) -> f64 {
    (internal_flux / STEFAN_BOLTZMANN).powf(0.25)
}

/// Pressure scale height, m: the height over which pressure falls by a factor
/// of e, `H = RT/(Mg)`.
///
/// This is the natural vertical ruler for the weather layer. An overturning
/// cell's return branch rides near the top of it, so "how many kilometers up
/// does the air cross to the night side" is answered here rather than
/// asserted.
pub fn scale_height(temperature: f64, molar_mass: f64, gravity: f64) -> f64 {
    GAS_CONSTANT * temperature / (molar_mass * gravity)
}

/// Frost point of CO2 over its own ice, K, at a partial pressure in Pa.
///
/// Inverts `P = 1.382e12 · exp(-3182.48/T)`. Below this temperature CO2
/// leaves the air and joins the ground.
pub fn co2_frost_point(partial_pressure: f64) -> f64 {
    3182.48 / (1.382e12 / partial_pressure).ln()
}

/// Condensation point of N2, K, at a partial pressure in Pa — the temperature
/// at which the bulk of an Earth-like atmosphere stops being atmosphere.
///
/// Antoine form `log10(P_bar) = A - B/(T + C)`, inverted.
pub fn n2_condensation_point(partial_pressure: f64) -> f64 {
    const A: f64 = 3.619_47;
    const B: f64 = 255.68;
    const C: f64 = -6.60;
    B / (A - (partial_pressure / 1e5).log10()) - C
}

/// Does the night side act as a cold trap?
///
/// The test is not subtle and is not meant to be: if the dark hemisphere sits
/// below the temperature at which the atmosphere's main constituent
/// condenses, then air that arrives there stops being air. Whatever the
/// circulation delivers, the trap keeps — and a planet that loses this race
/// loses its sky, and with it any habitable band.
pub fn night_side_collapses(night_temperature: f64, condensation_temperature: f64) -> bool {
    night_temperature < condensation_temperature
}

/// Greenhouse increment, K, implied by a surface sitting `surface` above the
/// bare radiative equilibrium `equilibrium` — the warming an atmosphere has
/// to supply for a stated surface temperature to be self-consistent.
///
/// Quoting this alongside Earth's own 33 K is what keeps a canonical band
/// temperature honest: it converts "we say the band is +8 C" into "we say the
/// band's air does this much work", which is a claim a reader can weigh.
pub fn greenhouse_increment(surface: f64, equilibrium: f64) -> f64 {
    surface - equilibrium
}

/// Celsius from kelvin, for reporting.
pub fn celsius(kelvin: f64) -> f64 {
    kelvin - 273.15
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proxima-like: 0.00155 solar bolometric luminosities.
    const STAR_LUMINOSITY: f64 = 0.001_55 * SOLAR_LUMINOSITY;
    const ORBITAL_DISTANCE: f64 = 0.0485 * AU;
    const SURFACE_GRAVITY: f64 = 9.82;
    const SURFACE_PRESSURE: f64 = 101_325.0;
    const ALBEDO: f64 = 0.3;

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn reference_planet_gets_about_two_thirds_of_earths_sunlight() {
        // The published figure for Proxima b is ~0.65 S_Earth. It is the one
        // number in this module that can be checked against a real
        // measurement, so it is the calibration for everything after it.
        let s = instellation(STAR_LUMINOSITY, ORBITAL_DISTANCE);
        assert_close(s / SOLAR_CONSTANT, 0.65, 0.02);
    }

    #[test]
    fn equilibrium_brackets_the_band() {
        // Two extremes of the same planet: all the heat kept where it lands,
        // and all of it shared out. Any real climate is inside this bracket,
        // and the canonical +8 C band has to be too or the world bible is
        // claiming something the star cannot pay for.
        let s = instellation(STAR_LUMINOSITY, ORBITAL_DISTANCE);
        let substellar = equilibrium_temperature(s, ALBEDO, NO_REDISTRIBUTION);
        let global = equilibrium_temperature(s, ALBEDO, FULL_REDISTRIBUTION);

        assert_close(substellar, 323.0, 0.02);
        assert_close(global, 228.0, 0.02);
        assert!(global < substellar);

        let band = 281.15; // +8 C, world bible
        assert!(
            band > global && band < substellar,
            "band {band} outside [{global}, {substellar}]"
        );
    }

    #[test]
    fn the_band_asks_its_air_for_about_what_earths_air_supplies() {
        // Earth runs a 33 K greenhouse. If the reference band needed several
        // times that, the canonical temperature would be the thing to move.
        let s = instellation(STAR_LUMINOSITY, ORBITAL_DISTANCE);
        let global = equilibrium_temperature(s, ALBEDO, FULL_REDISTRIBUTION);
        let increment = greenhouse_increment(281.15, global);
        assert!(
            (20.0..80.0).contains(&increment),
            "greenhouse increment {increment} K is not Earth-like"
        );
    }

    #[test]
    fn the_return_branch_rides_about_one_scale_height_up() {
        // The circulation plate draws the nightward branch at 7-10 km. That
        // has to be a consequence of the gas law, not a drawing decision.
        let h = scale_height(281.15, AIR_MOLAR_MASS, SURFACE_GRAVITY);
        assert_close(h, 8_200.0, 0.02);
        assert!((7_000.0..10_000.0).contains(&h), "scale height {h} m");
    }

    #[test]
    fn earth_scale_height_checks_the_formula() {
        // Same expression, a planet we have measured: ~8.4 km at 288 K.
        let h = scale_height(288.15, AIR_MOLAR_MASS, 9.80665);
        assert_close(h, 8_430.0, 0.01);
    }

    #[test]
    fn frost_points_match_their_reference_measurements() {
        // Mars: CO2 frosts at ~148 K under 610 Pa. At a full bar it is ~194 K.
        assert_close(co2_frost_point(610.0), 148.0, 0.01);
        assert_close(co2_frost_point(1e5), 194.0, 0.01);
        // N2 boils at 77.4 K under one atmosphere.
        assert_close(n2_condensation_point(101_325.0), 77.4, 0.01);
    }

    #[test]
    fn a_still_night_side_is_a_cold_trap() {
        // This is the stalled mode of the circulation plate. With nothing
        // carrying heat to the dark, the night side falls to the radiative
        // floor set by the planet's own interior — and that is far under the
        // point where an Earth-like atmosphere stops being a gas. The sky
        // migrates to the night side and freezes onto it.
        let floor = radiative_floor(EARTH_INTERNAL_FLUX);
        assert_close(floor, 35.2, 0.02);

        let n2 = n2_condensation_point(0.78 * SURFACE_PRESSURE);
        assert!(
            night_side_collapses(floor, n2),
            "floor {floor} K vs N2 point {n2} K"
        );
        // CO2 goes first, and by a wide margin.
        assert!(night_side_collapses(floor, co2_frost_point(40.0)));
    }

    #[test]
    fn a_circulating_night_side_keeps_its_air() {
        // The other half of the claim, and the reason anyone lives here. Give
        // the dark hemisphere the heat a working cell delivers — anywhere near
        // the fully-redistributed equilibrium — and it is nowhere near cold
        // enough to trap nitrogen.
        let s = instellation(STAR_LUMINOSITY, ORBITAL_DISTANCE);
        let shared = equilibrium_temperature(s, ALBEDO, FULL_REDISTRIBUTION);
        let n2 = n2_condensation_point(0.78 * SURFACE_PRESSURE);
        assert!(!night_side_collapses(shared, n2));
    }
}
