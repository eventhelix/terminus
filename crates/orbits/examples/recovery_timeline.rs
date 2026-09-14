// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Two ways a conversation changes anchor, second by second.
//!
//!   A. the planned move: stream the working memory make-before-break, hold
//!      both anchors for one overlapped moment, flip -- the conversation
//!      never stalls
//!   B. the dead anchor: declare, fetch the transcript from the vault, and
//!      re-read the whole conversation before the next word
//!
//! The site's migration plate draws exactly these two lanes; the last block
//! prints the fixture it mirrors.
//!
//! Run: cargo run -p terminus-orbits --example recovery_timeline

use terminus_orbits::hill::{hill_radius, SUN_MU};
use terminus_orbits::placement::{
    one_way_light_time, prefill_time, transfer_time, KvCacheModel, PREFILL_TOKENS_PER_SECOND,
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

fn main() {
    let planet = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
    let model = KvCacheModel {
        layers: 80,
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
    println!("   stall                 {:>8.0} s   the old anchor answers until the flip", 0.0);
    println!("   lost                  nothing\n");
    println!(
        "   The successor already holds the frozen mind -- the weights are the\n\
         \x20  same for every conversation and were copied at leisure. What moves\n\
         \x20  is the working memory, and it moves while the old anchor is still\n\
         \x20  talking. For one overlapped moment both hold the session; then the\n\
         \x20  routing flips. No token is ever late.\n"
    );

    // ---- B. the dead anchor -----------------------------------------------
    let l12 = hill_radius(&planet, 0.122 * SUN_MU, 7.2555e9);
    let vault_one_way = one_way_light_time(l12);
    let declare = MISSED_BEATS as f64 * HEARTBEAT_MS / 1e3;
    let transcript_bytes = profile.context_tokens as f64 * TRANSCRIPT_BYTES_PER_TOKEN;
    let transcript_s = transfer_time(transcript_bytes, 100e9);
    let prefill = prefill_time(profile.context_tokens, PREFILL_TOKENS_PER_SECOND);
    let fetched_at = declare + 2.0 * vault_one_way + transcript_s;
    let stall = fetched_at + prefill;

    println!("B. The dead anchor: declare, fetch, prefill\n");
    println!("   t = 0             the anchor goes dark; its working memory dies with it");
    println!(
        "   t = {:.1} s         declared: {} heartbeats of {:.0} ms go unanswered",
        declare, MISSED_BEATS, HEARTBEAT_MS
    );
    println!(
        "   t = {:.3} s       transcript back from the vault: {:.0} km away,\n\
         \x20                    {:.3} s of light each way, {:.0} kB in {:.0} us",
        fetched_at,
        l12 / 1e3,
        vault_one_way,
        transcript_bytes / 1e3,
        transcript_s * 1e6
    );
    println!(
        "   t = {:.3} s       prefill done: {} tokens at {:.0} tok/s = {:.3} s\n\
         \x20                    (stated, placement::PREFILL_TOKENS_PER_SECOND)",
        stall, profile.context_tokens, PREFILL_TOKENS_PER_SECOND, prefill
    );
    println!("   stall             {:>8.2} s", stall);
    println!("   lost              the last exchange: whatever was said after the vault's last copy\n");
    println!(
        "   Nothing can be streamed from a machine that is gone. The vault at\n\
         \x20  L1/L2 holds the transcript, half a light-second away, and the new\n\
         \x20  anchor rebuilds the working memory by reading it -- a full prefill\n\
         \x20  in the middle of a sentence. That is the right price for a failure\n\
         \x20  and the wrong price for a planned move, which is why the planned\n\
         \x20  move streams instead (ADR-0022).\n"
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
    println!("  vaultKm         {:.0}", l12 / 1e3);
    println!("  vaultOneWayS    {:.4}", vault_one_way);
    println!("  transcriptBytes {:.0}", transcript_bytes);
    println!("  transcriptS     {:.3e}", transcript_s);
    println!("  prefillTps      {:.0}", PREFILL_TOKENS_PER_SECOND);
    println!("  prefillS        {:.4}", prefill);
    println!("  stallS          {:.4}", stall);
}
