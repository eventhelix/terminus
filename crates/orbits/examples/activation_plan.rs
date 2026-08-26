//! How few satellites have to be switched on to serve the whole band?
//!
//! "Rings in reach" counts what a town can see, not what the fleet must
//! operate. Because polar rings converge at the poles, a high-latitude town
//! has all six rings overhead while needing no more service than anyone else.
//! This example asks the sharper question: at each instant, what is the
//! smallest set of satellites that still serves every point of the band?
//!
//! Four policies are compared against a proved optimum:
//!   - all on            - the fleet as flown today
//!   - duty ring only    - the tempting simplification (it does not cover)
//!   - duty first        - light the duty ring, patch the holes
//!   - greedy / exact    - set cover, heuristic and proved
//!
//! Run: cargo run --release -p terminus-orbits --example activation_plan

use terminus_orbits::activation::{
    covering_satellites, duty_first_activation, exact_activation, fleet_size, satellite_index,
    select_active, ActivationPlan,
};
use terminus_orbits::constellation::{band_point, PolarConstellation};
use terminus_orbits::duty::duty_ring;
use terminus_orbits::CentralBody;
use std::f64::consts::PI;

const MASK: f64 = 25.0 * PI / 180.0;
const BAND: f64 = 20.0 * PI / 180.0;
const ROTATION: f64 = 11.2 * 86_400.0;

struct Stat {
    name: &'static str,
    total: u64,
    max: usize,
    steps: u64,
    churn: u64,
    gaps: u64,
}

impl Stat {
    fn new(name: &'static str) -> Self {
        Stat { name, total: 0, max: 0, steps: 0, churn: 0, gaps: 0 }
    }
    fn record(&mut self, plan: &ActivationPlan, prev: &Option<Vec<bool>>, covered_all: bool) {
        self.total += plan.lit as u64;
        self.max = self.max.max(plan.lit);
        self.steps += 1;
        if !covered_all {
            self.gaps += 1;
        }
        if let Some(p) = prev {
            self.churn += (0..plan.active.len()).filter(|&s| p[s] != plan.active[s]).count() as u64;
        }
    }
    fn print(&self, fleet: usize, hours: f64) {
        println!(
            "{:>18} {:>10.1} {:>9} {:>11.0}% {:>14.1} {:>12}",
            self.name,
            self.total as f64 / self.steps as f64,
            self.max,
            100.0 * self.total as f64 / self.steps as f64 / fleet as f64,
            self.churn as f64 / hours,
            if self.gaps == 0 { "none".to_string() } else { format!("{} steps", self.gaps) }
        );
    }
}

fn covers_all(cov: &[Vec<usize>], active: &[bool]) -> bool {
    cov.iter().all(|sats| sats.iter().any(|&s| active[s]))
}

fn main() {
    let p = CentralBody::from_earth_masses(1.0, 6.371e6, ROTATION);
    let c = PolarConstellation {
        altitude: 2_200e3,
        planes: 6,
        sats_per_plane: 12,
        interplane_phase: 0.0,
    };
    let phases = vec![0.0; c.planes];
    let fleet = fleet_size(&c);

    let mut pts = Vec::new();
    for i in 0..72 {
        let az = i as f64 * 2.0 * PI / 72.0;
        for off in [-BAND, 0.0, BAND] {
            pts.push(band_point(az, off));
        }
    }

    let step = 60.0;
    let hours = ROTATION / 3600.0;

    println!("Serving the +/-20 deg band at a 25 deg mask, 6 x 12 at 2,200 km.");
    println!("Full 11.2-day rotation sampled every {step:.0} s, 72 azimuths x 3 band offsets.\n");
    println!(
        "{:>18} {:>10} {:>9} {:>12} {:>14} {:>12}",
        "policy", "mean lit", "max lit", "duty cycle", "switches/h", "coverage gaps"
    );

    let mut all_on = Stat::new("all on");
    let mut duty_only = Stat::new("duty ring only");
    let mut duty_first = Stat::new("duty first");
    let mut duty_first_p = Stat::new("duty first + prune");
    let mut greedy = Stat::new("greedy set cover");
    let mut exact = Stat::new("exact minimum");

    let (mut prev_df, mut prev_dfp, mut prev_g, mut prev_e) = (None, None, None, None);
    let (mut exact_solved, mut exact_better) = (0u64, 0u64);
    let mut t = 0.0;
    while t < ROTATION {
        let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
        let duty = duty_ring(&p, &c, t);

        let on = ActivationPlan { active: vec![true; fleet], lit: fleet, unservable: vec![] };
        all_on.record(&on, &None, covers_all(&cov, &on.active));

        let mut only = vec![false; fleet];
        for j in 0..c.sats_per_plane {
            only[satellite_index(&c, duty, j)] = true;
        }
        let only_plan = ActivationPlan { lit: c.sats_per_plane, active: only, unservable: vec![] };
        let ok = covers_all(&cov, &only_plan.active);
        duty_only.record(&only_plan, &None, ok);

        let df = duty_first_activation(&cov, &c, duty, prev_df.as_deref(), false);
        duty_first.record(&df, &prev_df, covers_all(&cov, &df.active));
        prev_df = Some(df.active);

        let dfp = duty_first_activation(&cov, &c, duty, prev_dfp.as_deref(), true);
        duty_first_p.record(&dfp, &prev_dfp, covers_all(&cov, &dfp.active));
        prev_dfp = Some(dfp.active);

        let g = select_active(&cov, fleet, 1, prev_g.as_deref());
        greedy.record(&g, &prev_g, covers_all(&cov, &g.active));
        prev_g = Some(g.active.clone());

        if let Some(e) = exact_activation(&cov, fleet, prev_e.as_deref(), 2_000_000) {
            exact_solved += 1;
            if e.lit < g.lit {
                exact_better += 1;
            }
            exact.record(&e, &prev_e, covers_all(&cov, &e.active));
            prev_e = Some(e.active);
        }

        t += step;
    }

    for s in [&all_on, &duty_only, &duty_first, &duty_first_p, &greedy, &exact] {
        s.print(fleet, hours);
    }

    println!(
        "\nThe exact search closed at {} of {} instants ({} of them strictly better\n\
         than greedy). It is affordable because the plan is a timetable computed on\n\
         the ground, not a decision made in orbit.",
        exact_solved, all_on.steps, exact_better
    );
    println!(
        "\n\"Duty ring only\" is listed to be refuted, not proposed: it leaves the band\n\
         uncovered at every instant sampled. Every other policy covers it completely."
    );

    // A dark satellite cannot take a handover the instant it is needed. Charge
    // each policy for lighting its satellites a warm-up ahead of service: the
    // set that must be on at t is the union of the plans over [t, t + lead].
    println!("\n\nWarm-up cost: satellites must be lit before they are needed.\n");
    println!(
        "{:>18} {:>14} {:>14} {:>14} {:>14}",
        "policy", "no lead", "2 min lead", "5 min lead", "10 min lead"
    );
    for (name, prune) in [("duty first + prune", true), ("duty first", false)] {
        let mut row = format!("{name:>18}");
        for lead in [0.0, 120.0, 300.0, 600.0] {
            let window = (lead / step) as usize + 1;
            let mut buf: Vec<Vec<bool>> = Vec::with_capacity(window);
            let (mut total, mut n) = (0u64, 0u64);
            let mut prev: Option<Vec<bool>> = None;
            let mut t = 0.0;
            while t < ROTATION {
                let cov = covering_satellites(&p, &c, &phases, &pts, MASK, t);
                let duty = duty_ring(&p, &c, t);
                let plan = duty_first_activation(&cov, &c, duty, prev.as_deref(), prune);
                prev = Some(plan.active.clone());
                buf.push(plan.active);
                if buf.len() > window {
                    buf.remove(0);
                }
                if buf.len() == window {
                    let lit = (0..fleet).filter(|&s| buf.iter().any(|b| b[s])).count();
                    total += lit as u64;
                    n += 1;
                }
                t += step;
            }
            row.push_str(&format!("{:>14.1}", total as f64 / n as f64));
        }
        println!("{row}");
    }
    println!(
        "\nA warm-up of a few minutes is the price of switching satellites off at all.\n\
         Even at ten minutes the fleet runs well under half lit."
    );
}
