//! Can one ring serve the band alone, and does multi-ring coverage depend on
//! how the rings are phased against each other?
//!
//! Evidence for the access-layer decision: the strict duty-ring option, the
//! altitudes and elevation masks that would rescue it, how many rings can
//! actually reach a town, and a randomized sweep over uncoordinated
//! inter-ring phasing.
//!
//! Run: cargo run --release -p terminus-orbits --example duty_ring_trade

use std::f64::consts::PI;
use terminus_orbits::circular::orbital_period;
use terminus_orbits::constellation::{band_point, visible_count_with_phases, PolarConstellation};
use terminus_orbits::coverage::edge_slant_range;
use terminus_orbits::duty::{
    duty_ring, min_sats_per_ring_for_duty_only, rings_in_reach, rings_serving, worst_cross_track,
};
use terminus_orbits::{constellation::plane_visible_count, CentralBody};

const BAND: f64 = 20.0 * PI / 180.0;
const MASK: f64 = 25.0 * PI / 180.0;
const ROTATION: f64 = 11.2 * 86_400.0;
const C_LIGHT: f64 = 299_792_458.0;

fn planet() -> CentralBody {
    CentralBody::from_earth_masses(1.0, 6.371e6, ROTATION)
}

fn ring(altitude: f64, planes: usize, sats: usize) -> PolarConstellation {
    PolarConstellation {
        altitude,
        planes,
        sats_per_plane: sats,
        interplane_phase: 0.0,
    }
}

/// Fewest satellites visible from any sampled band point over a full rotation.
fn min_visible(
    p: &CentralBody,
    c: &PolarConstellation,
    phases: &[f64],
    mask: f64,
    step: f64,
) -> usize {
    let mut min = usize::MAX;
    let mut t = 0.0;
    while t < ROTATION {
        for i in 0..72 {
            let az = i as f64 * 2.0 * PI / 72.0;
            for off in [-BAND, 0.0, BAND] {
                let n = visible_count_with_phases(p, c, band_point(az, off), phases, mask, t);
                if n == 0 {
                    return 0;
                }
                min = min.min(n);
            }
        }
        t += step;
    }
    min
}

/// Fewest satellites of the *duty ring alone* visible from any band point.
fn min_visible_duty_only(p: &CentralBody, c: &PolarConstellation, mask: f64, step: f64) -> usize {
    let mut min = usize::MAX;
    let mut t = 0.0;
    while t < ROTATION {
        let k = duty_ring(p, c, t);
        for i in 0..72 {
            let az = i as f64 * 2.0 * PI / 72.0;
            for off in [-BAND, 0.0, BAND] {
                let n = plane_visible_count(p, c, k, band_point(az, off), 0.0, mask, t);
                if n == 0 {
                    return 0;
                }
                min = min.min(n);
            }
        }
        t += step;
    }
    min
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn main() {
    let p = planet();
    let baseline = ring(2_200e3, 6, 12);

    println!("A. Could one ring serve the band alone?\n");
    println!(
        "   The duty ring is at worst {:.0} deg off the terminator, and the band\n\
         \x20  reaches {:.0} deg to either side, so a town can sit {:.0} deg cross-track\n\
         \x20  of the duty ring's track. A ring that cannot reach that far cannot be\n\
         \x20  rescued by adding satellites to it.\n",
        terminus_orbits::duty::max_duty_misalignment(6).to_degrees(),
        BAND.to_degrees(),
        worst_cross_track(BAND, 6).to_degrees()
    );
    println!(
        "{:>9} {:>8} {:>13} {:>15} {:>18}",
        "alt (km)", "mask", "lambda (deg)", "reach needed", "sats/ring needed"
    );
    let reach_cases: [(f64, f64); 8] = [
        (2_200.0, 25.0),
        (3_000.0, 25.0),
        (4_000.0, 25.0),
        (5_200.0, 25.0),
        (6_000.0, 25.0),
        (7_300.0, 25.0),
        (2_200.0, 10.0),
        (2_200.0, 5.0),
    ];
    for (alt, mask) in reach_cases {
        let c = ring(alt * 1e3, 6, 12);
        let m = mask.to_radians();
        let need = min_sats_per_ring_for_duty_only(&p, &c, BAND, m);
        println!(
            "{:>9.0} {:>7.0}d {:>12.2}d {:>14.2}d {:>18}",
            alt,
            mask,
            c.footprint_half_angle(&p, m).to_degrees(),
            worst_cross_track(BAND, 6).to_degrees(),
            need.map_or("impossible at any count".to_string(), |n| n.to_string())
        );
    }

    println!("\n   Simulated, to confirm the analysis:\n");
    println!(
        "{:>9} {:>8} {:>8} {:>7} {:>16} {:>14}",
        "alt (km)", "mask", "sats/ring", "total", "min (duty only)", "edge 1-way"
    );
    let sim_cases: [(f64, f64, usize); 6] = [
        (2_200.0, 25.0, 12),
        (2_200.0, 25.0, 24),
        (2_200.0, 25.0, 48),
        (2_200.0, 5.0, 14),
        (6_000.0, 25.0, 14),
        (7_300.0, 25.0, 9),
    ];
    for (alt, mask, sats) in sim_cases {
        let c = ring(alt * 1e3, 6, sats);
        let m = mask.to_radians();
        println!(
            "{:>9.0} {:>7.0}d {:>8} {:>7} {:>16} {:>11.1} ms",
            alt,
            mask,
            sats,
            6 * sats,
            min_visible_duty_only(&p, &c, m, 30.0),
            edge_slant_range(&p, alt * 1e3, m) / C_LIGHT * 1e3
        );
    }

    println!("\n\nB. How many rings can reach a town, and how many are serving it?\n");
    println!(
        "   Polar planes all converge at the poles, so a town's ring count depends\n\
         \x20  on latitude. Near the equator the wheel is a two-ring affair.\n"
    );
    println!(
        "{:>14} {:>26} {:>26} {:>14}",
        "|latitude|", "rings in reach (min/mean/max)", "rings serving (min/mean/max)", "duty share"
    );
    let phases = vec![0.0; baseline.planes];
    let buckets = [
        (0.0, 15.0),
        (15.0, 30.0),
        (30.0, 50.0),
        (50.0, 70.0),
        (70.0, 90.0),
    ];
    for (lo, hi) in buckets {
        let (mut rmin, mut rmax, mut rsum) = (usize::MAX, 0usize, 0u64);
        let (mut smin, mut smax, mut ssum) = (usize::MAX, 0usize, 0u64);
        let (mut n, mut duty_sats, mut all_sats) = (0u64, 0u64, 0u64);
        let mut t = 0.0;
        while t < ROTATION {
            let k = duty_ring(&p, &baseline, t);
            for i in 0..72 {
                let az = i as f64 * 2.0 * PI / 72.0;
                for off in [-BAND, 0.0, BAND] {
                    let g = band_point(az, off);
                    let lat = g[2].asin().to_degrees().abs();
                    if lat < lo || lat >= hi {
                        continue;
                    }
                    let reach = rings_in_reach(&p, &baseline, g, MASK, t);
                    let serve = rings_serving(&p, &baseline, g, &phases, MASK, t);
                    rmin = rmin.min(reach);
                    rmax = rmax.max(reach);
                    rsum += reach as u64;
                    smin = smin.min(serve);
                    smax = smax.max(serve);
                    ssum += serve as u64;
                    duty_sats += plane_visible_count(&p, &baseline, k, g, 0.0, MASK, t) as u64;
                    all_sats +=
                        visible_count_with_phases(&p, &baseline, g, &phases, MASK, t) as u64;
                    n += 1;
                }
            }
            t += 120.0;
        }
        if n == 0 {
            continue;
        }
        println!(
            "{:>10.0}-{:<3.0} {:>10} /{:>6.2} /{:>5} {:>12} /{:>6.2} /{:>5} {:>13.0}%",
            lo,
            hi,
            rmin,
            rsum as f64 / n as f64,
            rmax,
            smin,
            ssum as f64 / n as f64,
            smax,
            100.0 * duty_sats as f64 / all_sats as f64
        );
    }

    println!("\n\nC. Does coverage depend on inter-ring phasing?\n");
    println!(
        "   Each ring is given an independent, arbitrary along-orbit offset, as an\n\
         \x20  uncoordinated launch campaign would leave it. A coverage claim has to\n\
         \x20  survive every one of them.\n"
    );
    let trials: u64 = 64;
    let threads = std::thread::available_parallelism().map_or(8, |n| n.get());
    println!(
        "{:>9} {:>7} {:>9} {:>7} {:>18} {:>18} {:>11}",
        "alt (km)",
        "rings",
        "sats/ring",
        "total",
        "phase-locked min",
        "worst random min",
        "failures"
    );
    for (alt, planes, sats) in [
        (2_200.0, 6, 12),
        (2_200.0, 6, 18),
        (2_400.0, 6, 12),
        (2_200.0, 8, 12),
    ] {
        let c = ring(alt * 1e3, planes, sats);
        let locked = min_visible(&p, &c, &vec![0.0; planes], MASK, 30.0);
        let per = trials.div_ceil(threads as u64);
        let out: Vec<(usize, u64)> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..threads as u64)
                .map(|ti| {
                    let c = &c;
                    let p = &p;
                    sc.spawn(move || {
                        let (mut worst, mut fails) = (usize::MAX, 0u64);
                        for idx in (ti * per)..((ti + 1) * per).min(trials) {
                            let mut rng = Rng(0x5EED_1234u64
                                .wrapping_add(idx.wrapping_mul(0x9E37_79B9_7F4A_7C15)));
                            rng.next();
                            let ph: Vec<f64> = (0..planes)
                                .map(|_| rng.next() * 2.0 * PI / sats as f64)
                                .collect();
                            let m = min_visible(p, c, &ph, MASK, 30.0);
                            if m == 0 {
                                fails += 1;
                            }
                            worst = worst.min(m);
                        }
                        (worst, fails)
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().expect("worker")).collect()
        });
        println!(
            "{:>9.0} {:>7} {:>9} {:>7} {:>18} {:>18} {:>8}/{}",
            alt,
            planes,
            sats,
            planes * sats,
            locked,
            out.iter().map(|r| r.0).min().unwrap_or(0),
            out.iter().map(|r| r.1).sum::<u64>(),
            trials
        );
    }

    let period = orbital_period(&p, 2_200e3) / 60.0;
    println!(
        "\n\nBaseline stands: 6 rings x 12 at 2,200 km. The duty ring names the ring\n\
         doing the most work, not the only ring working; coverage is a property of\n\
         the wheel and holds under arbitrary inter-ring phasing. In-ring spacing is\n\
         {:.1} min at a {:.1} min period; dual coverage costs 6 x 18 = 108.",
        period / 12.0,
        period
    );
}
