//! Does it matter how the rings are phased against each other?
//!
//! Three phasings are worth asking about:
//!
//!   aligned  every ring in step (the baseline; `interplane_phase = 0`)
//!   optimal  each ring offset half an in-plane slot from the last, so
//!            neighbouring rings' satellites interleave -- the triangular
//!            lattice a cellular network uses to cover a plane with the
//!            fewest cells (a Walker phasing factor)
//!   random   an independent, arbitrary offset per ring, which is what an
//!            uncoordinated launch campaign actually produces
//!
//! The sizing question is whether the optimal phasing buys satellites. It does
//! not, and section C explains why the cellular instinct does not survive the
//! move from a plane to a polar wheel.
//!
//! Run: cargo run --release -p terminus-orbits --example phasing_options

use terminus_orbits::constellation::{band_point, visible_count_with_phases, PolarConstellation};
use terminus_orbits::CentralBody;
use std::f64::consts::PI;

const MASK: f64 = 25.0 * PI / 180.0;
const BAND: f64 = 20.0 * PI / 180.0;
const ROTATION: f64 = 11.2 * 86_400.0;

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

fn phases(mode: &str, planes: usize, sats: usize, rng: &mut Rng) -> Vec<f64> {
    let slot = 2.0 * PI / sats as f64;
    (0..planes)
        .map(|k| match mode {
            "optimal" => k as f64 * slot / 2.0,
            "random" => rng.next() * slot,
            _ => 0.0,
        })
        .collect()
}

fn min_visible(p: &CentralBody, c: &PolarConstellation, ph: &[f64], step: f64, az: usize) -> usize {
    let mut min = usize::MAX;
    let mut t = 0.0;
    while t < ROTATION {
        for i in 0..az {
            let a = i as f64 * 2.0 * PI / az as f64;
            for off in [-BAND, 0.0, BAND] {
                let n = visible_count_with_phases(p, c, band_point(a, off), ph, MASK, t);
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

fn ring(alt: f64, sats: usize) -> PolarConstellation {
    PolarConstellation {
        altitude: alt,
        planes: 6,
        sats_per_plane: sats,
        interplane_phase: 0.0,
    }
}

fn main() {
    let p = CentralBody::from_earth_masses(1.0, 6.371e6, ROTATION);
    let mut rng = Rng(0x51E7_2026);

    println!("A. The sampling trap, reproduced deliberately\n");
    println!(
        "   At coarse sampling the optimal phasing looks like it rescues 1,800 km.\n\
         \x20  It does not: the silences are simply narrower than the sample grid.\n"
    );
    println!(
        "{:>10} {:>10} {:>22} {:>22}",
        "alt (km)", "phasing", "min @ 120 s / 36 az", "min @ 30 s / 72 az"
    );
    for alt in [1_800.0, 2_200.0] {
        for mode in ["aligned", "optimal", "random"] {
            let c = ring(alt * 1e3, 12);
            let ph = phases(mode, 6, 12, &mut rng);
            println!(
                "{:>10.0} {:>10} {:>22} {:>22}",
                alt,
                mode,
                min_visible(&p, &c, &ph, 120.0, 36),
                min_visible(&p, &c, &ph, 30.0, 72)
            );
        }
    }

    println!("\n\nB. Does the optimal phasing buy satellites?\n");
    println!(
        "   Not a question with a monotone answer, so the whole profile is shown\n\
         \x20  rather than the first count that happens to work. `optimal` offsets each\n\
         \x20  ring by half a slot, and a slot is 2*pi/sats -- so changing the count\n\
         \x20  changes the geometry, and coverage can come back and go away again.\n"
    );
    print!("{:>10} {:>10}  ", "alt (km)", "phasing");
    for sats in 9..=14 {
        print!("{sats:>5}");
    }
    println!("   <- sats/ring");
    for alt in [2_200.0, 2_400.0] {
        for mode in ["aligned", "optimal", "random"] {
            print!("{:>10.0} {:>10}  ", alt, mode);
            for sats in 9..=14 {
                let c = ring(alt * 1e3, sats);
                let draws = if mode == "random" { 4 } else { 1 };
                let mut worst = usize::MAX;
                for _ in 0..draws {
                    let ph = phases(mode, 6, sats, &mut rng);
                    worst = worst.min(min_visible(&p, &c, &ph, 30.0, 72));
                }
                print!("{worst:>5}");
            }
            println!();
        }
    }

    println!("\n\nC. Why the honeycomb does not transfer\n");
    println!(
        "   A triangular lattice is the best way to tile a *plane* with discs, and\n\
         \x20  that is the instinct behind offsetting neighbouring rings by half a\n\
         \x20  slot. A polar wheel is not a plane:\n\n\
         \x20  - Every ring passes over both poles, so the cells pinch to nothing at\n\
         \x20    the caps. An offset tuned for the equator is wrong everywhere else.\n\
         \x20  - A ring is a closed circle, so it serves the band at TWO antipodal\n\
         \x20    longitudes at once, climbing north on one and falling south on the\n\
         \x20    other. 'The offset between two rings' is therefore not one number:\n\
         \x20    whatever interleave is arranged on the north-going side is mirrored\n\
         \x20    on the south-going side, and only an even satellite count keeps the\n\
         \x20    two consistent at all (180 deg is then a whole number of slots).\n\
         \x20  - Nodes are spread over 180 deg, not 360, so the wheel closes on itself\n\
         \x20    through a seam where neighbouring rings run in OPPOSITE directions:\n\
         \x20    one climbing north while the other falls south. No fixed phase offset\n\
         \x20    holds an interleave across that seam, because the relative geometry\n\
         \x20    there never stops changing.\n\
         \x20  - With an even number of rings a uniform k*(slot/2) collapses anyway:\n\
         \x20    modulo one slot it is 0, half, 0, half, ... - an alternation of two\n\
         \x20    groups rather than a progressive lattice.\n"
    );

    println!(
        "The optimal phasing is not reliably better, and at the 12-satellite baseline\n\
         it is worse: it breaks coverage that the aligned and random wheels hold. The\n\
         honeycomb buys nothing here, so the fleet keeps the freedom to ignore\n\
         inter-ring phase entirely (ADR-0016) -- worth far more than the satellites it\n\
         did not save."
    );
}
