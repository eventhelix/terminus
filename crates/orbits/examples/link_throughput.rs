// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

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
//! The sting is in section C. Nothing forces a session to change anchor -- a
//! ring reaches every anchor at every instant -- so how much working memory
//! the backbone carries is set by the re-anchor margin, a policy number, and
//! it moves the answer across four orders of magnitude.
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

/// How often a session changes anchor, per margin, from `feeder_terminals`
/// section H. These are policy outcomes, not geometry: a ring can reach every
/// anchor at every instant, so nothing forces a migration and the re-anchor
/// margin decides how many happen. Keep in step with that example.
const POLICY: [(f64, f64); 6] = [
    (0.0, 113.37),
    (2_500e3, 19.13),
    (5_000e3, 12.70),
    (10_000e3, 7.08),
    (20_000e3, 4.05),
    (25_000e3, 0.0),
];
/// Index into `POLICY` of the margin the library states as `REANCHOR_MARGIN`.
const CHOSEN: usize = 2;

/// Seconds between migrations at a given rate per session per day. An infinite
/// interval is a session that never moves.
fn dwell_from(changes_per_day: f64) -> f64 {
    if changes_per_day <= 0.0 {
        f64::INFINITY
    } else {
        86_400.0 / changes_per_day
    }
}

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
        "   one migration costs   {:>9.2} s of a 100 Gbps link ({:.1} s at 10)",
        transfer_time(profile.kv_bytes(&model), 100e9),
        transfer_time(profile.kv_bytes(&model), 10e9)
    );
    println!(
        "\n   The two flows are not the same order of magnitude, and how far\n\
         \x20  apart they end up is decided by how often a session migrates -- not\n\
         \x20  by anything the users do:\n"
    );
    println!(
        "{:>22} {:>16} {:>14}",
        "if a session moved", "migration rate", "vs conversation"
    );
    for (label, hours) in [
        ("every 2 h", 2.0),
        ("every 6 h", 6.0),
        ("once a day", 24.0),
        ("never", f64::INFINITY),
    ] {
        let dwell = hours * 3_600.0;
        let ratio = profile.migration_ratio(&model, dwell);
        println!(
            "{:>22} {:>16} {:>13}x",
            label,
            gbps(profile.migration_bps(&model, dwell)),
            ratio.round()
        );
    }
    println!(
        "\n   At the top of that table the backbone is a memory-moving network\n\
         \x20  that also carries speech, and sizing it from token rates would be\n\
         \x20  wrong by three orders of magnitude. At the bottom it carries speech\n\
         \x20  and nothing else. Section C is about which end the policy chooses,\n\
         \x20  and the default chooses neither. The bottom of this table is only\n\
         \x20  reachable by holding an anchor so hard that the path it leaves you\n\
         \x20  holding has no time left in it to think."
    );

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

        let dwell = dwell_from(POLICY[CHOSEN].1);
        let radio = link_load(per_access_sat, &profile, &model, dwell, 0.0);
        let necklace = link_load(per_necklace, &profile, &model, dwell, 2.0);
        let feeder = link_load(per_feeder, &profile, &model, dwell, 2.0);

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
            "\n   Busiest link total: {}. The default policy moves working memory in\n\
         \x20  steady state, and even so the mean is not what sizes this link:\n\
         \x20  {:.1} GB moves in {:.2} s on a 100 Gbps link and {:.1} s on 10 Gbps,\n\
         \x20  and one failed telescope strands a whole bucket of sessions at once.\n\
         \x20  The burst sizes the link; the mean only says what it carries in\n\
         \x20  between.",
            gbps(feeder.total()),
            profile.kv_bytes(&model) / 1e9,
            transfer_time(profile.kv_bytes(&model), 100e9),
            transfer_time(profile.kv_bytes(&model), 10e9)
        );
    }

    println!("\n\nC. What the re-anchor margin costs the backbone\n");
    println!(
        "   Nothing forces a migration: a ring reaches every anchor at every\n\
         \x20  instant, so a session could hold one for ever. The re-anchor margin\n\
         \x20  decides how many happen, and because working memory outweighs\n\
         \x20  conversation by three orders of magnitude, that one policy number\n\
         \x20  sizes the entire backbone.\n"
    );
    let sessions = TERMINALS_CEILING * CONCURRENCY;
    let per_feeder = sessions / (RINGS * SATS_PER_RING * 2.0);
    println!(
        "   At the million-terminal ceiling, {:.0} sessions, busiest feeder link:\n",
        sessions
    );
    println!(
        "{:>14} {:>14} {:>16} {:>16}",
        "margin (km)", "changes/day", "conversation", "migration"
    );
    for (i, (margin, per_day)) in POLICY.iter().enumerate() {
        let load = link_load(per_feeder, &profile, &model, dwell_from(*per_day), 2.0);
        println!(
            "{:>14.0} {:>14.2} {:>16} {:>16}{}",
            margin / 1e3,
            per_day,
            gbps(load.conversational),
            gbps(load.migration),
            if i == CHOSEN { "  <- chosen" } else { "" }
        );
    }
    println!(
        "\n   The span is four orders of magnitude, and neither end of it can be\n\
         \x20  bought. Chasing the shortest path costs more backbone than a hundred\n\
         \x20  gigabit link can carry. The free row at the bottom is free only of\n\
         \x20  bandwidth: holding an anchor until nothing ever beats it means\n\
         \x20  holding a p95 round trip of 290 ms, which leaves 10 ms of the RFP's\n\
         \x20  300 ms to think in.\n\n\
         \x20  So the honest reading is that this network is sized by a policy\n\
         \x20  decision, not by its users. Latency and bandwidth are being traded\n\
         \x20  directly against each other, and the exchange rate is brutal: the\n\
         \x20  whole span of mean path length across these six policies is\n\
         \x20  18,569 km to 28,385 km -- 33 ms one way -- for a backbone bill that\n\
         \x20  varies by ten thousand fold. The default sits where both currencies\n\
         \x20  are still affordable, and where it sits is the only judgement in\n\
         \x20  this example rather than a measurement."
    );
    println!(
        "\n\nD. What this does and does not settle\n\n\
         \x20  Concurrency is a guess at {:.0}%, and every number above scales\n\
         \x20  linearly in it. Context length is the other lever and it is worse\n\
         \x20  than linear in consequence: at 131,072 tokens the working memory is\n\
         \x20  {:.0} GB, four times the figure used here.\n\n\
         \x20  Uniform towns is the assumption doing the quiet work. Real settlements\n\
         \x20  cluster, and a clustered band would load a few links far harder than\n\
         \x20  this arithmetic suggests while leaving others idle. The averages here\n\
         \x20  are a floor on the busiest link, never a description of it.\n\n\
         \x20  Load balancing is still not bought. A session moves to shorten its\n\
         \x20  own path and never to spare an anchor's compute, so nothing here\n\
         \x20  notices sessions piling onto whichever anchor happens to sit over a\n\
         \x20  crowded stretch of the band. The margin is the only lever that\n\
         \x20  exists and it is a blunt one -- it moves every session at once --\n\
         \x20  which is exactly why it has to be tunable in flight rather than\n\
         \x20  fixed at launch.\n\n\
         \x20  What it does buy is a bill for a feature. Streaming working memory\n\
         \x20  from one anchor to another -- make-before-break, the {:.0} GB in\n\
         \x20  {:.2} s that section A prices -- has a steady-state customer at this\n\
         \x20  setting: every session moves {:.2} times a day, which at the ceiling\n\
         \x20  is better than a million migrations a day, each one inside a live\n\
         \x20  conversation that must not stall. Re-reading the transcript from the\n\
         \x20  vault answers a *failed* anchor, where there is nothing left to\n\
         \x20  stream from, but it cannot answer a planned one: it costs a full\n\
         \x20  prefill in the middle of a sentence. So context transfer is\n\
         \x20  first-release work rather than a later block, and that is what the\n\
         \x20  thinking time in section C was bought with.",
        CONCURRENCY * 100.0,
        model.bytes(131_072) / 1e9,
        profile.kv_bytes(&model) / 1e9,
        transfer_time(profile.kv_bytes(&model), 100e9),
        POLICY[CHOSEN].1
    );
}
