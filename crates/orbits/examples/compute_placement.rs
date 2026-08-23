//! Where should LLM inference live for a tidally locked reference planet?
//! Light-time prices for each candidate anchor location, session-anchor
//! arithmetic (access dwell vs compute dwell), and KV-cache size and
//! migration costs for the reference model.
//!
//! Run: cargo run -p helixsim-orbits --example compute_placement

use helixsim_orbits::circular::orbital_period;
use helixsim_orbits::coverage::{edge_slant_range, max_pass_duration};
use helixsim_orbits::hill::{hill_radius, SUN_MU};
use helixsim_orbits::placement::{
    one_way_light_time, shell_distance, transfer_time, KvCacheModel,
};
use helixsim_orbits::CentralBody;

const ACCESS_ALT: f64 = 2_200e3;
const MEO_ALT: f64 = 20_000e3;
const ACCESS_SATS_PER_PLANE: usize = 12; // ADR-0003 baseline ring

fn ms(seconds: f64) -> f64 {
    seconds * 1e3
}

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let min_elevation = 25.0_f64.to_radians();

    let user_leg = one_way_light_time(edge_slant_range(&planet, ACCESS_ALT, min_elevation));
    println!("User to access satellite (2,200 km, edge of footprint): {:.1} ms one way\n", ms(user_leg));

    println!("Anchor candidates:");
    for (label, sep_deg) in [("MEO 20,000 km, overhead", 0.0), ("MEO 20,000 km, 30° away", 30.0), ("MEO 20,000 km, 60° away", 60.0)] {
        let d = shell_distance(&planet, ACCESS_ALT, MEO_ALT, (sep_deg as f64).to_radians());
        println!(
            "  {label:<28} {:>8.0} km  {:>6.1} ms one way from access",
            d / 1e3,
            ms(one_way_light_time(d))
        );
    }
    let l12 = hill_radius(&planet, 0.122 * SUN_MU, 7.2555e9);
    println!(
        "  {:<28} {:>8.0} km  {:>6.2} s one way from planet",
        "L1/L2 balance points", l12 / 1e3, one_way_light_time(l12)
    );
    println!(
        "  {:<28} {:>8.2e} km {:>6.1} s one way from planet",
        "L4/L5 (orbital radius)", 7.2555e6, one_way_light_time(7.2555e9)
    );

    let worst = shell_distance(&planet, ACCESS_ALT, MEO_ALT, 60.0_f64.to_radians());
    let rtt = 2.0 * (user_leg + one_way_light_time(worst));
    println!(
        "\nFirst-token propagation budget, MEO anchor (worst geometry):\n\
         \x20 2 x ({:.1} + {:.1}) ms = {:.0} ms round trip, leaving {:.0} ms of\n\
         \x20 thinking time inside the RFP's 300 ms budget.",
        ms(user_leg),
        ms(one_way_light_time(worst)),
        ms(rtt),
        300.0 - ms(rtt)
    );

    let access_dwell = max_pass_duration(&planet, ACCESS_ALT, min_elevation);
    let meo_dwell = max_pass_duration(&planet, MEO_ALT, min_elevation);
    // A link does not last a pass: the town is handed to the next satellite in
    // the same plane after one in-plane spacing (ADR-0015). That interval, not
    // the pass, is what an anchored session has to ride out.
    let handover_interval = orbital_period(&planet, ACCESS_ALT) / ACCESS_SATS_PER_PLANE as f64;
    println!(
        "
Session-anchor arithmetic:
           access pass, best case:  {:>6.1} min
           access handover every:   {:>6.1} min   (period / satellites per plane)
           MEO pass, best case:     {:>6.1} min
           access handovers survived by one anchored session: ~{:.0}",
        access_dwell / 60.0,
        handover_interval / 60.0,
        meo_dwell / 60.0,
        meo_dwell / handover_interval
    );

    let model = KvCacheModel {
        layers: 80,
        kv_heads: 8,
        head_dim: 128,
        bytes_per_value: 2,
    };
    println!(
        "\nReference model working memory (KV cache), {:.0} KiB per token:",
        model.bytes_per_token() / 1024.0
    );
    println!(
        "{:>16} {:>10} {:>14} {:>14}",
        "context (tok)", "size (GB)", "@10 Gbps (s)", "@100 Gbps (s)"
    );
    for tokens in [8_192_u64, 32_768, 131_072] {
        let bytes = model.bytes(tokens);
        println!(
            "{:>16} {:>10.1} {:>14.1} {:>14.2}",
            tokens,
            bytes / 1e9,
            transfer_time(bytes, 10e9),
            transfer_time(bytes, 100e9)
        );
    }
}
