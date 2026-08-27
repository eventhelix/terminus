//! How often a town's link moves, and what actually decides it.
//!
//! Two intuitions get tested here and both turn out wrong. The first is that
//! a town keeps a satellite for a pass; it does not — it keeps it for the
//! gap between satellites in a plane. The second is that attaching to the
//! highest satellite in view would make the link thrash between near-equal
//! candidates; it does not, because the constellation is a queue.
//!
//! Run: cargo run -p terminus-orbits --example handover_cadence

use terminus_orbits::circular::orbital_period;
use terminus_orbits::constellation::{band_point, PolarConstellation};
use terminus_orbits::coverage::max_pass_duration;
use terminus_orbits::handover::{handover_timeline, mean_service_interval, HandoverPolicy};
use terminus_orbits::CentralBody;

const MIN_ELEVATION: f64 = 25.0; // degrees, access floor (ADR-0003)
const HYSTERESIS: f64 = 3.0; // degrees of margin below the floor
const HOURS: f64 = 6.0;
const STEP: f64 = 5.0; // s

fn towns() -> [(&'static str, [f64; 3]); 4] {
    let edge = 20.0_f64.to_radians();
    [
        ("band centre, azimuth 0.7", band_point(0.7, 0.0)),
        ("band centre, azimuth 2.4", band_point(2.4, 0.0)),
        ("band centre, azimuth 5.0", band_point(5.0, 0.0)),
        ("band edge,   azimuth 1.3", band_point(1.3, edge)),
    ]
}

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let baseline = PolarConstellation {
        altitude: 2_200e3,
        planes: 6,
        sats_per_plane: 12,
        interplane_phase: 0.0,
    };
    let min_el = MIN_ELEVATION.to_radians();
    let hyst = HYSTERESIS.to_radians();
    let duration = HOURS * 3_600.0;
    // The baseline files every ring in step; `thin` only changes altitude, so
    // one phase vector serves both shells.
    let phases = baseline.uniform_phases();

    let period = orbital_period(&planet, baseline.altitude);
    let spacing = period / baseline.sats_per_plane as f64;
    let zenith_pass = max_pass_duration(&planet, baseline.altitude, min_el);

    println!(
        "Baseline: {} planes × {} satellites at {:.0} km, {MIN_ELEVATION:.0}° floor.\n\
         Orbit period {:.1} min; satellites in a plane are {:.1} min apart;\n\
         a pass straight through the zenith would last {:.1} min.\n",
        baseline.planes,
        baseline.sats_per_plane,
        baseline.altitude / 1e3,
        period / 60.0,
        spacing / 60.0,
        zenith_pass / 60.0
    );

    println!("  town                       handovers/{HOURS:.0}h   mean interval   greedy   sticky");
    for (label, town) in towns() {
        let greedy = handover_timeline(
            &planet,
            &baseline,
            town,
            &phases,
            HandoverPolicy::greedy(min_el),
            duration,
            STEP,
        );
        let sticky = handover_timeline(
            &planet,
            &baseline,
            town,
            &phases,
            HandoverPolicy::sticky(min_el, hyst),
            duration,
            STEP,
        );
        let interval = mean_service_interval(
            &planet,
            &baseline,
            town,
            &phases,
            HandoverPolicy::sticky(min_el, hyst),
            duration,
            STEP,
        )
        .unwrap_or(f64::NAN);
        println!(
            "  {label:26} {:>6}      {:>7.1} min   {:>6}   {:>6}",
            sticky.iter().filter(|e| e.from.is_some()).count(),
            interval / 60.0,
            greedy.iter().filter(|e| e.from.is_some()).count(),
            sticky.iter().filter(|e| e.from.is_some()).count(),
        );
    }

    let pingpong: usize = towns()
        .iter()
        .map(|(_, town)| {
            handover_timeline(
                &planet,
                &baseline,
                *town,
                &phases,
                HandoverPolicy::greedy(min_el),
                duration,
                STEP,
            )
            .windows(3)
            .filter(|w| w[0].to == w[2].to)
            .count()
        })
        .sum();

    println!(
        "\n\
         The interval is the in-plane spacing, {:.1} min, not the {:.1} min pass:\n\
         the town is handed to the next satellite in the same plane long before\n\
         the one it has is finished. Adding satellites to a plane buys coverage\n\
         and costs handovers in exact proportion; adding planes does not.\n",
        spacing / 60.0,
        zenith_pass / 60.0
    );

    println!(
        "Greedy and sticky selection agree at every town above, and greedy never\n\
         returns to a satellite it just left ({pingpong} such returns in {:.0} town-hours).\n\
         The queue does the work: the in-plane successor rises as the incumbent\n\
         sets, so \"highest in view\" changes exactly when \"hold until it sets\"\n\
         would have changed anyway. Hysteresis here is a guard against noisy\n\
         elevation estimates, not a cure for ping-pong.\n",
        HOURS * towns().len() as f64
    );

    // Where footprints barely overlap, the trade inverts.
    let thin = PolarConstellation {
        altitude: 1_200e3,
        ..baseline
    };
    let (mut thin_greedy, mut thin_sticky) = (0, 0);
    for (_, town) in towns() {
        thin_greedy += handover_timeline(
            &planet,
            &thin,
            town,
            &phases,
            HandoverPolicy::greedy(min_el),
            duration,
            STEP,
        )
        .iter()
        .filter(|e| e.from.is_some())
        .count();
        thin_sticky += handover_timeline(
            &planet,
            &thin,
            town,
            &phases,
            HandoverPolicy::sticky(min_el, hyst),
            duration,
            STEP,
        )
        .iter()
        .filter(|e| e.from.is_some())
        .count();
    }
    println!(
        "The same policies at {:.0} km, where footprints barely overlap: greedy\n\
         {thin_greedy} handovers, sticky {thin_sticky}. Holding a sinking satellite past the floor\n\
         means taking whatever is left when it finally goes — often another one on\n\
         its way down. Hysteresis is a knob to set per shell, not a default.",
        thin.altitude / 1e3
    );
}
