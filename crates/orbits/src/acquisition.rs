// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Cold-start acquisition arithmetic: how long a just-landed terminal waits
//! for a beacon when satellites raster their footprint spot by spot — and
//! why the terminal's own receive side never rasters back (ADR-0027): each
//! element of its panel hears the whole visible sky at once, and the phase
//! tilt of the arriving wavefront across the face measures the beacon's
//! direction without ever forming a search beam.

use crate::coverage::footprint_radius;
use crate::CentralBody;

/// Area estimate of a footprint's spot count: the ratio of the footprint's
/// spherical-cap area to a spot's cap area.
///
/// Not a covering, and loose both ways. Circles cannot tile, so this many
/// spots leave gaps (a gap-free hexagonal grid of them needs 2π/√27 ≈
/// 1.209× as many); and every position is priced at the given (nadir) spot
/// size, while a leaning beam's spot lands up to 5.3× longer
/// (`beams::spot_half_extent`). The raster the lantern walks is
/// `covering_raster`, which fixes both.
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

/// Number of receive-beam positions needed to tile the sky a terminal must
/// watch — the spherical cap above `min_elevation` (rad) — with beams of
/// `beamwidth` (rad): the same cap-area ratio as [`spots_per_footprint`],
/// pointed up instead of down. This is the price a terminal would pay to
/// search for the beacon with a directional beam; ADR-0027 exists because
/// that price, either way you pay it, is worse than not searching at all.
pub fn sky_positions(min_elevation: f64, beamwidth: f64) -> f64 {
    let cap = std::f64::consts::FRAC_PI_2 - min_elevation;
    (1.0 - cap.cos()) / (1.0 - (beamwidth / 2.0).cos())
}

/// Root-mean-square direction-of-arrival error — same angular unit as
/// `beamwidth` — when an aperture reads a wavefront's direction from the
/// phase tilt across its own face: the classic monopulse/interferometer
/// rule of thumb `θ_bw / (k·√(2·SNR))` with slope constant k = 1.6.
///
/// The physics, in one breath: a plane wave arriving off boresight reaches
/// one edge of the aperture a fraction of a wavelength before the other,
/// so every element sees the same signal at a slightly different carrier
/// phase, and fitting the tilt of that phase plane *is* the direction
/// measurement. No beam is formed and nothing is scanned; accuracy is set
/// by the aperture size (through `beamwidth`) and by how cleanly each
/// phase is read (through `snr`, a linear power ratio).
pub fn doa_rms(beamwidth: f64, snr: f64) -> f64 {
    beamwidth / (1.6 * (2.0 * snr).sqrt())
}

/// Fraction of a satellite's footprint that lies within `band_half_angle`
/// (rad) of the habitable band's central great circle, for a sub-satellite
/// point `track_offset` (rad) off that circle.
///
/// The raster region is this intersection — one generic rule for every
/// active satellite: a duty-ring satellite riding the terminator keeps ~95%
/// of its footprint, while a hole-filler lit from a displaced ring keeps
/// only the sliver that clips the band. This is the share of *area*; the
/// trimmed round counts raster *positions* (`raster_in_band`), and rim
/// positions are few and long, so the two differ.
pub fn band_raster_fraction(
    body: &CentralBody,
    altitude: f64,
    min_elevation: f64,
    band_half_angle: f64,
    track_offset: f64,
) -> f64 {
    let lambda = footprint_radius(body, altitude, min_elevation) / body.radius;
    let (n_rho, n_psi) = (256, 512);
    let (mut inside, mut total) = (0.0, 0.0);
    for i in 0..n_rho {
        let rho: f64 = lambda * (i as f64 + 0.5) / n_rho as f64;
        let weight = rho.sin();
        for j in 0..n_psi {
            let psi = 2.0 * std::f64::consts::PI * (j as f64 + 0.5) / n_psi as f64;
            // Spherical law of cosines: the latitude (off the band's central
            // great circle) of the cap point at polar coords (rho, psi).
            let sin_lat =
                track_offset.sin() * rho.cos() + track_offset.cos() * rho.sin() * psi.cos();
            total += weight;
            if sin_lat.abs() <= band_half_angle.sin() {
                inside += weight;
            }
        }
    }
    inside / total
}

/// Normalized offset `u` of the look direction (`eta`, `phi`) — nadir angle
/// and azimuth at the satellite, rad — from a beam of full half-power width
/// `beamwidth` aimed at (`aim_eta`, `aim_phi`): `u = 1` on the beam's
/// half-power (-3 dB) contour. A planar array's beam is an elliptical cone,
/// broadened by 1/cos η in its scan plane and not across it — the same
/// model as `beams::spot_outline` — so on the ground it lands elongated.
pub fn beam_offset(beamwidth: f64, aim_eta: f64, aim_phi: f64, eta: f64, phi: f64) -> f64 {
    let dir = |e: f64, p: f64| [e.sin() * p.cos(), e.sin() * p.sin(), -e.cos()];
    let d = dir(eta, phi);
    let axis = dir(aim_eta, aim_phi);
    let radial = [
        aim_eta.cos() * aim_phi.cos(),
        aim_eta.cos() * aim_phi.sin(),
        aim_eta.sin(),
    ];
    let cross = [-aim_phi.sin(), aim_phi.cos(), 0.0];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let along = dot(d, axis);
    let (dr, dc) = (dot(d, radial).atan2(along), dot(d, cross).atan2(along));
    let (half_r, half_c) = (beamwidth / aim_eta.cos() / 2.0, beamwidth / 2.0);
    ((dr / half_r).powi(2) + (dc / half_c).powi(2)).sqrt()
}

/// Gain lost (dB, ≤ 0) at normalized offset `u` from a beam's axis: the
/// Gaussian main-lobe approximation, -3 dB at the half-power contour.
pub fn beam_edge_loss_db(u: f64) -> f64 {
    -3.0 * u * u
}

/// The beacon raster as look directions `(nadir angle, azimuth)` (rad): a
/// hexagonal covering of the cone of directions out to `eta_max`, laid in
/// rings that follow each beam's true size. Rings step outward by
/// `radial_step` half-widths of the local, scan-broadened beam (β/2cos η),
/// and within a ring beams sit √3 cross half-widths (β/2) apart, staggered
/// ring to ring — so the rim, where beams land long, takes far fewer
/// positions than a nadir-pitch grid. `covering_raster` picks the step.
pub fn beacon_raster(eta_max: f64, beamwidth: f64, radial_step: f64) -> Vec<(f64, f64)> {
    let half_c = beamwidth / 2.0;
    let mut out = vec![(0.0, 0.0)];
    let mut eta = 0.0_f64;
    let mut ring = 0;
    loop {
        let half_r = beamwidth / (eta + 0.5 * radial_step * beamwidth / 2.0).cos() / 2.0;
        eta += radial_step * half_r;
        ring += 1;
        let n = (2.0 * std::f64::consts::PI * eta.sin() / (3.0_f64.sqrt() * half_c)).ceil();
        let stagger = if ring % 2 == 1 { 0.5 } else { 0.0 };
        for i in 0..n as usize {
            out.push((eta, 2.0 * std::f64::consts::PI * (i as f64 + stagger) / n));
        }
        if eta - 0.5 * radial_step * half_r >= eta_max {
            break;
        }
    }
    out
}

/// Worst normalized offset over the cone of directions out to `eta_max`:
/// the largest, over every direction, of its offset from the nearest raster
/// beam. `≤ 1` means the raster leaves no point outside a half-power
/// contour; the beam-edge loss there is `beam_edge_loss_db` of it. Returns
/// `(u, eta, phi)` of the worst direction.
pub fn raster_worst_offset(
    raster: &[(f64, f64)],
    eta_max: f64,
    beamwidth: f64,
    n_eta: usize,
    n_phi: usize,
) -> (f64, f64, f64) {
    let mut worst = (0.0_f64, 0.0, 0.0);
    sweep_nearest(raster, eta_max, beamwidth, n_eta, n_phi, |eta, phi, u| {
        if u > worst.0 {
            worst = (u, eta, phi);
        }
    });
    worst
}

/// Visit every direction of an `n_eta` × `n_phi` grid over the cone out to
/// `eta_max`, with its offset `u` from the nearest raster beam.
fn sweep_nearest(
    raster: &[(f64, f64)],
    eta_max: f64,
    beamwidth: f64,
    n_eta: usize,
    n_phi: usize,
    mut visit: impl FnMut(f64, f64, f64),
) {
    let dir = |e: f64, p: f64| [e.sin() * p.cos(), e.sin() * p.sin(), -e.cos()];
    let dirs: Vec<[f64; 3]> = raster.iter().map(|&(e, p)| dir(e, p)).collect();
    for i in 0..=n_eta {
        let eta = eta_max * i as f64 / n_eta as f64;
        // Only beams within three local half-widths can be the nearest one.
        let reach = 1.5 * beamwidth / eta.cos();
        let near: Vec<usize> = (0..raster.len())
            .filter(|&k| (raster[k].0 - eta).abs() < reach)
            .collect();
        let cos_reach = reach.cos();
        for j in 0..n_phi {
            let phi = 2.0 * std::f64::consts::PI * j as f64 / n_phi as f64;
            let d = dir(eta, phi);
            let u = near
                .iter()
                .filter(|&&k| d[0] * dirs[k][0] + d[1] * dirs[k][1] + d[2] * dirs[k][2] > cos_reach)
                .map(|&k| beam_offset(beamwidth, raster[k].0, raster[k].1, eta, phi))
                .fold(f64::INFINITY, f64::min);
            visit(eta, phi, u);
        }
    }
}

/// The weakest beacon anywhere in the footprint: over every look direction
/// out to `eta_max`, `peak_db(eta)` — the link toward that direction at a
/// beam's peak — plus the beam-edge loss from the nearest raster beam.
/// Returns `(snr_db, eta, edge_loss_db)` at the weakest direction.
pub fn raster_link_floor(
    raster: &[(f64, f64)],
    eta_max: f64,
    beamwidth: f64,
    peak_db: impl Fn(f64) -> f64,
) -> (f64, f64, f64) {
    let mut floor = (f64::INFINITY, 0.0, 0.0);
    sweep_nearest(raster, eta_max, beamwidth, 240, 720, |eta, _, u| {
        let snr = peak_db(eta) + beam_edge_loss_db(u);
        if snr < floor.0 {
            floor = (snr, eta, beam_edge_loss_db(u));
        }
    });
    floor
}

/// Raster positions whose aim point lands inside the habitable band
/// (`band_half_angle`, rad, about its central great circle) for a
/// sub-satellite point `track_offset` (rad) off that circle — the positions
/// a lantern actually walks under the footprint-∩-band rule. Rim positions
/// are few and long, so this is not the band's share of the footprint's
/// area.
pub fn raster_in_band(
    body: &CentralBody,
    altitude: f64,
    raster: &[(f64, f64)],
    band_half_angle: f64,
    track_offset: f64,
) -> usize {
    raster
        .iter()
        .filter(|&&(eta, phi)| {
            let rho = crate::beams::ray_ground_angle(body, altitude, eta);
            let sin_lat =
                track_offset.sin() * rho.cos() + track_offset.cos() * rho.sin() * phi.cos();
            sin_lat.abs() <= band_half_angle.sin()
        })
        .count()
}

/// The gap-free beacon raster for one footprint: the widest ring step (in
/// 0.05 increments from 1.5 down) whose raster leaves every direction out
/// to `eta_max` within a half-power contour. Returns the raster and its
/// worst offset `(u, eta, phi)`.
pub fn covering_raster(eta_max: f64, beamwidth: f64) -> (Vec<(f64, f64)>, (f64, f64, f64)) {
    let mut step = 1.5;
    loop {
        let raster = beacon_raster(eta_max, beamwidth, step);
        let worst = raster_worst_offset(&raster, eta_max, beamwidth, 240, 720);
        if worst.0 <= 1.0 || step <= 0.5 {
            return (raster, worst);
        }
        step -= 0.05;
    }
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

    #[test]
    fn six_hundred_receive_beam_positions_tile_the_visible_sky() {
        // A 0.5 m terminal panel throws a 5.0° beam at X (8.4 GHz); the sky
        // it must watch is the cap above the 25° elevation floor.
        use crate::radio::beamwidth_deg;
        let bw = beamwidth_deg(0.5, 8.4e9).to_radians();
        assert_close(sky_positions(25.0_f64.to_radians(), bw), 607.47, 1e-3);
    }

    #[test]
    fn an_unsynchronized_two_sided_search_blows_the_budget() {
        // If the terminal searched with a directional receive beam, each of
        // its 607 sky positions must be held for one full lantern round
        // before moving on — over an hour and a half against a 15-minute
        // acquisition budget.
        let round = beacon_raster_period(925.0, 0.010);
        let search = 607.47 * round;
        assert_close(search, 5_619.0, 1e-2);
        assert!(search / (15.0 * 60.0) > 6.0, "must be ~6x over the budget");
    }

    #[test]
    fn a_nested_receive_raster_is_worse_than_not_scanning() {
        // The alternative: sweep all 607 positions electronically inside
        // each 10 ms beacon dwell. The pointed beam buys array-over-element
        // gain, but splitting the dwell 607 ways costs more integration
        // time than the gain repays — scanning nets ~-2.2 dB.
        use crate::radio::{beamwidth_deg, dish_gain_dbi};
        let bw = beamwidth_deg(0.5, 8.4e9).to_radians();
        let n = sky_positions(25.0_f64.to_radians(), bw);
        let element_gain_dbi = 5.0;
        let net = (dish_gain_dbi(0.5, 8.4e9, 0.6) - element_gain_dbi) - 10.0 * n.log10();
        assert_close(net, -2.182, 1e-2);
        assert!(net < 0.0);
    }

    /// The X beacon's footprint: the reference shell, the 25° mask, and the
    /// 0.7 m face's 3.57° beam.
    fn x_beacon() -> (CentralBody, f64, f64) {
        use crate::beams::nadir_angle;
        use crate::radio::beamwidth_deg;
        let p = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        let edge = footprint_radius(&p, 2_200e3, 25.0_f64.to_radians()) / p.radius;
        let eta_max = nadir_angle(&p, 2_200e3, edge);
        (p, eta_max, beamwidth_deg(0.7, 8.4e9).to_radians())
    }

    #[test]
    fn the_raster_covers_the_footprint_with_elongated_spots() {
        // A gap-free covering that follows each beam's true size: 925
        // positions (9.25 s), against 1,333 for the gappy area estimate and
        // 1,611 for a gap-free grid of nadir-sized spots. No direction sits
        // outside a half-power contour, so beam-edge loss stays within 3 dB.
        let (_, eta_max, bw) = x_beacon();
        let (raster, (u, _, _)) = covering_raster(eta_max, bw);
        assert_eq!(raster.len(), 925);
        assert!(u <= 1.0, "worst offset {u}");
        assert!(beam_edge_loss_db(u) >= -3.0);
        // A step wider than the covering one leaves gaps.
        let looser = beacon_raster(eta_max, bw, 1.15);
        assert!(raster_worst_offset(&looser, eta_max, bw, 240, 720).0 > 1.0);
    }

    #[test]
    fn beacon_closes_everywhere_in_the_footprint() {
        // Every direction, at its nearest raster beam: 10 W behind the
        // satellite's 0.7 m X aperture steered off nadir (scan loss, rolloff
        // 1.2), the slant to the ground, the terminal's 5 dBi element leaned
        // to the satellite, 50 kHz at 290 K. The weakest point sits between
        // beams near the rim — 15.1 dB, against 17.3 dB at a rim beam's peak.
        use crate::beams::{ray_ground_angle, slant_range};
        use crate::radio::{fspl_db, planar_array_gain_dbi, scan_loss_db, thermal_noise_dbw};
        let (p, eta_max, bw) = x_beacon();
        let (raster, _) = covering_raster(eta_max, bw);
        let peak = |eta: f64| {
            let gamma = ray_ground_angle(&p, 2_200e3, eta);
            10.0 + planar_array_gain_dbi(0.7, 8.4e9, 0.6, eta, 1.2)
                - fspl_db(slant_range(&p, 2_200e3, gamma), 8.4e9)
                + 5.0
                + scan_loss_db(eta + gamma, 1.2)
                - thermal_noise_dbw(290.0, 50e3)
        };
        let (snr, eta, edge) = raster_link_floor(&raster, eta_max, bw, peak);
        assert_close(snr, 15.15, 1e-3);
        assert_close(eta.to_degrees(), 41.8, 1e-2);
        assert_close(edge, -2.6, 2e-2);
        assert_close(peak(eta_max), 17.3, 1e-2);

        // The compass there: the X receive pattern broadened at the
        // satellite's zenith angle (η + γ), read at that SNR, fixes the
        // beacon's direction to 0.87° rms — still 3.8x finer than the 3.31°
        // Ka pencil (broadened at the 65° rim lean) the box must seed.
        use crate::radio::scanned_beamwidth_deg;
        let zenith = eta + ray_ground_angle(&p, 2_200e3, eta);
        let doa = doa_rms(
            scanned_beamwidth_deg(0.5, 8.4e9, zenith),
            10.0_f64.powf(snr / 10.0),
        );
        assert_close(doa, 0.87, 1e-2);
        let ka = scanned_beamwidth_deg(0.5, 30e9, 65.0_f64.to_radians());
        assert_close(ka / doa, 3.8, 2e-2);
    }

    #[test]
    fn trimmed_rounds_count_positions_not_area() {
        // Under the footprint-∩-band rule a duty-ring satellite walks 839 of
        // the 925 positions (8.4 s) and a hole-filler 30° off walks 182
        // (1.8 s): rim positions are few and long, so these are not the
        // 95% and 24% area shares.
        let (p, eta_max, bw) = x_beacon();
        let (raster, _) = covering_raster(eta_max, bw);
        let band = 20.0_f64.to_radians();
        assert_eq!(raster_in_band(&p, 2_200e3, &raster, band, 0.0), 839);
        assert_eq!(
            raster_in_band(&p, 2_200e3, &raster, band, 30.0_f64.to_radians()),
            182
        );
    }

    #[test]
    fn doa_improves_as_the_square_root_of_snr() {
        assert_close(doa_rms(1.0, 400.0), 0.0221, 1e-2);
        let four_times = doa_rms(5.0, 100.0) / doa_rms(5.0, 400.0);
        assert_close(four_times, 2.0, 1e-9);
    }

    #[test]
    fn raster_region_is_the_footprint_band_intersection() {
        // One generic rule: footprint ∩ ±20° band. On the terminator the
        // 22.65°-radius footprint keeps ~95%; the share falls monotonically
        // as the sub-satellite point moves off the band's central circle,
        // and vanishes once the footprint no longer reaches the band.
        let p = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        let eps = 25.0_f64.to_radians();
        let band = 20.0_f64.to_radians();
        let frac = |t: f64| band_raster_fraction(&p, 2_200e3, eps, band, t.to_radians());
        assert_close(frac(0.0), 0.95, 2e-2);
        let mut last = 1.0;
        for t in [0.0, 10.0, 20.0, 30.0, 40.0] {
            let f = frac(t);
            assert!(f < last, "not monotone at {t}");
            last = f;
        }
        assert_close(frac(20.0), 0.51, 5e-2);
        assert!(frac(45.0) < 1e-9);
    }
}
