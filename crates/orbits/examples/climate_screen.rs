// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! What the star delivers to the reference planet, how warm that leaves the
//! ground, how deep the weather layer is, and what happens to the sky if the
//! circulation ever stops.
//!
//! This is the evidence behind the climate paragraphs of "know your planet"
//! and the RFP, and behind both modes of the circulation plate: the
//! temperatures it prints, the altitude its return branch rides at, and the
//! cold trap its stalled mode runs.
//!
//! It does not print a wind speed. The overturning velocity is a GCM result,
//! not a closed-form one, and the series cites it as literature rather than
//! manufacturing a number here.
//!
//! Run: cargo run -p terminus-orbits --example climate_screen

use terminus_orbits::climate::{
    celsius, co2_frost_point, equilibrium_temperature, greenhouse_increment, instellation,
    n2_condensation_point, night_side_collapses, radiative_floor, scale_height, AIR_MOLAR_MASS, AU,
    DAYSIDE_REDISTRIBUTION, EARTH_INTERNAL_FLUX, FULL_REDISTRIBUTION, NO_REDISTRIBUTION,
    SOLAR_CONSTANT, SOLAR_LUMINOSITY,
};

// World bible, "The planet" and "The star".
const STAR_LUMINOSITY: f64 = 0.001_55 * SOLAR_LUMINOSITY;
const ORBITAL_DISTANCE: f64 = 0.0485 * AU;
const SURFACE_GRAVITY: f64 = 9.82;
const SURFACE_PRESSURE: f64 = 101_325.0;
const ALBEDO: f64 = 0.3;
const BAND_TEMPERATURE: f64 = 281.15; // +8 C, the canonical twilight band

fn main() {
    println!("A. What the star delivers\n");
    let s = instellation(STAR_LUMINOSITY, ORBITAL_DISTANCE);
    println!(
        "   star luminosity         = {:.4e} W  (0.00155 solar)",
        STAR_LUMINOSITY
    );
    println!(
        "   orbital distance        = {:.4e} m  (0.0485 AU)",
        ORBITAL_DISTANCE
    );
    println!("   instellation            = {s:.1} W/m^2");
    println!(
        "   => {:.2} x Earth's {SOLAR_CONSTANT:.0} W/m^2  (published for Proxima b: 0.65)\n",
        s / SOLAR_CONSTANT
    );

    println!("B. What that leaves the ground, before any air moves\n");
    println!(
        "   The redistribution factor is the whole climate model in one number.\n\
         \x20  These two rows are the extremes; every real climate is between them.\n"
    );
    let substellar = equilibrium_temperature(s, ALBEDO, NO_REDISTRIBUTION);
    let dayside = equilibrium_temperature(s, ALBEDO, DAYSIDE_REDISTRIBUTION);
    let global = equilibrium_temperature(s, ALBEDO, FULL_REDISTRIBUTION);
    println!(
        "   substellar, no transport  = {substellar:6.1} K  ({:+6.1} C)",
        celsius(substellar)
    );
    println!(
        "   day side only             = {dayside:6.1} K  ({:+6.1} C)",
        celsius(dayside)
    );
    println!(
        "   shared over the sphere    = {global:6.1} K  ({:+6.1} C)",
        celsius(global)
    );
    println!(
        "   canonical twilight band   = {BAND_TEMPERATURE:6.1} K  ({:+6.1} C)   <- inside the bracket",
        celsius(BAND_TEMPERATURE)
    );
    let increment = greenhouse_increment(BAND_TEMPERATURE, global);
    println!("   => greenhouse increment the band asks for: {increment:.1} K  (Earth's is 33 K)\n");

    println!("C. How deep the weather is\n");
    let h = scale_height(BAND_TEMPERATURE, AIR_MOLAR_MASS, SURFACE_GRAVITY);
    let h_earth = scale_height(288.15, AIR_MOLAR_MASS, 9.806_65);
    println!(
        "   scale height H = RT/(Mg)  = {:.2} km   (Earth: {:.2} km)",
        h / 1e3,
        h_earth / 1e3
    );
    println!(
        "   => the cell's nightward branch rides near the top of that layer,\n\
         \x20     which is why the figure draws it at 7-10 km rather than picking one.\n"
    );

    println!("D. The cold trap: what the circulation is for\n");
    let floor = radiative_floor(EARTH_INTERNAL_FLUX);
    let n2 = n2_condensation_point(0.78 * SURFACE_PRESSURE);
    let co2 = co2_frost_point(40.0); // ~400 ppm of a 1-bar atmosphere
    println!("   night side with no transport, radiating against the");
    println!(
        "   planet's own interior heat ({EARTH_INTERNAL_FLUX} W/m^2) = {floor:.1} K  ({:+.1} C)",
        celsius(floor)
    );
    println!(
        "   CO2 frosts out at         = {co2:6.1} K  ({:+6.1} C)",
        celsius(co2)
    );
    println!(
        "   N2 condenses at           = {n2:6.1} K  ({:+6.1} C)",
        celsius(n2)
    );
    println!(
        "   => stalled: {}",
        if night_side_collapses(floor, n2) {
            "the night side is below both. The sky migrates to the dark\n                and freezes onto it, and the habitable band goes with it."
        } else {
            "no collapse"
        }
    );
    println!(
        "   => running: night side near {global:.0} K, {} — the air stays air.\n",
        if night_side_collapses(global, n2) {
            "still a trap"
        } else {
            "well above the N2 point"
        }
    );

    println!("E. What is NOT computed here\n");
    println!(
        "   Surface wind speed. The series quotes 5-15 m/s at the surface and a\n\
         \x20  return flow several times faster, from the GCM literature for tidally\n\
         \x20  locked M-dwarf planets. No closed form on this page produces that\n\
         \x20  number, and printing one would be false precision."
    );
}
