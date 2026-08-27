//! What has to flow through each class of link, and which flow decides the
//! sizing.
//!
//! Two traffic types share the backbone. The conversation is a token stream —
//! a few dozen small packets a second, and only while the model is actually
//! answering. The working memory behind it is eleven gigabytes for a long
//! session, and every anchor change moves all of it.
//!
//! With towns spread uniformly around the band, the session count divides
//! evenly enough that the per-link answer is arithmetic rather than
//! simulation: the topology already fixed how many links there are and how
//! many sessions ride the busiest of them.
//!
//! Run: cargo run --release -p terminus-orbits --example link_throughput

use terminus_orbits::placement::{transfer_time, KvCacheModel};
use terminus_orbits::traffic::{link_load, SessionProfile};

/// TER-REQ-005: ten thousand terminals at first light, one million without
/// redesign.
const TERMINALS_FIRST_LIGHT: f64 = 10_000.0;
const TERMINALS_CEILING: f64 = 1_000_000.0;

/// Fraction of terminals with a conversation in progress at any instant. A
/// guess, and the load scales linearly in it, so it is stated rather than
/// buried.
const CONCURRENCY: f64 = 0.10;

/// An anchor holds a session about three and a half hours — the MEO pass of
/// 206.6 min from `compute_placement`, which is also the ~19 access handovers
/// one anchored session rides out.
const ANCHOR_DWELL: f64 = 3.4 * 3_600.0;

/// Share of sessions that leave their ring through a borrowed telescope
/// rather than their own satellite's. Pooling left the median access
/// satellite using one of its two feeder links, so most do not borrow; this
/// is a stated guess for the rest.
const BORROW_SHARE: f64 = 0.25;

/// The wheel: 6 rings of 12. The shell: 24 anchors, 6 feeder telescopes each.
const RINGS: f64 = 6.0;
const SATS_PER_RING: f64 = 12.0;
const ANCHORS: f64 = 24.0;

fn gbps(bps: f64) -> String {
    if bps >= 1e9 {
        format!("{:.1} Gbps", bps / 1e9)
    } else if bps >= 1e6 {
        format!("{:.1} Mbps", bps / 1e6)
    } else {
        format!("{:.1} kbps", bps / 1e3)
    }
}

fn main() {
    let model = KvCacheModel {
        layers: 80,
        kv_heads: 8,
        head_dim: 128,
        bytes_per_value: 2,
    };
    let profile = SessionProfile {
        tokens_per_second: 20.0,
        wire_bytes_per_token: 64.0,
        context_tokens: 32_768,
        duty_cycle: 0.30,
    };

    println!("A. One conversation\n");
    println!(
        "   token stream          {:>3.0} tokens/s at {:.0} B on the wire, {:.0}% duty",
        profile.tokens_per_second,
        profile.wire_bytes_per_token,
        profile.duty_cycle * 100.0
    );
    println!(
        "   conversational rate   {:>12}",
        gbps(profile.conversational_bps())
    );
    println!(
        "   working memory        {:>9.1} GB  ({} tokens of context)",
        profile.kv_bytes(&model) / 1e9,
        profile.context_tokens
    );
    println!(
        "   anchor dwell          {:>9.1} h   so one migration every {:.1} h",
        ANCHOR_DWELL / 3_600.0,
        ANCHOR_DWELL / 3_600.0
    );
    println!(
        "   migration, averaged   {:>12}",
        gbps(profile.migration_bps(&model, ANCHOR_DWELL))
    );
    println!(
        "\n   Ratio: working memory outweighs the conversation it belongs to by\n\
         \x20  {:.0} to one. A backbone sized from token rates would be wrong by that\n\
         \x20  factor, not by a margin. Everything below is really a memory-moving\n\
         \x20  network that also carries speech.",
        profile.migration_ratio(&model, ANCHOR_DWELL)
    );
    println!(
        "\n   The saving grace is that most conversations never migrate at all:\n\
         \x20  a session must outlive the anchor's hold to move even once."
    );
    for (label, secs) in [
        ("a 10-minute question", 600.0),
        ("a 1-hour lesson", 3_600.0),
        ("a 6-hour working day", 6.0 * 3_600.0),
    ] {
        println!(
            "     {:<22} {:>5.2} migrations",
            label,
            profile.migrations_per_session(secs, ANCHOR_DWELL)
        );
    }

    for (fleet_label, terminals) in [
        ("first light (TER-REQ-005)", TERMINALS_FIRST_LIGHT),
        ("the million-terminal ceiling", TERMINALS_CEILING),
    ] {
        let sessions = terminals * CONCURRENCY;
        println!(
            "\n\nB. {} \u{2014} {:.0} terminals, {:.0}% concurrent = {:.0} sessions\n",
            fleet_label,
            terminals,
            CONCURRENCY * 100.0,
            sessions
        );

        // Uniform towns, so sessions divide evenly over the links that exist.
        // A radio link carries no migration: the terminal is never told its
        // mind moved. A feeder link carries each migration twice, because with
        // no links in the shell the memory goes down through the wheel and
        // back up again.
        let per_access_sat = sessions / (RINGS * SATS_PER_RING);
        // Every feeder link has one end on the wheel and one on the shell, so
        // the two sides must agree on how many there are -- and they do:
        // 72 satellites x 2 telescopes = 24 anchors x 6 = 144. A useful check
        // that the topology closes.
        let wheel_ends = RINGS * SATS_PER_RING * 2.0;
        let shell_ends = ANCHORS * RINGS;
        assert_eq!(
            wheel_ends, shell_ends,
            "feeder telescopes must pair up: {wheel_ends} on the wheel, {shell_ends} on the shell"
        );
        let per_feeder = sessions / wheel_ends;
        // A session only crosses the necklace when it borrows a ring mate's
        // telescope. Pooling put the median satellite at one feeder link in
        // use of its two, so most sessions leave through their own satellite;
        // this is the share that does not.
        let per_necklace = sessions * BORROW_SHARE / (RINGS * SATS_PER_RING);

        let radio = link_load(per_access_sat, &profile, &model, ANCHOR_DWELL, 0.0);
        let necklace = link_load(per_necklace, &profile, &model, ANCHOR_DWELL, 2.0);
        let feeder = link_load(per_feeder, &profile, &model, ANCHOR_DWELL, 2.0);

        println!(
            "{:<36} {:>9} {:>12} {:>12}",
            "link", "sessions", "conversation", "migration"
        );
        for (label, n, load) in [
            ("radio, terminal to satellite", per_access_sat, radio),
            ("necklace, borrowed traffic only", per_necklace, necklace),
            ("feeder (144 of them, both ends)", per_feeder, feeder),
        ] {
            println!(
                "{:<36} {:>9.0} {:>12} {:>12}",
                label,
                n,
                gbps(load.conversational),
                gbps(load.migration)
            );
        }
        println!(
            "\n   Busiest link total: {}. A migration itself is a burst, not a\n\
         \x20  trickle: {:.1} GB moves in {:.2} s on a 100 Gbps link and {:.1} s on\n\
         \x20  10 Gbps, so the link must be sized for the burst as well as the mean.",
            gbps(feeder.total()),
            profile.kv_bytes(&model) / 1e9,
            transfer_time(profile.kv_bytes(&model), 100e9),
            transfer_time(profile.kv_bytes(&model), 10e9)
        );
    }

    println!(
        "\n\nC. What this does and does not settle\n\n\
         \x20  Concurrency is a guess at {:.0}%, and every number above scales\n\
         \x20  linearly in it. Context length is the other lever and it is worse\n\
         \x20  than linear in consequence: at 131,072 tokens the working memory is\n\
         \x20  {:.0} GB, four times the figure used here.\n\n\
         \x20  Uniform towns is the assumption doing the quiet work. Real settlements\n\
         \x20  cluster, and a clustered band would load a few links far harder than\n\
         \x20  this arithmetic suggests while leaving others idle. The averages here\n\
         \x20  are a floor on the busiest link, never a description of it.",
        CONCURRENCY * 100.0,
        model.bytes(131_072) / 1e9
    );
}
