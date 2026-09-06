// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Review arithmetic for the interstellar backbone proposed in GitHub issue
//! #7 (1 Tbps full duplex, Solar System <-> Proxima Centauri).
//!
//! This example is self-contained on purpose: the interstellar link is not
//! part of the site's article series, so none of this belongs in the
//! library or in the posts' evidence examples. It exists so every number in
//! the issue-7 review comment can be regenerated from one command.
//!
//! Part I asks six questions, one section each:
//!
//!   A. Does the stated hardware deliver enough photons for 1 Tbps?
//!   B. Does the stated modulation fit 1 Tbps into any optical band?
//!   C. How far off-axis is the transmit aim point, and how well must the
//!      far gateway's orbit be known 4.25 years ahead?
//!   D. How much stellar light lands in a data lane?
//!   E. What does the receive-side event stream look like?
//!   F. Framing and transport arithmetic.
//!
//! Part II answers what Part I found:
//!
//!   G. A commissioning profile chosen by searching the CCSDS HPE menu for
//!      rate and photon margin, plus the coherent upgrade path.
//!   H. A pointing budget, an 8.5-year conical-scan calibration loop, and
//!      the orbit contract each gateway must fly.
//!   I. The Sun-side stand-off distance, or equivalently the scattered-
//!      light specification, derived from the diffraction floor.
//!   J. The combiner reduced to slot-synchronous count sums.
//!   K. Framing that fits RFC 6330's field widths.
//!
//! Inputs are the issue's own design point plus published astrometry and
//! photometry; each constant names its source. Nothing here is a
//! simulation — it is closed-form first-order physics, and its purpose is
//! to find the places where the issue's numbers do not close.
//!
//! Run: cargo run --release -p terminus-orbits --example interstellar_link

use std::f64::consts::PI;

use terminus_orbits::hill::SUN_MU;
use terminus_orbits::placement::SPEED_OF_LIGHT;

// ---------------------------------------------------------------------------
// Physical constants
// ---------------------------------------------------------------------------

/// Planck constant (J·s), CODATA 2018 exact.
const PLANCK: f64 = 6.626_070_15e-34;
/// Astronomical unit (m), IAU 2012 exact.
const AU: f64 = 1.495_978_707e11;
/// Julian light-year (m): c × 365.25 × 86 400.
const LIGHT_YEAR: f64 = 9.460_730_472_580_8e15;
/// Parsec (m).
const PARSEC: f64 = 3.085_677_581_491_37e16;
/// Julian year (s).
const YEAR: f64 = 365.25 * 86_400.0;
/// Milliarcsecond in radians.
const MAS: f64 = PI / (180.0 * 3600.0 * 1000.0);

// ---------------------------------------------------------------------------
// The issue's design point (issue #7, sections 3.3–3.6, 5.3, 8.2)
// ---------------------------------------------------------------------------

/// Net application rate per direction (bit/s).
const NET_RATE: f64 = 1e12;
/// Outer fountain overhead on normal objects.
const FOUNTAIN_OVERHEAD: f64 = 0.10;
/// Bulk-data PPM order and inner code rate for first production service.
const PPM_ORDER: u32 = 64;
const CODE_RATE: f64 = 1.0 / 3.0;
/// Data wavelength (m): optical C-band.
const WAVELENGTH: f64 = 1.55e-6;
/// Coherent transmit array: dense 300 m aperture, 100 kW average optical.
const TX_DIAMETER: f64 = 300.0;
const TX_POWER: f64 = 100e3;
/// Receive array: ~0.8 km² aggregate collecting area from ~1 150 30 m-class
/// collectors spread over a 5–20 km baseline, direct detection.
const RX_AREA: f64 = 0.8e6;
const RX_COLLECTOR_DIAMETER: f64 = 30.0;
const RX_COLLECTORS: f64 = 1_150.0;
/// Accepted frame payload (bytes) and fountain symbol size (bytes).
const FRAME_PAYLOAD: f64 = 65_536.0;
/// Candidate gateway orbit radii around each star (AU), issue section 1.
const GATEWAY_ORBITS_AU: [f64; 2] = [1.0, 5.0];

// ---------------------------------------------------------------------------
// CCSDS 141.0-B-2 / 142.0-B-2 (March 2026) — the issue's own PHY vocabulary
// ---------------------------------------------------------------------------

/// HPE telemetry band: 191.3–195.9 THz on the ITU-T G.694.1 100 GHz grid
/// (141.0-B-2 §3.3). Wavelengths 1530.33–1567.13 nm.
const HPE_BAND_LOW_HZ: f64 = 191.3e12;
const HPE_BAND_HIGH_HZ: f64 = 195.9e12;
const HPE_GRID_HZ: f64 = 100e9;
/// Smallest HPE slot width is 0.125 ns (141.0-B-2 §3.9), i.e. 8 GHz slots.
const HPE_MIN_SLOT_S: f64 = 0.125e-9;
/// Widest conventional low-loss silica window worth arguing about,
/// S+C+L bands ~1460–1625 nm, as an optimistic upper bound on WDM room.
const SCL_BAND_LOW_HZ: f64 = SPEED_OF_LIGHT / 1.625e-6;
const SCL_BAND_HIGH_HZ: f64 = SPEED_OF_LIGHT / 1.460e-6;

// ---------------------------------------------------------------------------
// Proxima Centauri and the Sun
// ---------------------------------------------------------------------------

/// Gaia DR3 parallax 768.0665 mas → 1.30197 pc, 4.2465 ly.
const DISTANCE_PC: f64 = 1.301_97;
/// Gaia DR3 proper motion (mas/yr): RA −3781.741, Dec +769.465.
const PM_RA_MAS_YR: f64 = -3781.741;
const PM_DEC_MAS_YR: f64 = 769.465;
/// Gaia DR3 radial velocity (m/s).
const RADIAL_VELOCITY: f64 = -22_204.0;
/// Proxima mass 0.1221 M☉ (Kervella et al. 2017 via the Wikipedia infobox).
const PROXIMA_MU: f64 = 0.1221 * SUN_MU;
/// ICRS J2000 position: 14h 29m 42.946s, −62° 40′ 46.16″.
const PROXIMA_RA_DEG: f64 = (14.0 + 29.0 / 60.0 + 42.946 / 3600.0) * 15.0;
const PROXIMA_DEC_DEG: f64 = -(62.0 + 40.0 / 60.0 + 46.16 / 3600.0);
/// Mean obliquity of the ecliptic, J2000 (deg).
const OBLIQUITY_DEG: f64 = 23.439_291;
/// Lunar axial tilt to the ecliptic (deg).
const LUNAR_TILT_DEG: f64 = 1.54;
/// 2MASS H-band apparent magnitude of Proxima (Cutri et al. 2003, via
/// SIMBAD): 4.835. H band (1.5–1.8 µm) brackets the 1.55 µm data lanes.
const PROXIMA_H_MAG: f64 = 4.835;
/// Absolute 2MASS H magnitude of the Sun, Vega system (Willmer 2018).
const SUN_ABS_H_MAG: f64 = 3.32;
/// 2MASS H zero-point flux density (W m⁻² µm⁻¹), Cohen et al. 2003.
const H_ZERO_POINT: f64 = 1.133e-9;
/// Narrow optical filter width per data lane (µm): 0.1 nm.
const LANE_FILTER_UM: f64 = 1e-4;
/// Scattered-light suppression assumed at tens of λ/D off-axis for a clean
/// unobstructed telescope without a coronagraph. A stated guess.
const PSF_WING_SUPPRESSION: f64 = 1e-4;

// ---------------------------------------------------------------------------
// Closed-form pieces
// ---------------------------------------------------------------------------

/// Photon energy (J) at `wavelength` (m): hc/λ.
fn photon_energy(wavelength: f64) -> f64 {
    PLANCK * SPEED_OF_LIGHT / wavelength
}

/// On-axis gain of a uniformly illuminated, diffraction-limited circular
/// aperture of `diameter` (m): (πD/λ)².
fn aperture_gain(diameter: f64, wavelength: f64) -> f64 {
    let x = PI * diameter / wavelength;
    x * x
}

/// Angle (rad) from boresight to the first Airy null: 1.22 λ/D.
fn first_null(diameter: f64, wavelength: f64) -> f64 {
    1.22 * wavelength / diameter
}

/// Half-power half-width (rad) of the Airy main lobe: ≈ 0.515 λ/D.
fn half_power_half_width(diameter: f64, wavelength: f64) -> f64 {
    0.515 * wavelength / diameter
}

/// Fraunhofer (far-field) distance (m) of an aperture: 2D²/λ. Closer than
/// this the beam has not yet formed its far-field pattern.
fn fraunhofer_distance(diameter: f64, wavelength: f64) -> f64 {
    2.0 * diameter * diameter / wavelength
}

/// Power (W) collected by `rx_area` (m²) at `range` (m) from a
/// diffraction-limited transmitter: Pₜ·G·A/(4πR²). No losses.
fn received_power(
    tx_power: f64,
    tx_diameter: f64,
    rx_area: f64,
    wavelength: f64,
    range: f64,
) -> f64 {
    tx_power * aperture_gain(tx_diameter, wavelength) * rx_area / (4.0 * PI * range * range)
}

/// Diameter (m) of the filled circle with the same area.
fn filled_equivalent_diameter(area: f64) -> f64 {
    2.0 * (area / PI).sqrt()
}

/// Information bits carried per PPM slot at order `m` and code rate `r`:
/// r·log₂M / M.
fn ppm_bits_per_slot(m: u32, r: f64) -> f64 {
    r * f64::from(m).log2() / f64::from(m)
}

/// Mean detected photons per information bit at which a noiseless
/// (zero-background) Poisson M-PPM channel has capacity exactly r·log₂M
/// bits per symbol. Each symbol is an M-ary erasure channel: capacity
/// log₂M·(1 − e⁻ⁿ) for n mean photons per symbol, so n = −ln(1 − r).
fn ppm_noiseless_photons_per_bit(m: u32, r: f64) -> f64 {
    let per_symbol = -(1.0 - r).ln();
    per_symbol / (r * f64::from(m).log2())
}

/// Photons per information bit at Shannon capacity for an ideal
/// heterodyne receiver (N₀ = hν) at spectral efficiency `eta` bit/s/Hz:
/// (2^η − 1)/η. Tends to ln 2 ≈ 0.69 as η → 0.
fn coherent_photons_per_bit(eta: f64) -> f64 {
    (2f64.powf(eta) - 1.0) / eta
}

/// Number of channels on a `spacing`-Hz grid spanning `low`..`high` Hz.
fn wdm_channels(low: f64, high: f64, spacing: f64) -> f64 {
    ((high - low) / spacing).floor() + 1.0
}

/// Point-ahead angle (rad) for relative transverse velocity `v` (m/s):
/// 2v/c — one v/c for the light you see, one for the light you send.
fn point_ahead(v: f64) -> f64 {
    2.0 * v / SPEED_OF_LIGHT
}

/// Circular orbital speed (m/s) at radius `r` around gravitational
/// parameter `mu`.
fn orbital_speed(mu: f64, r: f64) -> f64 {
    (mu / r).sqrt()
}

/// In-band power (W) collected from a star of Vega magnitude `mag` by
/// `area` (m²) through a filter `bandwidth_um` wide, given the band's
/// zero-point flux density (W m⁻² µm⁻¹).
fn stellar_in_band_power(mag: f64, zero_point: f64, bandwidth_um: f64, area: f64) -> f64 {
    zero_point * 10f64.powf(-0.4 * mag) * bandwidth_um * area
}

/// Apparent magnitude at `distance_pc` of a source with absolute
/// magnitude `abs_mag`.
fn apparent_magnitude(abs_mag: f64, distance_pc: f64) -> f64 {
    abs_mag + 5.0 * (distance_pc / 10.0).log10()
}

/// Ecliptic latitude (rad) of an ICRS direction (`ra`, `dec` in rad) for
/// obliquity `eps` (rad).
fn ecliptic_latitude(ra: f64, dec: f64, eps: f64) -> f64 {
    (dec.sin() * eps.cos() - dec.cos() * ra.sin() * eps.sin()).asin()
}

fn db(x: f64) -> f64 {
    10.0 * x.log10()
}

fn from_db(x: f64) -> f64 {
    10f64.powf(x / 10.0)
}

// ---------------------------------------------------------------------------
// Part II: what to do about it (sections G–K)
// ---------------------------------------------------------------------------

/// Practical gap (dB) between a real photon-counting PPM decoder and the
/// Poisson capacity line. SCPPM sits ~0.5 dB from capacity (Moision &
/// Hamkins 2003); the rest covers slot sync, dead time, and residual
/// background. A stated allowance.
const PPM_DECODER_GAP_DB: f64 = 1.5;
/// Gap for pilot-aided coherent detection with a strong code: the 2008
/// Lincoln Laboratory homodyne-PSK demonstration reached 1.5 photons/bit,
/// 4.5 dB from the Shannon limit (Stevens et al. 2008).
const COHERENT_GAP_DB: f64 = 4.5;
/// Slot clock for every lane: the CCSDS HPE maximum.
const SLOT_RATE_HZ: f64 = 1.0 / HPE_MIN_SLOT_S;
/// Grid spacings to search (Hz). 25 GHz is the tightest that respects the
/// 141.0-B-2 spectral mask (95% of energy within ±10 GHz of centre);
/// 12.5 GHz needs a cleaner laser than the standard asks for.
const GRIDS_HZ: [(f64, &str); 2] = [
    (25e9, "25 GHz, CCSDS mask"),
    (12.5e9, "12.5 GHz, tighter laser"),
];
const PPM_ORDERS: [u32; 7] = [4, 8, 16, 32, 64, 128, 256];
const CODE_RATES: [(f64, &str); 3] = [(1.0 / 3.0, "1/3"), (0.5, "1/2"), (2.0 / 3.0, "2/3")];
/// Lane reserve above the required rate: the issue wants ≥16 independently
/// failing lanes plus control/pilot lanes, so do not size to the last lane.
const LANE_RESERVE: f64 = 1.10;
/// Implementation loss assumed in section A, reused here.
const IMPLEMENTATION_LOSS_DB: f64 = 10.0;

/// One candidate commissioning profile.
struct Profile {
    m: u32,
    rate: &'static str,
    grid: &'static str,
    lanes: f64,
    /// bit/s after the outer code with every lane at the maximum slot clock.
    delivered: f64,
    /// Detected photons per net bit the decoder needs: capacity × gap.
    photons_needed: f64,
    /// Photon margin (dB) against what the link delivers.
    margin_db: f64,
}

/// Every (grid, M, r) from the CCSDS HPE menu that delivers `required`
/// bit/s with the lane reserve, sorted by photon margin.
fn search_profiles(photons_available: f64, required: f64) -> Vec<Profile> {
    let mut out = Vec::new();
    for (grid_hz, grid) in GRIDS_HZ {
        let lanes = wdm_channels(SCL_BAND_LOW_HZ, SCL_BAND_HIGH_HZ, grid_hz);
        for m in PPM_ORDERS {
            for (r, rate) in CODE_RATES {
                let delivered = lanes * SLOT_RATE_HZ * ppm_bits_per_slot(m, r);
                if delivered < required * LANE_RESERVE {
                    continue;
                }
                let photons_needed =
                    ppm_noiseless_photons_per_bit(m, r) * from_db(PPM_DECODER_GAP_DB);
                out.push(Profile {
                    m,
                    rate,
                    grid,
                    lanes,
                    delivered,
                    photons_needed,
                    margin_db: db(photons_available / photons_needed),
                });
            }
        }
    }
    out.sort_by(|a, b| b.margin_db.total_cmp(&a.margin_db));
    out
}

/// Fraction of on-axis power at `offset` (rad) from boresight, Gaussian
/// fit to the Airy main lobe with half-power half-width `hphw`.
fn pointing_loss(offset: f64, hphw: f64) -> f64 {
    (-std::f64::consts::LN_2 * (offset / hphw).powi(2)).exp()
}

/// Peak-to-mean received-power modulation seen by the far end when the
/// transmitter, mispointed by `offset`, dithers its aim by ±`dither`
/// (conical scan): the first-order derivative of [`pointing_loss`].
fn conscan_modulation(offset: f64, dither: f64, hphw: f64) -> f64 {
    2.0 * std::f64::consts::LN_2 * offset * dither / (hphw * hphw)
}

/// Photon-limited centroid precision (rad) of an aperture `diameter` (m)
/// on a point source after `photons` detected: (λ/D)/√N.
fn centroid_precision(diameter: f64, wavelength: f64, photons: f64) -> f64 {
    wavelength / diameter / photons.sqrt()
}

/// Along-track velocity change (m/s) that moves a circular-orbit body by
/// `displacement` (m) along its track after `time` (s): Δx ≈ 3·Δv·t.
fn along_track_dv(displacement: f64, time: f64) -> f64 {
    displacement / (3.0 * time)
}

/// Asymptotic Airy-pattern intensity relative to the core at `theta`
/// (rad) off-axis for an unobstructed aperture: 8/(π x³) with
/// x = πDθ/λ. The diffraction floor beneath any real scatter.
fn airy_wing_envelope(theta: f64, diameter: f64, wavelength: f64) -> f64 {
    let x = PI * diameter * theta / wavelength;
    8.0 / (PI * x.powi(3))
}

/// Off-axis angle (rad) at which [`airy_wing_envelope`] falls to `target`.
fn airy_wing_angle(target: f64, diameter: f64, wavelength: f64) -> f64 {
    let x = (8.0 / (PI * target)).cbrt();
    x * wavelength / (PI * diameter)
}

fn si(x: f64, unit: &str) -> String {
    let (scale, prefix) = if x >= 1e12 {
        (1e12, "T")
    } else if x >= 1e9 {
        (1e9, "G")
    } else if x >= 1e6 {
        (1e6, "M")
    } else if x >= 1e3 {
        (1e3, "k")
    } else if x >= 1.0 {
        (1.0, "")
    } else if x >= 1e-3 {
        (1e-3, "m")
    } else if x >= 1e-6 {
        (1e-6, "µ")
    } else if x >= 1e-9 {
        (1e-9, "n")
    } else if x >= 1e-12 {
        (1e-12, "p")
    } else if x >= 1e-15 {
        (1e-15, "f")
    } else if x >= 1e-18 {
        (1e-18, "a")
    } else {
        (1e-21, "z")
    };
    format!("{:.3} {prefix}{unit}", x / scale)
}

fn main() {
    let range = DISTANCE_PC * PARSEC;
    let coded_rate = NET_RATE * (1.0 + FOUNTAIN_OVERHEAD) / CODE_RATE;
    let outer_rate = NET_RATE * (1.0 + FOUNTAIN_OVERHEAD);

    println!(
        "Issue #7 review arithmetic: 1 Tbps optical backbone over {:.4} ly\n",
        range / LIGHT_YEAR
    );

    // -----------------------------------------------------------------------
    println!("A. Photon budget at the issue's design point\n");
    let e_photon = photon_energy(WAVELENGTH);
    let gain = aperture_gain(TX_DIAMETER, WAVELENGTH);
    let p_rx = received_power(TX_POWER, TX_DIAMETER, RX_AREA, WAVELENGTH, range);
    let photons_per_s = p_rx / e_photon;
    let photons_per_bit_ideal = photons_per_s / NET_RATE;
    let implementation_loss_db = 10.0;
    let photons_per_bit_real = photons_per_bit_ideal / 10f64.powf(implementation_loss_db / 10.0);
    println!(
        "   photon energy at {:.2} µm            {}\n\
         \x20  TX gain, {:.0} m dense aperture       {:.1} dB\n\
         \x20  RX equivalent filled diameter        {:.0} m ({} m² from {:.0} × {:.0} m)\n\
         \x20  received power, no losses            {}\n\
         \x20  received photon rate                 {:.2e} /s\n\
         \x20  photons per net bit, no losses       {:.1}\n\
         \x20  photons per net bit at −{:.0} dB       {:.2}   (optics, pointing, detector, fill factor: a stated guess)",
        WAVELENGTH * 1e6,
        si(e_photon, "J"),
        TX_DIAMETER,
        db(gain),
        filled_equivalent_diameter(RX_AREA),
        RX_AREA,
        RX_COLLECTORS,
        RX_COLLECTOR_DIAMETER,
        si(p_rx, "W"),
        photons_per_s,
        photons_per_bit_ideal,
        implementation_loss_db,
        photons_per_bit_real,
    );
    let ppm_need = ppm_noiseless_photons_per_bit(PPM_ORDER, CODE_RATE);
    println!(
        "\n   What the modulation asks for (detected photons per net bit):\n\
         \x20  {PPM_ORDER}-PPM, r = 1/3, noiseless capacity   {:.2}\n\
         \x20  4-PPM,  r = 1/2, noiseless capacity   {:.2}\n\
         \x20  coherent at 1 bit/s/Hz, Shannon       {:.2}\n\
         \x20  coherent at 2 bit/s/Hz, Shannon       {:.2}\n\
         \x20  coherent at 4 bit/s/Hz, Shannon       {:.2}\n\
         \x20  (real codes sit 2–3× above the capacity line; background adds more)",
        ppm_need,
        ppm_noiseless_photons_per_bit(4, 0.5),
        coherent_photons_per_bit(1.0),
        coherent_photons_per_bit(2.0),
        coherent_photons_per_bit(4.0),
    );
    println!(
        "\n   Verdict: photons are not the problem. The design point lands at\n\
         \x20  {:.0} photons/bit before losses, ~{:.1} after a −{:.0} dB guess, against a\n\
         \x20  need of {:.2} for the proposed 64-PPM profile. Section B is the problem.",
        photons_per_bit_ideal, photons_per_bit_real, implementation_loss_db, ppm_need
    );

    // -----------------------------------------------------------------------
    println!("\n\nB. Slot-rate arithmetic: can 64-PPM at r = 1/3 carry 1 Tbps?\n");
    let bits_per_slot = ppm_bits_per_slot(PPM_ORDER, CODE_RATE);
    let slots_needed = outer_rate / bits_per_slot;
    let hpe_lanes = wdm_channels(HPE_BAND_LOW_HZ, HPE_BAND_HIGH_HZ, HPE_GRID_HZ);
    let hpe_slot_rate = 1.0 / HPE_MIN_SLOT_S;
    let hpe_ceiling_slots = hpe_lanes * hpe_slot_rate;
    let hpe_ceiling_bps = hpe_ceiling_slots * bits_per_slot;
    println!(
        "   rate after 10% fountain overhead      {}\n\
         \x20  rate after r = 1/3 inner code         {}\n\
         \x20  net bits per PPM slot                 {:.4}   (r·log₂M / M)\n\
         \x20  PPM slots per second needed           {:.2e}  = {} of slot clock\n\n\
         \x20  CCSDS 141.0-B-2 HPE band              {:.1}–{:.1} THz, 100 GHz grid → {:.0} lanes\n\
         \x20  CCSDS 141.0-B-2 smallest slot         {:.3} ns → {} slots/s per lane\n\
         \x20  HPE ceiling, every lane at max clock  {:.2e} slots/s → {} net\n\
         \x20  shortfall against 1 Tbps              {:.0}×",
        si(outer_rate, "bit/s"),
        si(coded_rate, "bit/s"),
        bits_per_slot,
        slots_needed,
        si(slots_needed, "Hz"),
        HPE_BAND_LOW_HZ / 1e12,
        HPE_BAND_HIGH_HZ / 1e12,
        hpe_lanes,
        HPE_MIN_SLOT_S * 1e9,
        si(hpe_slot_rate, "Hz"),
        hpe_ceiling_slots,
        si(hpe_ceiling_bps, "bit/s"),
        NET_RATE / hpe_ceiling_bps,
    );
    let scl_width = SCL_BAND_HIGH_HZ - SCL_BAND_LOW_HZ;
    println!(
        "\n   Even ignoring the grid: at 1 slot per Hz of optical bandwidth (the\n\
         \x20  physical floor, since a slot needs at least its own inverse width of\n\
         \x20  spectrum) the {PPM_ORDER}-PPM profile needs {} of spectrum.\n\
         \x20  The whole S+C+L window (1460–1625 nm) is {} wide.",
        si(slots_needed, "Hz"),
        si(scl_width, "Hz"),
    );
    println!("\n   Spectrum needed at the 1 slot/Hz floor for other profiles:");
    for (m, r, label) in [
        (64u32, 1.0 / 3.0, "64-PPM r=1/3 (issue, ROBUST DATA)"),
        (16, 0.5, "16-PPM r=1/2 (issue, NOMINAL DATA)"),
        (4, 0.5, "4-PPM  r=1/2"),
        (4, 2.0 / 3.0, "4-PPM  r=2/3"),
    ] {
        let need = outer_rate / ppm_bits_per_slot(m, r);
        println!(
            "     {label:<36} {:>12}   {:.2} photons/bit at capacity",
            si(need, "Hz"),
            ppm_noiseless_photons_per_bit(m, r)
        );
    }
    for eta in [1.0, 2.0, 4.0] {
        println!(
            "     {:<36} {:>12}   {:.2} photons/bit at capacity",
            format!("coherent, {eta:.0} bit/s/Hz"),
            si(outer_rate / eta, "Hz"),
            coherent_photons_per_bit(eta)
        );
    }
    println!(
        "\n   Verdict: the commissioning profile cannot reach 1 Tbps in any optical\n\
         \x20  band. Photon efficiency and spectral efficiency trade against each\n\
         \x20  other (Moision & Hamkins 2003); at ~1 photon/bit and 1 Tbps the link\n\
         \x20  must sit near 1 bit/s/Hz, which means low-order PPM at r ≥ 1/2 or\n\
         \x20  pilot-aided coherent modulation. Open question 12 is not a later\n\
         \x20  optimization — it decides whether the design point closes."
    );

    // -----------------------------------------------------------------------
    println!("\n\nC. Pointing: beam footprint, point-ahead, and the far gateway's orbit\n");
    let theta_null = first_null(TX_DIAMETER, WAVELENGTH);
    let theta_hp = half_power_half_width(TX_DIAMETER, WAVELENGTH);
    println!(
        "   first null                            {:.2} nrad → {:.0} km radius at range\n\
         \x20  half-power half-width                 {:.2} nrad → {:.0} km radius at range\n\
         \x20  Fraunhofer distance of the TX array  {} = {:.2} AU (no far-field test target closer)",
        theta_null * 1e9,
        theta_null * range / 1e3,
        theta_hp * 1e9,
        theta_hp * range / 1e3,
        si(fraunhofer_distance(TX_DIAMETER, WAVELENGTH), "m"),
        fraunhofer_distance(TX_DIAMETER, WAVELENGTH) / AU,
    );
    let pm_total = PM_RA_MAS_YR.hypot(PM_DEC_MAS_YR);
    let v_transverse = pm_total * MAS / YEAR * range;
    let pa_stellar = point_ahead(v_transverse);
    println!(
        "\n   Gaia DR3 proper motion                {:.1} mas/yr → transverse velocity {:.1} km/s\n\
         \x20  radial velocity                       {:.1} km/s (Doppler {:.1} GHz at {:.1} THz)\n\
         \x20  bulk point-ahead, 2v⊥/c               {:.1} µrad = {:.1}″ = {:.0} data beamwidths",
        pm_total,
        v_transverse / 1e3,
        RADIAL_VELOCITY / 1e3,
        (RADIAL_VELOCITY / SPEED_OF_LIGHT * SPEED_OF_LIGHT / WAVELENGTH).abs() / 1e9,
        SPEED_OF_LIGHT / WAVELENGTH / 1e12,
        pa_stellar * 1e6,
        pa_stellar / MAS / 1000.0,
        pa_stellar / (2.0 * theta_hp),
    );
    println!("\n   Orbital aberration terms on top (each gateway's own orbit):");
    for a in GATEWAY_ORBITS_AU {
        let v_sun = orbital_speed(SUN_MU, a * AU);
        let v_prox = orbital_speed(PROXIMA_MU, a * AU);
        println!(
            "     {a:.0} AU: Sun-side {:.1} km/s → ±{:.0} µrad;  Proxima-side {:.1} km/s → ±{:.0} µrad;  period {:.1} / {:.1} yr",
            v_sun / 1e3,
            point_ahead(v_sun) * 1e6,
            v_prox / 1e3,
            point_ahead(v_prox) * 1e6,
            2.0 * PI * (a * AU) / v_sun / YEAR,
            2.0 * PI * (a * AU) / v_prox / YEAR,
        );
    }
    let pointing_budget = 1e-9;
    let offset = pointing_budget * range;
    let dv = pointing_budget * SPEED_OF_LIGHT / 2.0;
    println!(
        "\n   To hold {:.0} nrad of pointing error (≈ {:.0}% of half-power width):\n\
         \x20  far gateway position, {:.2} yr ahead    ±{:.0} km\n\
         \x20  relative transverse velocity           ±{:.2} m/s\n\
         \x20  point-ahead calibration                1 part in {:.0} of the bulk offset",
        pointing_budget * 1e9,
        pointing_budget / theta_hp * 100.0,
        range / SPEED_OF_LIGHT / YEAR,
        offset / 1e3,
        dv,
        pa_stellar / pointing_budget,
    );
    for a in GATEWAY_ORBITS_AU {
        let v_prox = orbital_speed(PROXIMA_MU, a * AU);
        println!(
            "     Proxima gateway at {a:.0} AU: ±{:.0} km along-track = ±{:.1} h of orbital timing, held over {:.2} yr",
            offset / 1e3,
            offset / v_prox / 3600.0,
            range / SPEED_OF_LIGHT / YEAR,
        );
    }
    let beta = ecliptic_latitude(
        PROXIMA_RA_DEG.to_radians(),
        PROXIMA_DEC_DEG.to_radians(),
        OBLIQUITY_DEG.to_radians(),
    );
    println!(
        "\n   Proxima's ecliptic latitude is {:.1}°, so from the lunar south pole it\n\
         \x20  never sets: elevation {:.1}° ± {:.1}° (lunar tilt), Sun always within\n\
         \x20  ±{:.1}° of the horizon. A lunar site has a centimetre-class ephemeris,\n\
         \x20  no atmosphere, permanently shadowed cryogenic ground, and solid rock\n\
         \x20  for a 5–20 km baseline — worth trading against a free-flying gateway.",
        beta.to_degrees(),
        beta.to_degrees().abs(),
        LUNAR_TILT_DEG,
        LUNAR_TILT_DEG,
    );

    // -----------------------------------------------------------------------
    println!("\n\nD. Stellar background in a data lane\n");
    let lanes_for_budget = wdm_channels(SCL_BAND_LOW_HZ, SCL_BAND_HIGH_HZ, 25e9);
    let signal_per_lane = p_rx / lanes_for_budget;
    let collector_lambda_over_d = WAVELENGTH / RX_COLLECTOR_DIAMETER;
    let proxima_power = stellar_in_band_power(PROXIMA_H_MAG, H_ZERO_POINT, LANE_FILTER_UM, RX_AREA);
    let sun_app_h = apparent_magnitude(SUN_ABS_H_MAG, DISTANCE_PC);
    let sun_power = stellar_in_band_power(sun_app_h, H_ZERO_POINT, LANE_FILTER_UM, RX_AREA);
    println!(
        "   lanes assumed for per-lane signal      {:.0} (S+C+L on a 25 GHz grid)\n\
         \x20  signal per lane, no losses            {}\n\
         \x20  Proxima, H = {:.3}, whole star in {:.1} nm  {} over the array ({:.2}× one lane)\n\
         \x20  Sun from Proxima, H = {:.2}             {} over the array ({:.0}× one lane)",
        lanes_for_budget,
        si(signal_per_lane, "W"),
        PROXIMA_H_MAG,
        LANE_FILTER_UM * 1e3,
        si(proxima_power, "W"),
        proxima_power / signal_per_lane,
        sun_app_h,
        si(sun_power, "W"),
        sun_power / signal_per_lane,
    );
    println!("\n   Star–gateway separation as seen from the far end:");
    for a in GATEWAY_ORBITS_AU {
        let sep = a * AU / range;
        println!(
            "     {a:.0} AU: {:.2} µrad = {:.2}″ = {:.0} λ/D of a {:.0} m collector → star at {:.0e} of its core: Proxima {:.0e}, Sun {:.0e} of a lane",
            sep * 1e6,
            sep / MAS / 1000.0,
            sep / collector_lambda_over_d,
            RX_COLLECTOR_DIAMETER,
            PSF_WING_SUPPRESSION,
            proxima_power * PSF_WING_SUPPRESSION / signal_per_lane,
            sun_power * PSF_WING_SUPPRESSION / signal_per_lane,
        );
    }
    println!(
        "\n   Verdict: with ≥1 AU of separation each 30 m collector resolves the\n\
         \x20  star by tens of beamwidths and a 0.1 nm filter leaves it far below one\n\
         \x20  lane, flares included (the cited NIR superflare was +10%). The Sun is\n\
         \x20  {:.0}× brighter than Proxima at H, so the Solar gateway's stand-off\n\
         \x20  distance matters more than the Proxima gateway's, and the issue does\n\
         \x20  not state it.",
        sun_power / proxima_power
    );

    // -----------------------------------------------------------------------
    println!("\n\nE. Receive-side event stream\n");
    let per_collector = photons_per_s / RX_COLLECTORS;
    let per_collector_lane = per_collector / lanes_for_budget;
    let event_bytes = 8.0;
    println!(
        "   photons per collector                 {:.2e} /s\n\
         \x20  photons per collector per lane        {:.2e} /s  (SNSPD arrays: ~1 Gcps, DSOC ground detector)\n\
         \x20  detector channels (collectors × lanes) {:.2e}\n\
         \x20  timestamp stream at {:.0} B/event       {}\n\
         \x20  memory for 1 s / 60 s of coded stream {} / {}",
        per_collector,
        per_collector_lane,
        RX_COLLECTORS * lanes_for_budget,
        event_bytes,
        si(photons_per_s * event_bytes, "B/s"),
        si(coded_rate / 8.0, "B"),
        si(coded_rate / 8.0 * 60.0, "B"),
    );

    // -----------------------------------------------------------------------
    println!("\n\nF. Framing and transport\n");
    let frames_per_s = NET_RATE / (FRAME_PAYLOAD * 8.0);
    let frames_per_century = frames_per_s * YEAR * 100.0;
    println!(
        "   64 KiB frames per second              {:.2e}\n\
         \x20  frames per century                    {:.1e} → at 1e−18 undetected/frame, {:.3} expected per century\n\
         \x20  64-bit tag false-accept per bad frame {:.1e}; 128-bit {:.1e}\n\
         \x20  RFC 6330 symbol-size field T          16 bits → max 65 535 B; a 64 KiB symbol (65 536 B) does not fit\n\
         \x20  RFC 6330 K′max                        56 403 ≥ 16 384 symbols per 1 GiB block, fine\n\
         \x20  1 GiB block at 1 Tbps                 {:.2} ms",
        frames_per_s,
        frames_per_century,
        frames_per_century * 1e-18,
        2f64.powi(-64),
        2f64.powi(-128),
        (1u64 << 30) as f64 * 8.0 / NET_RATE * 1e3,
    );

    // =======================================================================
    println!("\n\n================ Part II: what to do about it ================\n");

    // -----------------------------------------------------------------------
    println!("G. A capacity-derived commissioning profile\n");
    let photons_available = photons_per_bit_ideal / from_db(IMPLEMENTATION_LOSS_DB);
    let profiles = search_profiles(photons_available, outer_rate);
    println!(
        "   Search: every CCSDS HPE (M, r) on a 25 or 12.5 GHz grid across S+C+L,\n\
         \x20  all lanes at the 8 GHz slot clock, delivering ≥ {:.0}% of {} after the\n\
         \x20  outer code, ranked by photon margin against {:.2} photons/bit\n\
         \x20  (decoder {:.1} dB off the Poisson capacity line).\n",
        LANE_RESERVE * 100.0,
        si(outer_rate, "bit/s"),
        photons_available,
        PPM_DECODER_GAP_DB,
    );
    println!(
        "   {:<8} {:<5} {:<24} {:>6} {:>12} {:>10} {:>8}",
        "PPM", "rate", "grid", "lanes", "delivered", "need γ/b", "margin"
    );
    for p in profiles.iter().take(8) {
        println!(
            "   {:<8} {:<5} {:<24} {:>6.0} {:>12} {:>10.2} {:>+7.1} dB",
            format!("{}-PPM", p.m),
            p.rate,
            p.grid,
            p.lanes,
            si(p.delivered, "bit/s"),
            p.photons_needed,
            p.margin_db,
        );
    }
    let best_ccsds = profiles
        .iter()
        .find(|p| p.grid.starts_with("25"))
        .expect("some profile fits on the CCSDS grid");
    println!(
        "\n   Recommendation: {}-PPM, r = {} on the {} grid — {:.0} lanes,\n\
         \x20  {} delivered, {:+.1} dB photon margin. Replace the 64-PPM / r = 1/3\n\
         \x20  ROBUST DATA row with it; keep 256-PPM r = 1/3 for SAFE CONTROL, where\n\
         \x20  rate is irrelevant and photon efficiency is everything.",
        best_ccsds.m,
        best_ccsds.rate,
        best_ccsds.grid,
        best_ccsds.lanes,
        si(best_ccsds.delivered, "bit/s"),
        best_ccsds.margin_db,
    );
    println!("\n   Coherent upgrade path (pilot-aided, {COHERENT_GAP_DB:.1} dB off Shannon):");
    for eta in [1.0, 2.0] {
        let need = coherent_photons_per_bit(eta) * from_db(COHERENT_GAP_DB);
        let margin = db(photons_available / need);
        println!(
            "     {eta:.0} bit/s/Hz: {} of spectrum, needs {need:.2} photons/bit → {margin:+.1} dB; closes at {} TX or a {:.2}× wider array",
            si(outer_rate / eta, "Hz"),
            si(TX_POWER * need / photons_available, "W"),
            (need / photons_available).sqrt(),
        );
    }
    println!(
        "   So: commission on photon-counting PPM, and treat coherent as the\n\
         \x20  lane-count-halving upgrade that a 4–6 dB link improvement buys."
    );

    // -----------------------------------------------------------------------
    println!("\n\nH. Pointing: budget, calibration loop, and orbit contract\n");
    let offset = 1e-9;
    let dither = 0.5e-9;
    let pilot_photons_per_s = 1e6;
    println!(
        "   loss at {:.0} nrad mispointing           {:+.2} dB\n\
         \x20  conscan: ±{:.1} nrad dither at {:.0} nrad offset → {:.1}% power modulation, costs {:+.2} dB\n\
         \x20  TX array as astrometric receiver      λ/D = {:.2} nrad; {:.0e} pilot photons → {:.1} prad centroid\n\
         \x20  Δv to fix ±{:.0} km along-track over {:.2} yr   {:.2} m/s",
        offset * 1e9,
        db(pointing_loss(offset, theta_hp)),
        dither * 1e9,
        offset * 1e9,
        conscan_modulation(offset, dither, theta_hp) * 100.0,
        db(pointing_loss(dither, theta_hp)),
        WAVELENGTH / TX_DIAMETER * 1e9,
        pilot_photons_per_s,
        centroid_precision(TX_DIAMETER, WAVELENGTH, pilot_photons_per_s) * 1e12,
        pointing_budget * range / 1e3,
        range / SPEED_OF_LIGHT / YEAR,
        along_track_dv(pointing_budget * range, range / SPEED_OF_LIGHT),
    );
    println!(
        "\n   Plan: (1) fast jitter from local inertial sensors and metrology, as the\n\
         \x20  issue says; (2) receive boresight from the far pilot through the TX\n\
         \x20  array's own optics, so transmit and receive share one calibrated axis;\n\
         \x20  (3) a slow conical scan of the data beam whose modulation the far end\n\
         \x20  measures and reports on the return link — an 8.5-year loop, which is\n\
         \x20  fine because the terms it calibrates (proper-motion model, point-ahead\n\
         \x20  offset, array boresight) are constant or secular; (4) gateway orbits\n\
         \x20  published in the control plane as reference ephemerides, flown to\n\
         \x20  ±{:.0} km with {:.1} m/s-class corrections, and verified by the far\n\
         \x20  end from pilot Doppler and astrometry.",
        pointing_budget * range / 1e3,
        along_track_dv(pointing_budget * range, range / SPEED_OF_LIGHT),
    );

    // -----------------------------------------------------------------------
    println!("\n\nI. Stand-off distance from the diffraction floor\n");
    let lane_fraction_target = 1e-3;
    let sun_lane_ratio = sun_power / signal_per_lane;
    for a in GATEWAY_ORBITS_AU {
        let sep = a * AU / range;
        let env = airy_wing_envelope(sep, RX_COLLECTOR_DIAMETER, WAVELENGTH);
        println!(
            "   {a:.0} AU: Airy envelope {env:.1e} → Sun {:.1e} of a lane, Proxima {:.1e} of a lane (diffraction only)",
            sun_lane_ratio * env,
            proxima_power / signal_per_lane * env,
        );
    }
    let floor_target = lane_fraction_target / sun_lane_ratio;
    let standoff = airy_wing_angle(floor_target, RX_COLLECTOR_DIAMETER, WAVELENGTH) * range;
    println!(
        "\n   To keep the Sun ≤ {:.0e} of a lane the collectors must suppress it to\n\
         \x20  {:.1e}. Diffraction alone reaches that at {:.1} AU stand-off; closer in,\n\
         \x20  the same number becomes the scattered-light specification on every\n\
         \x20  30 m collector (a lunar site at 1 AU needs ≤ {:.0e}). Write one or the\n\
         \x20  other into section 1; the Proxima side is {:.0}× easier and can stay\n\
         \x20  at 1–5 AU.",
        lane_fraction_target,
        floor_target,
        standoff / AU,
        floor_target,
        sun_power / proxima_power,
    );

    // -----------------------------------------------------------------------
    println!("\n\nJ. Combiner: count sums are the sufficient statistic\n");
    let lanes = best_ccsds.lanes;
    let slots_per_s = lanes * SLOT_RATE_HZ;
    let count_bits = 4.0;
    let per_collector_lane = photons_per_s / RX_COLLECTORS / lanes;
    let mean_gap_slots = SLOT_RATE_HZ / per_collector_lane;
    let event_bits = mean_gap_slots.log2().ceil() + f64::from(best_ccsds.m).log2();
    println!(
        "   With no background and equal detector efficiency, the sum of photon\n\
         \x20  counts per slot across collectors is a sufficient statistic for the\n\
         \x20  Poisson channel: LLR combining reduces to slot-synchronous addition.\n\n\
         \x20  slots per second, {lanes:.0} lanes × 8 GHz     {:.2e}\n\
         \x20  photons per slot, array total          {:.2}   (pulsed slot: ×{})\n\
         \x20  per collector per lane                 {:.1e} /s → one photon per {:.0} slots\n\
         \x20  event = Δslot + slot-in-symbol         {event_bits:.0} bits → {} per collector, {} aggregate\n\
         \x20  count sums to the decoders at {count_bits:.0} bits  {}   (vs {} of raw timestamps)",
        slots_per_s,
        photons_per_s / from_db(IMPLEMENTATION_LOSS_DB) / slots_per_s,
        best_ccsds.m,
        per_collector_lane,
        mean_gap_slots,
        si(photons_per_s / RX_COLLECTORS * event_bits / 8.0, "B/s"),
        si(photons_per_s * event_bits / 8.0, "B/s"),
        si(slots_per_s * count_bits / 8.0, "B/s"),
        si(photons_per_s * event_bytes, "B/s"),
    );
    println!(
        "\n   Architecture: geometric-delay correction and slot alignment at each\n\
         \x20  collector; a tree of adders (clusters of ~32 collectors, then the\n\
         \x20  array) producing per-lane per-slot counts; decoders consume counts.\n\
         \x20  Unequal efficiencies or background bring back per-collector weights,\n\
         \x20  but as a scalar per collector per lane, not a per-photon LLR."
    );

    // -----------------------------------------------------------------------
    println!("\n\nK. Framing that fits RFC 6330\n");
    let symbol = 32_768.0;
    let symbols_per_frame = FRAME_PAYLOAD / symbol;
    let k = (1u64 << 30) as f64 / symbol;
    let header = 64.0;
    let tag = 16.0;
    println!(
        "   fountain symbol T = {symbol:.0} B (fits 16-bit T); {symbols_per_frame:.0} per 64 KiB frame\n\
         \x20  K per 1 GiB block = {k:.0} ≤ K′max 56 403\n\
         \x20  frame header {header:.0} B + 128-bit tag {tag:.0} B = {:.2}% overhead\n\
         \x20  false accept per decoder-passed bad frame  {:.1e}\n\
         \x20  expected undetected frames per century     {:.1e}",
        (header + tag) / FRAME_PAYLOAD * 100.0,
        2f64.powi(-128),
        frames_per_century * 2f64.powi(-128),
    );
    println!(
        "\n   Also: a wrong-but-accepted symbol poisons its whole 1 GiB block and the\n\
         \x20  block hash cannot name it, so the per-frame tag is what keeps the\n\
         \x20  fountain layer honest — say so in §8.3."
    );
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
    fn ccsds_grid_search_picks_8_ppm_rate_half() {
        // 1.14 photons/bit is section A's figure after −10 dB.
        let profiles = search_profiles(1.14, NET_RATE * (1.0 + FOUNTAIN_OVERHEAD));
        let best = profiles.iter().find(|p| p.grid.starts_with("25")).unwrap();
        assert_eq!((best.m, best.rate), (8, "1/2"));
        assert!(
            best.margin_db > 2.0 && best.margin_db < 3.0,
            "{}",
            best.margin_db
        );
        // The issue's own profile never appears: it cannot deliver the rate.
        assert!(profiles.iter().all(|p| !(p.m == 64 && p.rate == "1/3")));
    }

    #[test]
    fn pointing_loss_is_half_at_half_power_half_width() {
        assert_close(pointing_loss(2.66e-9, 2.66e-9), 0.5, 1e-12);
        assert_close(pointing_loss(0.0, 2.66e-9), 1.0, 1e-12);
    }

    #[test]
    fn conscan_matches_numerical_derivative() {
        let (hphw, offset, dither) = (2.66e-9, 1e-9, 0.05e-9);
        let numeric = (pointing_loss(offset - dither, hphw) - pointing_loss(offset + dither, hphw))
            / pointing_loss(offset, hphw)
            / 2.0;
        assert_close(conscan_modulation(offset, dither, hphw), numeric, 1e-2);
    }

    #[test]
    fn airy_wing_angle_inverts_envelope() {
        let theta = airy_wing_angle(1e-8, 30.0, 1.55e-6);
        assert_close(airy_wing_envelope(theta, 30.0, 1.55e-6), 1e-8, 1e-9);
    }

    #[test]
    fn along_track_correction_is_a_tenth_of_a_metre_per_second() {
        let range = DISTANCE_PC * PARSEC;
        assert_close(
            along_track_dv(1e-9 * range, range / SPEED_OF_LIGHT),
            0.1,
            5e-3,
        );
    }

    #[test]
    fn photon_energy_at_c_band() {
        assert_close(photon_energy(1.55e-6), 1.2816e-19, 1e-3);
    }

    #[test]
    fn gain_of_300_m_aperture_is_176_db() {
        assert_close(db(aperture_gain(300.0, 1.55e-6)), 175.68, 1e-3);
    }

    #[test]
    fn issue_design_point_receives_about_1_5_microwatts() {
        let range = DISTANCE_PC * PARSEC;
        let p = received_power(TX_POWER, TX_DIAMETER, RX_AREA, WAVELENGTH, range);
        assert_close(p, 1.457e-6, 2e-2);
        // Cross-check with the +30 dB scaling the issue quotes: 30 m / 10 kW
        // at the same receiver is 1000× less.
        let p0 = received_power(10e3, 30.0, RX_AREA, WAVELENGTH, range);
        assert_close(p / p0, 1000.0, 1e-9);
    }

    #[test]
    fn first_null_of_issue_beam_is_6_3_nrad() {
        assert_close(first_null(300.0, 1.55e-6), 6.303e-9, 1e-3);
    }

    #[test]
    fn ppm_bits_per_slot_64_third() {
        assert_close(ppm_bits_per_slot(64, 1.0 / 3.0), 1.0 / 32.0, 1e-12);
        assert_close(ppm_bits_per_slot(4, 0.5), 0.25, 1e-12);
    }

    #[test]
    fn hpe_band_holds_47_lanes() {
        assert_eq!(
            wdm_channels(HPE_BAND_LOW_HZ, HPE_BAND_HIGH_HZ, HPE_GRID_HZ),
            47.0
        );
    }

    #[test]
    fn noiseless_ppm_capacity_photons() {
        // 64-PPM at r = 1/3: n = −ln(2/3) = 0.405 photons/symbol over 2 bits.
        assert_close(ppm_noiseless_photons_per_bit(64, 1.0 / 3.0), 0.2027, 1e-3);
    }

    #[test]
    fn coherent_limit_tends_to_ln2() {
        assert_close(coherent_photons_per_bit(1e-6), std::f64::consts::LN_2, 1e-5);
        assert_close(coherent_photons_per_bit(2.0), 1.5, 1e-12);
    }

    #[test]
    fn point_ahead_for_proxima_is_about_160_microradians() {
        let range = DISTANCE_PC * PARSEC;
        let v = PM_RA_MAS_YR.hypot(PM_DEC_MAS_YR) * MAS / YEAR * range;
        assert_close(v, 23.8e3, 1e-2);
        assert_close(point_ahead(v), 1.59e-4, 1e-2);
    }

    #[test]
    fn fraunhofer_distance_of_tx_array_is_under_one_au() {
        assert_close(fraunhofer_distance(300.0, 1.55e-6), 1.161e11, 1e-3);
        assert!(fraunhofer_distance(300.0, 1.55e-6) < AU);
    }

    #[test]
    fn proxima_ecliptic_latitude() {
        let beta = ecliptic_latitude(
            PROXIMA_RA_DEG.to_radians(),
            PROXIMA_DEC_DEG.to_radians(),
            OBLIQUITY_DEG.to_radians(),
        );
        assert_close(beta.to_degrees(), -44.7, 5e-3);
    }

    #[test]
    fn sun_is_a_few_hundred_times_brighter_than_proxima_at_h() {
        let sun = apparent_magnitude(SUN_ABS_H_MAG, DISTANCE_PC);
        let ratio = 10f64.powf(-0.4 * (sun - PROXIMA_H_MAG));
        assert!(ratio > 200.0 && ratio < 300.0, "ratio {ratio}");
    }
}
