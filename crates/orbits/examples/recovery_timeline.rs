// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Two ways a conversation changes anchor, second by second.
//!
//!   A. the planned move: stream the working memory make-before-break, hold
//!      both anchors for one overlapped moment, flip -- the conversation
//!      never stalls
//!   B. the dead anchor: declare, replay the transcript from the terminal
//!      that has been holding it, and re-read the whole conversation before
//!      the next word
//!
//! The site's migration plate draws exactly these two lanes; the last block
//! prints the fixture it mirrors.
//!
//! Run: cargo run -p terminus-orbits --example recovery_timeline

use terminus_orbits::coverage::edge_slant_range;
use terminus_orbits::hill::{hill_radius, SUN_MU};
use terminus_orbits::placement::{
    edge_replay_time, one_way_light_time, prefill_time, replay_crossover_bps, shell_distance,
    transfer_time, KvCacheModel, PREFILL_TOKENS_PER_SECOND, TERMINAL_UPLINK_BPS,
    TRANSCRIPT_BYTES_PER_TOKEN,
};
use terminus_orbits::traffic::SessionProfile;
use terminus_orbits::CentralBody;

/// Keep-alive heartbeat interval (ms), restating ADR-0009's declaration rule:
/// an anchor is declared dead after `MISSED_BEATS` heartbeats of this spacing
/// go unanswered. Same figures `failure_timeline` uses for a telescope.
const HEARTBEAT_MS: f64 = 100.0;
/// Missed heartbeats before declaring, per ADR-0009.
const MISSED_BEATS: usize = 3;

/// Access shell and anchor shell, the two altitudes the replay path crosses.
const ACCESS_ALT: f64 = 2_200e3;
const MEO_ALT: f64 = 20_000e3;
/// Worst geometry, the one the first-token budget is argued at: the town at
/// the edge of its satellite's footprint, its anchor 60 degrees around the sky.
const WORST_SEPARATION_DEG: f64 = 60.0;

/// Fleet and load figures the burst below is counted against, from
/// `link_throughput` (TER-REQ-005 ceiling, 10% concurrent) and ADR-0014.
const SESSIONS_AT_CEILING: f64 = 100_000.0;
const ANCHORS: f64 = 24.0;
const ACCESS_SATELLITES: f64 = 72.0;
/// What a dark feeder telescope strands instead, for scale: 113 sessions of
/// working memory, 1.2 TB (`feeder_terminals`, section H).
const DARK_TELESCOPE_BURST_BYTES: f64 = 1.2e12;

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let model = KvCacheModel {
        layers: 80,
        query_heads: 64,
        kv_heads: 8,
        head_dim: 128,
        bytes_per_value: 2,
    };
    // The reference session `link_throughput` prices: a long tutoring
    // conversation, answered at twenty tokens a second.
    let profile = SessionProfile {
        tokens_per_second: 20.0,
        wire_bytes_per_token: 64.0,
        context_tokens: 32_768,
        duty_cycle: 0.30,
    };

    // ---- A. the planned move ----------------------------------------------
    let kv = profile.kv_bytes(&model);
    let stream_100 = transfer_time(kv, 100e9);
    let stream_10 = transfer_time(kv, 10e9);
    // The old anchor keeps answering while the copy is in flight, so the
    // successor is short by whatever was said meanwhile. That tail is sent
    // after the bulk and is the whole cost of the overlap.
    let catch_up_tokens = (profile.tokens_per_second * stream_100).ceil() as u64;
    let catch_up_bytes = model.bytes(catch_up_tokens);
    let catch_up_s = transfer_time(catch_up_bytes, 100e9);

    println!("A. The planned move: stream, overlap, flip\n");
    println!(
        "   working memory        {:>8.2} GB   ({} tokens x {:.0} KiB)",
        kv / 1e9,
        profile.context_tokens,
        model.bytes_per_token() / 1024.0
    );
    println!("   stream at 100 Gbps    {:>8.3} s", stream_100);
    println!("   stream at 10 Gbps     {:>8.2} s", stream_10);
    println!(
        "   answered meanwhile    {:>8} tokens at {:.0} tok/s: {:.1} MB, {:.2} ms to catch up",
        catch_up_tokens,
        profile.tokens_per_second,
        catch_up_bytes / 1e6,
        catch_up_s * 1e3
    );
    println!(
        "   stall                 {:>8.0} s   the old anchor answers until the flip",
        0.0
    );
    println!("   lost                  nothing\n");
    println!(
        "   The successor already holds the frozen mind -- the weights are the\n\
         \x20  same for every conversation and were copied at leisure. What moves\n\
         \x20  is the working memory, and it moves while the old anchor is still\n\
         \x20  talking. For one overlapped moment both hold the session; then the\n\
         \x20  routing flips. No token is ever late.\n"
    );

    // ---- B. the dead anchor ----------------------------------------------
    //
    // Nothing streams off a dead machine, so the successor has to be handed
    // the conversation as text and read it. The durable copy is the
    // terminal's own: it sent every question and received every answer, so it
    // already holds the transcript and costs the network nothing to keep it
    // (ADR-0030). The path back is the ordinary access path -- radio up to the
    // serving satellite, feeder link on to the successor -- priced here at the
    // worst geometry the first-token budget uses.
    let min_elevation = 25.0_f64.to_radians();
    let radio_leg = one_way_light_time(edge_slant_range(&planet, ACCESS_ALT, min_elevation));
    let feeder_leg = one_way_light_time(shell_distance(
        &planet,
        ACCESS_ALT,
        MEO_ALT,
        WORST_SEPARATION_DEG.to_radians(),
    ));
    let edge_one_way = radio_leg + feeder_leg;

    let declare = MISSED_BEATS as f64 * HEARTBEAT_MS / 1e3;
    let transcript_bytes = profile.context_tokens as f64 * TRANSCRIPT_BYTES_PER_TOKEN;
    let upload_s = transfer_time(transcript_bytes, TERMINAL_UPLINK_BPS);
    let replay_s = edge_replay_time(transcript_bytes, TERMINAL_UPLINK_BPS, edge_one_way);
    let prefill = prefill_time(profile.context_tokens, PREFILL_TOKENS_PER_SECOND);
    let replayed_at = declare + replay_s;
    let stall = replayed_at + prefill;

    println!("B. The dead anchor: declare, replay, prefill\n");
    println!("   t = 0             the anchor goes dark; its working memory dies with it");
    println!(
        "   t = {:.1} s         declared: {} heartbeats of {:.0} ms go unanswered",
        declare, MISSED_BEATS, HEARTBEAT_MS
    );
    println!(
        "   t = {:.3} s       transcript replayed from the terminal: {:.1} ms of radio\n\
         \x20                    + {:.1} ms of feeder each way, {:.0} kB up at {:.0} Mbps\n\
         \x20                    = {:.0} ms",
        replayed_at,
        radio_leg * 1e3,
        feeder_leg * 1e3,
        transcript_bytes / 1e3,
        TERMINAL_UPLINK_BPS / 1e6,
        upload_s * 1e3,
    );
    println!(
        "   t = {:.3} s       prefill done: {} tokens at {:.0} tok/s = {:.3} s\n\
         \x20                    (stated, placement::PREFILL_TOKENS_PER_SECOND)",
        stall, profile.context_tokens, PREFILL_TOKENS_PER_SECOND, prefill
    );
    println!("   stall             {:>8.2} s", stall);
    println!(
        "   lost              the last exchange: whatever was said after the terminal's\n\
         \x20                    last complete exchange (TER-REQ-014)\n"
    );

    // ---- B2. the trade that put the transcript there ----------------------
    //
    // The rejected architecture kept the durable copy on a spacecraft at the
    // balance points. Its advantage was a fat laser link at the far end; its
    // disadvantage was being half a light second away. One uplink rate trades
    // one for the other, and every terminal is on the winning side of it.
    let l12 = hill_radius(&planet, 0.122 * SUN_MU, 7.2555e9);
    let remote_one_way = one_way_light_time(l12);
    let remote_fetch = 2.0 * remote_one_way + transfer_time(transcript_bytes, 100e9);
    let crossover = replay_crossover_bps(transcript_bytes, remote_one_way, edge_one_way);

    println!("B2. Against the rejected store at the balance points\n");
    println!(
        "   store at L1/L2    {:.0} km, {:.3} s of light each way: {:.3} s to fetch",
        l12 / 1e3,
        remote_one_way,
        remote_fetch
    );
    println!(
        "   terminal          {:.1} ms each way: {:.3} s to replay at {:.0} Mbps",
        edge_one_way * 1e3,
        replay_s,
        TERMINAL_UPLINK_BPS / 1e6
    );
    println!(
        "   crossover         {:.2} Mbps of uplink -- below it the balance points win,\n\
         \x20                    above it the terminal does. The conversation itself costs\n\
         \x20                    {:.1} kbps, so the crossover is {:.0}x the rate a terminal\n\
         \x20                    must already sustain to hold the conversation at all.\n",
        crossover / 1e6,
        profile.conversational_bps() / 1e3,
        crossover / profile.conversational_bps(),
    );

    println!("   stall by uplink rate:\n");
    println!(
        "{:>14} {:>12} {:>12} {:>12}",
        "uplink", "upload", "replay", "stall"
    );
    for rate in [1e6, TERMINAL_UPLINK_BPS, 100e6] {
        let r = edge_replay_time(transcript_bytes, rate, edge_one_way);
        println!(
            "{:>11.0} Mbps {:>10.0} ms {:>10.0} ms {:>10.2} s{}",
            rate / 1e6,
            transfer_time(transcript_bytes, rate) * 1e3,
            r * 1e3,
            declare + r + prefill,
            if rate == TERMINAL_UPLINK_BPS {
                "  <- stated"
            } else {
                ""
            }
        );
    }
    println!(
        "\n   The prefill dominates every row: rebuilding {} tokens of working\n\
         \x20  memory costs {:.2} s at the stated rate, against {:.0} ms of getting\n\
         \x20  the text there. Recovery is bounded by re-reading, not by fetching,\n\
         \x20  which is why where the text was kept was never worth a spacecraft.\n",
        profile.context_tokens,
        prefill,
        replay_s * 1e3,
    );

    // ---- B3. the burst a dead anchor makes on the radio --------------------
    //
    // One dead anchor does not replay one session. Every session it held
    // replays at once, and all of them come up through the access shell.
    let stranded = SESSIONS_AT_CEILING / ANCHORS;
    let per_access_sat = stranded / ACCESS_SATELLITES;
    let burst_bytes = per_access_sat * transcript_bytes;
    println!("B3. The burst, at the million-terminal ceiling\n");
    println!(
        "   {:.0} sessions concurrent over {:.0} anchors: {:.0} strand at once,\n\
         \x20  spread across {:.0} access satellites = {:.0} sessions and {:.1} MB of\n\
         \x20  transcript on one satellite's radio.\n",
        SESSIONS_AT_CEILING,
        ANCHORS,
        stranded,
        ACCESS_SATELLITES,
        per_access_sat,
        burst_bytes / 1e6,
    );
    println!(
        "   That is the whole of what moving durability to the ground costs the\n\
         \x20  space segment: {:.0} orders of magnitude under the {:.1} TB a dark feeder\n\
         \x20  telescope moves (feeder_terminals, section H): a transcript is text, and\n\
         \x20  that burst is working memory. What it is not is free at the terminal\n\
         \x20  end: an access satellite's radio capacity is not modelled anywhere in\n\
         \x20  this simulator, so the time to drain this burst is an open item,\n\
         \x20  declared in ADR-0030 rather than computed.\n",
        (DARK_TELESCOPE_BURST_BYTES / burst_bytes).log10(),
        DARK_TELESCOPE_BURST_BYTES / 1e12,
    );

    // ---- fixture ----------------------------------------------------------
    println!("Fixture for static/scripts/migration-plate-data.js (copy verbatim):\n");
    println!("  contextTokens   {}", profile.context_tokens);
    println!("  kvBytes         {:.0}", kv);
    println!("  streamS100      {:.4}", stream_100);
    println!("  streamS10       {:.3}", stream_10);
    println!("  tokensPerS      {:.0}", profile.tokens_per_second);
    println!("  catchUpTokens   {}", catch_up_tokens);
    println!("  catchUpMs       {:.3}", catch_up_s * 1e3);
    println!("  heartbeatMs     {:.0}", HEARTBEAT_MS);
    println!("  missedBeats     {}", MISSED_BEATS);
    println!("  declareS        {:.1}", declare);
    println!("  edgeOneWayS     {:.4}", edge_one_way);
    println!("  radioLegMs      {:.1}", radio_leg * 1e3);
    println!("  feederLegMs     {:.1}", feeder_leg * 1e3);
    println!("  uplinkBps       {:.0}", TERMINAL_UPLINK_BPS);
    println!("  transcriptBytes {:.0}", transcript_bytes);
    println!("  uploadS         {:.4}", upload_s);
    println!("  replayS         {:.4}", replay_s);
    println!("  prefillTps      {:.0}", PREFILL_TOKENS_PER_SECOND);
    println!("  prefillS        {:.4}", prefill);
    println!("  stallS          {:.4}", stall);
}
