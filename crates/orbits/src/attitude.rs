// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Terminal attitude: how a box that landed askew learns its own
//! orientation well enough to point at a satellite it has not heard yet
//! (ADR-0031).
//!
//! One bearing fixes only two of the three attitude angles: a rotation
//! about the line to the satellite leaves that line unchanged. That is
//! harmless for the conjugate reply (ADR-0027), which retraces the arrival
//! without any attitude, and for closed-loop tracking of the serving
//! satellite; it is fatal for handover, where the box must turn "the next
//! satellite is there", known in the local level frame, into its own panel
//! frame. Two non-parallel directions known in both frames fix all three
//! angles — the TRIAD method. The terminal has two sources of such pairs:
//!
//! - **gravity and a bearing**: a solid-state accelerometer reads "down" in
//!   the panel frame; the satellite says where it is in the local level
//!   frame and the wavefront says where it is in the panel frame;
//! - **two bearings over time**: the serving satellite's direction at two
//!   instants, measured on the panel and announced by the satellite.
//!
//! The first fails only when the satellite is near the zenith (the two
//! directions nearly coincide); the second fails only when the two bearings
//! are close together. Used together they cover each other.

/// A direction or vector in three dimensions.
pub type Vec3 = [f64; 3];
/// A rotation matrix, row-major: `m[i][j]`.
pub type Mat3 = [[f64; 3]; 3];

pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn normalize(a: Vec3) -> Vec3 {
    let n = dot(a, a).sqrt();
    [a[0] / n, a[1] / n, a[2] / n]
}

pub fn mat_vec(m: &Mat3, v: Vec3) -> Vec3 {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}

fn columns(a: Vec3, b: Vec3, c: Vec3) -> Mat3 {
    [[a[0], b[0], c[0]], [a[1], b[1], c[1]], [a[2], b[2], c[2]]]
}

fn mat_mul_t(a: &Mat3, b: &Mat3) -> Mat3 {
    // a · bᵀ
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| a[i][k] * b[j][k]).sum();
        }
    }
    m
}

/// Angle (rad) between two directions.
pub fn angle_between(a: Vec3, b: Vec3) -> f64 {
    dot(normalize(a), normalize(b)).clamp(-1.0, 1.0).acos()
}

/// A direction in the local level frame (x east, y north, z up) at
/// `zenith` angle and `azimuth` (rad).
pub fn sky(zenith: f64, azimuth: f64) -> Vec3 {
    [
        zenith.sin() * azimuth.sin(),
        zenith.sin() * azimuth.cos(),
        zenith.cos(),
    ]
}

/// The TRIAD attitude: the rotation taking local-level directions into the
/// panel frame, from two directions known in both. `primary` is trusted
/// fully; `secondary` only sets the rotation about it, so the more accurate
/// measurement goes first.
pub fn triad(
    primary_body: Vec3,
    secondary_body: Vec3,
    primary_ref: Vec3,
    secondary_ref: Vec3,
) -> Mat3 {
    let frame = |p: Vec3, s: Vec3| {
        let t1 = normalize(p);
        let t2 = normalize(cross(t1, s));
        columns(t1, t2, cross(t1, t2))
    };
    mat_mul_t(
        &frame(primary_body, secondary_body),
        &frame(primary_ref, secondary_ref),
    )
}

/// A small, seeded generator so every run of the Monte Carlo below prints
/// the same numbers (xorshift64*, Box–Muller normals).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }

    pub fn uniform(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let x = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((x >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    pub fn normal(&mut self) -> f64 {
        let (u1, u2) = (self.uniform(), self.uniform());
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// `v` measured with an rms error of `sigma` (rad) on each of the two axes
/// across it.
pub fn perturb(v: Vec3, sigma: f64, rng: &mut Rng) -> Vec3 {
    let v = normalize(v);
    let helper = if v[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let p = normalize(cross(v, helper));
    let q = cross(v, p);
    let (a, b) = (sigma * rng.normal(), sigma * rng.normal());
    normalize([
        v[0] + a * p[0] + b * q[0],
        v[1] + a * p[1] + b * q[1],
        v[2] + a * p[2] + b * q[2],
    ])
}

/// The rotation of a box that landed tilted `tilt` (rad) about a random
/// horizontal axis and then turned to a random heading.
pub fn random_attitude(tilt: f64, rng: &mut Rng) -> Mat3 {
    let heading = 2.0 * std::f64::consts::PI * rng.uniform();
    let axis_az = 2.0 * std::f64::consts::PI * rng.uniform();
    let rot = |axis: Vec3, ang: f64| -> Mat3 {
        let (s, c) = (ang.sin(), ang.cos());
        let [x, y, z] = normalize(axis);
        [
            [
                c + x * x * (1.0 - c),
                x * y * (1.0 - c) - z * s,
                x * z * (1.0 - c) + y * s,
            ],
            [
                y * x * (1.0 - c) + z * s,
                c + y * y * (1.0 - c),
                y * z * (1.0 - c) - x * s,
            ],
            [
                z * x * (1.0 - c) - y * s,
                z * y * (1.0 - c) + x * s,
                c + z * z * (1.0 - c),
            ],
        ]
    };
    let yaw = rot([0.0, 0.0, 1.0], heading);
    let lean = rot([axis_az.cos(), axis_az.sin(), 0.0], tilt);
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| lean[i][k] * yaw[k][j]).sum();
        }
    }
    m
}

/// Handover pointing error: over `trials` landings, the worst angle (rad),
/// across every direction in the sky above `min_elevation`, between where
/// the attitude estimate says a target is and where it really is — the
/// error an open-loop repoint at handover would carry. `measure` builds the
/// estimate from the true attitude. Returns the 95th percentile across
/// trials.
pub fn handover_pointing_p95(
    trials: usize,
    tilt: f64,
    min_elevation: f64,
    seed: u64,
    mut measure: impl FnMut(&Mat3, &mut Rng) -> Mat3,
) -> f64 {
    let mut rng = Rng::new(seed);
    let zenith_max = std::f64::consts::FRAC_PI_2 - min_elevation;
    let targets: Vec<Vec3> = (0..=8)
        .flat_map(|i| {
            let z = zenith_max * i as f64 / 8.0;
            (0..24).map(move |j| sky(z, 2.0 * std::f64::consts::PI * j as f64 / 24.0))
        })
        .collect();
    let mut worst: Vec<f64> = (0..trials)
        .map(|_| {
            let truth = random_attitude(tilt, &mut rng);
            let estimate = measure(&truth, &mut rng);
            targets
                .iter()
                .map(|&t| angle_between(mat_vec(&estimate, t), mat_vec(&truth, t)))
                .fold(0.0, f64::max)
        })
        .collect();
    worst.sort_by(|a, b| a.partial_cmp(b).unwrap());
    worst[(0.95 * (trials - 1) as f64).round() as usize]
}

/// Gravity and one bearing: the accelerometer's "down" (rms `sigma_g` per
/// axis) as primary, the beacon bearing to a satellite at `zenith` (rms
/// `sigma_b`) as secondary.
pub fn gravity_and_bearing(
    sigma_g: f64,
    sigma_b: f64,
    zenith: f64,
) -> impl FnMut(&Mat3, &mut Rng) -> Mat3 {
    move |truth, rng| {
        let down = [0.0, 0.0, -1.0];
        let sat = sky(zenith, 2.0 * std::f64::consts::PI * rng.uniform());
        triad(
            perturb(mat_vec(truth, down), sigma_g, rng),
            perturb(mat_vec(truth, sat), sigma_b, rng),
            down,
            sat,
        )
    }
}

/// Two bearings to the serving satellite `separation` (rad) apart along its
/// pass, each with rms `sigma_b`, the first at `zenith`.
pub fn two_bearings(
    sigma_b: f64,
    zenith: f64,
    separation: f64,
) -> impl FnMut(&Mat3, &mut Rng) -> Mat3 {
    move |truth, rng| {
        let az = 2.0 * std::f64::consts::PI * rng.uniform();
        let s1 = sky(zenith, az);
        // The satellite moves `separation` across the sky, in a random
        // direction, staying above the horizon.
        let s2 = loop {
            let dir = 2.0 * std::f64::consts::PI * rng.uniform();
            let p = normalize(cross(s1, [0.0, 0.0, 1.0]));
            let q = cross(s1, p);
            let (c, s) = (separation.cos(), separation.sin());
            let cand = [
                c * s1[0] + s * (dir.cos() * p[0] + dir.sin() * q[0]),
                c * s1[1] + s * (dir.cos() * p[1] + dir.sin() * q[1]),
                c * s1[2] + s * (dir.cos() * p[2] + dir.sin() * q[2]),
            ];
            if cand[2] > 0.0 {
                break cand;
            }
        };
        triad(
            perturb(mat_vec(truth, s1), sigma_b, rng),
            perturb(mat_vec(truth, s2), sigma_b, rng),
            s1,
            s2,
        )
    }
}

/// The aperture (m) whose broadside half-power beam reaches `half_width`
/// (rad) each side of its axis at `frequency` (Hz): the patch of the
/// terminal's face that forms a listening beam just wide enough to cover
/// the attitude uncertainty at handover (the 70·λ/D rule, `radio`).
pub fn listen_aperture(half_width: f64, frequency: f64) -> f64 {
    70.0 * (crate::placement::SPEED_OF_LIGHT / frequency) / (2.0 * half_width.to_degrees())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triad_recovers_an_exact_attitude() {
        let mut rng = Rng::new(7);
        let truth = random_attitude(0.3, &mut rng);
        let (a, b) = (sky(0.4, 1.0), [0.0, 0.0, -1.0]);
        let est = triad(mat_vec(&truth, b), mat_vec(&truth, a), b, a);
        for t in [sky(0.2, 0.5), sky(1.0, 3.0), sky(0.7, -2.0)] {
            assert!(angle_between(mat_vec(&est, t), mat_vec(&truth, t)) < 1e-12);
        }
    }

    #[test]
    fn attitude_narrows_the_sky_to_a_few_degrees_not_a_pencil() {
        // Cold-start bearings (0.874°) and a 0.25° accelerometer, box tilted
        // 5°: the policy bound is the worse of gravity + bearing with the
        // satellite 30° from the zenith and two bearings 40° apart — about
        // 3.7°, far wider than the 0.70° Ka pencil half-width. Near the
        // zenith gravity + bearing degrades sharply; two bearings cover it.
        let (sb, sg) = (0.874_f64.to_radians(), 0.25_f64.to_radians());
        let (tilt, el) = (5.0_f64.to_radians(), 25.0_f64.to_radians());
        let p = |m| handover_pointing_p95(2_000, tilt, el, 11, m).to_degrees();
        let gb30 = p(gravity_and_bearing(sg, sb, 30.0_f64.to_radians()));
        let gb5 = p(gravity_and_bearing(sg, sb, 5.0_f64.to_radians()));
        let tb40 = handover_pointing_p95(
            2_000,
            tilt,
            el,
            13,
            two_bearings(sb, 30.0_f64.to_radians(), 40.0_f64.to_radians()),
        )
        .to_degrees();
        assert!((gb30 - 3.34).abs() < 0.05, "{gb30}");
        assert!((tb40 - 3.66).abs() < 0.05, "{tb40}");
        assert!(gb5 > 15.0, "{gb5}");
        assert!(gb30.max(tb40) > 5.0 * 0.70);
    }

    #[test]
    fn a_listening_beam_sized_to_the_bound_reads_the_bearing_finely() {
        // A 5° half-width listening beam at X is a 0.25 m patch of the
        // 0.5 m face; it reads the incoming satellite's X dwell at the rim
        // to well inside the Ka pencil's half-width at the 70° lean.
        use crate::acquisition::doa_rms;
        use crate::radio::{
            fspl_db, planar_array_gain_dbi, scanned_beamwidth_deg, thermal_noise_dbw,
        };
        let d = listen_aperture(5.0_f64.to_radians(), 8.4e9);
        assert!((d - 0.25).abs() < 0.005, "{d}");
        let scan = 70.0_f64.to_radians();
        let snr = 10.0 + planar_array_gain_dbi(0.7, 8.4e9, 0.6, 42.4_f64.to_radians(), 1.2)
            - 3.0
            - fspl_db(3.642e6, 8.4e9)
            + planar_array_gain_dbi(d, 8.4e9, 0.6, scan, 1.2)
            - thermal_noise_dbw(290.0, 50e3);
        assert!((snr - 32.9).abs() < 0.1, "{snr}");
        let bearing = doa_rms(
            scanned_beamwidth_deg(d, 8.4e9, scan),
            10.0_f64.powf(snr / 10.0),
        );
        assert!((bearing - 0.29).abs() < 0.01, "{bearing}");
        assert!(bearing * 6.5 < scanned_beamwidth_deg(0.5, 30e9, scan) / 2.0);
    }

    #[test]
    fn one_bearing_alone_leaves_the_twist_unknown() {
        // A rotation about the line to the satellite moves every other
        // direction but leaves that bearing exactly where it was.
        let s = sky(0.5, 0.3);
        let mut rng = Rng::new(3);
        let truth = random_attitude(0.2, &mut rng);
        let b = mat_vec(&truth, s);
        // Twist the true attitude about the measured bearing.
        let (c, sn) = (0.4_f64.cos(), 0.4_f64.sin());
        let [x, y, z] = b;
        let twist: Mat3 = [
            [
                c + x * x * (1.0 - c),
                x * y * (1.0 - c) - z * sn,
                x * z * (1.0 - c) + y * sn,
            ],
            [
                y * x * (1.0 - c) + z * sn,
                c + y * y * (1.0 - c),
                y * z * (1.0 - c) - x * sn,
            ],
            [
                z * x * (1.0 - c) - y * sn,
                z * y * (1.0 - c) + x * sn,
                c + z * z * (1.0 - c),
            ],
        ];
        let twisted = |v: Vec3| mat_vec(&twist, mat_vec(&truth, v));
        assert!(angle_between(twisted(s), b) < 1e-12);
        assert!(angle_between(twisted(sky(1.0, 2.0)), mat_vec(&truth, sky(1.0, 2.0))) > 0.1);
    }
}
