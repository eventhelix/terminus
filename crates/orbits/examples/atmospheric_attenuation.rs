//! Atmospheric fine print for the band plan of a tidally locked reference
//! planet: clear-air gaseous absorption (ITU-R P.676-3 Annex 2) and rain
//! attenuation (ITU-R P.838-3, circular polarization) across the candidate
//! bands, plus the two molecular landmarks — the 22.235 GHz water-vapour
//! rotational line and the 60 GHz oxygen spin-flip wall.
//!
//! Atmosphere: the world bible's working assumption — Earth-like pressure
//! and composition (ITU reference: 1013 hPa, 15 °C, 7.5 g/m³ water
//! vapour). The star owns 1–3 GHz outright (coherent stellar emission),
//! independent of anything the air does.
//!
//! Run: cargo run -p terminus-orbits --example atmospheric_attenuation

use terminus_orbits::atmosphere::{rain_db_per_km, SurfaceAir};

const STEADY_RAIN: f64 = 5.0; // mm/h
const HEAVY_RAIN: f64 = 25.0; // mm/h
const STORM_CELL_KM: f64 = 5.0;

fn main() {
    let air = SurfaceAir::default();
    println!("Sea-level air: 1013 hPa, 15 C, 7.5 g/m3 water vapour (world-bible");
    println!("working assumption: Earth-like pressure and composition).");
    println!("Rain: ITU-R P.838-3, circular polarization.\n");

    println!(
        "{:<10} {:>8} {:>13} {:>12} {:>13}  {:<16}",
        "point", "f (GHz)", "clear (dB/km)", "5 mm/h", "25 mm/h", "note"
    );
    for (label, f_ghz, note) in [
        ("L", 1.6, "IN STELLAR BAND"),
        ("S", 2.5, "IN STELLAR BAND"),
        ("X", 8.4, "diversity band"),
        ("Ku", 14.0, ""),
        ("H2O line", 22.235, "vapour rotation"),
        ("Ka", 30.0, "primary band"),
        ("O2 wall", 60.0, "spin-flip complex"),
    ] {
        let f = f_ghz * 1e9;
        println!(
            "{:<10} {:>8.3} {:>13.3} {:>12.2} {:>13.2}  {:<16}",
            label,
            f_ghz,
            air.gaseous_db_per_km(f),
            rain_db_per_km(f, STEADY_RAIN),
            rain_db_per_km(f, HEAVY_RAIN),
            note
        );
    }

    let ka_storm = rain_db_per_km(30e9, HEAVY_RAIN) * STORM_CELL_KM;
    let x_storm = rain_db_per_km(8.4e9, HEAVY_RAIN) * STORM_CELL_KM;
    println!(
        "\nA {STORM_CELL_KM:.0} km heavy-rain cell (25 mm/h) in the path costs \
         {ka_storm:.1} dB at Ka\nand {x_storm:.1} dB at X: the storm can spend \
         Ka's entire 25.5 dB aperture\nadvantage, which is why X-band rides \
         along as the all-weather fallback."
    );
}
