// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Compute-placement arithmetic: distances and light times between orbital
//! shells, and the size and transfer cost of LLM inference state.

use crate::CentralBody;

/// Speed of light in vacuum, m/s.
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;

/// Straight-line distance (m) between two satellites on circular shells at
/// `alt1` and `alt2`, separated by central angle `separation` (rad) as seen
/// from the body center.
pub fn shell_distance(body: &CentralBody, alt1: f64, alt2: f64, separation: f64) -> f64 {
    let r1 = body.radius + alt1;
    let r2 = body.radius + alt2;
    (r1 * r1 + r2 * r2 - 2.0 * r1 * r2 * separation.cos()).sqrt()
}

/// One-way light travel time (s) over `distance` (m).
pub fn one_way_light_time(distance: f64) -> f64 {
    distance / SPEED_OF_LIGHT
}

/// One-way latency (s) over `distance` (m) through `relays` satellites.
///
/// Light is not the whole delay. Every satellite that forwards a packet
/// rather than originating or terminating it has to receive the frame, decode
/// its error correction, look up where it goes, re-encode and re-transmit, and
/// that work costs `relay_delay` seconds each time. See
/// [`crate::routing::RELAY_DELAY`] for the figure and for how much of a guess
/// it is.
///
/// Count the satellites that *forward*: the access satellite a town is talking
/// to, plus one for each necklace hop. The anchor at the far end terminates the
/// packet and is not a relay.
pub fn one_way_latency(distance: f64, relays: usize, relay_delay: f64) -> f64 {
    one_way_light_time(distance) + relays as f64 * relay_delay
}

/// Per-token key/value-cache footprint of a transformer model: for every
/// token processed, each layer stores a key and a value vector per KV head.
///
/// The reference model is the 70-billion-parameter class (the shape of
/// Llama 3 70B): 80 layers and grouped-query attention, 64 query heads sharing
/// 8 key/value heads of 128 numbers each. Only the key/value heads are cached,
/// so `query_heads` never enters [`Self::bytes_per_token`]; it records what
/// the sharing saves, [`Self::unshared_bytes_per_token`].
#[derive(Debug, Clone, Copy)]
pub struct KvCacheModel {
    pub layers: usize,
    /// Heads that ask: each forms a query from the new token. Not cached.
    pub query_heads: usize,
    /// Heads that are cached. `query_heads` must be a whole multiple of this.
    pub kv_heads: usize,
    pub head_dim: usize,
    pub bytes_per_value: usize,
}

impl KvCacheModel {
    /// Bytes of KV cache appended per token of context.
    pub fn bytes_per_token(&self) -> f64 {
        (2 * self.layers * self.kv_heads * self.head_dim * self.bytes_per_value) as f64
    }

    /// Query heads sharing each cached key/value head (grouped-query attention).
    pub fn queries_per_kv_head(&self) -> usize {
        self.query_heads / self.kv_heads
    }

    /// Bytes per token the cache would take if every query head kept keys and
    /// values of its own -- the footprint grouped-query attention avoids.
    pub fn unshared_bytes_per_token(&self) -> f64 {
        (2 * self.layers * self.query_heads * self.head_dim * self.bytes_per_value) as f64
    }

    /// Total KV cache (bytes) for a conversation of `tokens` tokens.
    pub fn bytes(&self, tokens: u64) -> f64 {
        self.bytes_per_token() * tokens as f64
    }
}

/// Time (s) to move `bytes` over a link of `bits_per_second`.
pub fn transfer_time(bytes: f64, bits_per_second: f64) -> f64 {
    bytes * 8.0 / bits_per_second
}

/// Tokens per second a fresh anchor can re-read a transcript at when it has
/// to rebuild a conversation's working memory from text -- a *prefill*.
///
/// **A stated guess.** It is the order of magnitude at which a large
/// accelerator cluster prefills an 80-layer model at long context, not a
/// measurement of any hardware this proposal has priced. What turns on it is
/// the stall a *dead* anchor costs: there is nothing left to stream, so the
/// successor re-reads the whole context at this rate before it can say another
/// word. Nothing about a *planned* move depends on it -- a
/// planned move streams the working memory make-before-break and never
/// prefills (ADR-0022). Halve or double it and the recovery stall moves with
/// it; the planned lane does not move at all.
pub const PREFILL_TOKENS_PER_SECOND: f64 = 10_000.0;

/// Bytes of transcript text per token, for sizing what a fresh anchor has to
/// be given before it can rebuild a conversation. Stated, and small: a
/// 32k-token transcript is a tenth of a megabyte, three orders of magnitude
/// under the working memory it regenerates.
pub const TRANSCRIPT_BYTES_PER_TOKEN: f64 = 4.0;

/// Bits per second a parachute terminal can push back up to its access
/// satellite.
///
/// **A stated guess**, and deliberately a modest one: a half-meter panel
/// closing a 3,642 km link has considerably more than this, but nothing in
/// this simulator prices a terminal's modem, so nothing here may assume one.
/// What turns on it is how fast a conversation comes back after its anchor
/// dies. [`replay_crossover_bps`] is what makes the guess safe -- it says what
/// uplink the edge copy needs in order to beat a durable store half a light
/// second away, and the answer lands far below any rate that could carry the
/// conversation itself.
pub const TERMINAL_UPLINK_BPS: f64 = 10e6;

/// Seconds to put a conversation's transcript in front of a fresh anchor when
/// the durable copy is the terminal's own: the successor asks, the terminal
/// answers, each leg `one_way_s`, the upload at `uplink_bps`.
pub fn edge_replay_time(transcript_bytes: f64, uplink_bps: f64, one_way_s: f64) -> f64 {
    2.0 * one_way_s + transfer_time(transcript_bytes, uplink_bps)
}

/// The terminal uplink rate at which edge replay costs exactly what a fetch
/// from a durable store `remote_one_way_s` away would cost, given a store on a
/// link fat enough that its own transfer time vanishes.
///
/// Above this rate the copy at the edge arrives first; below it the remote
/// store does, and that difference is the whole of what a spacecraft at the
/// balance points would have bought. Infinite when the store is no further
/// away than the terminal, where no uplink can win.
pub fn replay_crossover_bps(
    transcript_bytes: f64,
    remote_one_way_s: f64,
    edge_one_way_s: f64,
) -> f64 {
    let saved = 2.0 * (remote_one_way_s - edge_one_way_s);
    if saved <= 0.0 {
        return f64::INFINITY;
    }
    transcript_bytes * 8.0 / saved
}

/// Time (s) to rebuild `tokens` of working memory from a transcript at
/// `tokens_per_second`.
pub fn prefill_time(tokens: u64, tokens_per_second: f64) -> f64 {
    tokens as f64 / tokens_per_second
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_planet() -> CentralBody {
        CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0)
    }

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn shell_distance_access_to_meo() {
        let p = reference_planet();
        // Radial: 26,371 − 8,571 = 17,800 km. At 60° separation: 23,300 km.
        assert_close(shell_distance(&p, 2_200e3, 20_000e3, 0.0), 1.78e7, 1e-3);
        assert_close(
            shell_distance(&p, 2_200e3, 20_000e3, 60.0_f64.to_radians()),
            2.3300e7,
            1e-3,
        );
    }

    #[test]
    fn light_times_for_canon_distances() {
        // L1/L2 at ~146,000 km ⇒ ~0.487 s; L4/L5 at ~7.2555e6 km ⇒ ~24.2 s.
        assert_close(one_way_light_time(1.46e8), 0.4870, 1e-3);
        assert_close(one_way_light_time(7.2555e9), 24.202, 1e-3);
    }

    #[test]
    fn reference_model_kv_cache_sizes() {
        // 80 layers × 8 KV heads × head dim 128 × 2 bytes, keys and values:
        // 327,680 B/token (320 KiB); 32k-token context ≈ 10.7 GB.
        let m = KvCacheModel {
            layers: 80,
            query_heads: 64,
            kv_heads: 8,
            head_dim: 128,
            bytes_per_value: 2,
        };
        assert_eq!(m.bytes_per_token(), 327_680.0);
        assert_close(m.bytes(32_768), 1.0737e10, 1e-3);

        // Grouped-query attention: 64 query heads share the 8 cached heads,
        // 8 apiece. Without the sharing the cache would be 8× larger,
        // 2,621,440 B/token (2.5 MiB) and a 32k context ≈ 85.9 GB.
        assert_eq!(m.queries_per_kv_head(), 8);
        assert_eq!(m.unshared_bytes_per_token(), 2_621_440.0);
        assert_eq!(m.unshared_bytes_per_token(), 8.0 * m.bytes_per_token());
        assert_close(m.unshared_bytes_per_token() * 32_768.0, 8.5899e10, 1e-3);
    }

    #[test]
    fn kv_migration_takes_about_a_second_at_100_gbps() {
        let m = KvCacheModel {
            layers: 80,
            query_heads: 64,
            kv_heads: 8,
            head_dim: 128,
            bytes_per_value: 2,
        };
        assert_close(transfer_time(m.bytes(32_768), 100e9), 0.859, 1e-3);
        assert_close(transfer_time(m.bytes(32_768), 10e9), 8.59, 1e-3);
    }

    #[test]
    fn prefill_of_the_reference_session_takes_seconds_not_milliseconds() {
        // 32,768 tokens at the stated 10,000 tok/s: 3.28 s. This is the
        // stall a dead anchor costs and a planned move never pays.
        assert_close(
            prefill_time(32_768, PREFILL_TOKENS_PER_SECOND),
            3.2768,
            1e-6,
        );
        assert_eq!(prefill_time(0, PREFILL_TOKENS_PER_SECOND), 0.0);
    }

    #[test]
    fn transcript_is_a_tenth_of_a_megabyte() {
        // What a fresh anchor is given is text, not working memory: 32k
        // tokens of it is 131 kB against 10.7 GB of KV cache.
        let bytes = 32_768.0 * TRANSCRIPT_BYTES_PER_TOKEN;
        assert_eq!(bytes, 131_072.0);
        assert!(bytes < 1e-3 * 1.0737e10);
    }

    /// Worst geometry, the same one the first-token budget is argued at: the
    /// town at the edge of its satellite's footprint, the anchor 60 degrees
    /// around the sky. 12.1 ms of radio plus 77.7 ms of feeder link.
    const EDGE_ONE_WAY_S: f64 = 0.0898;
    /// L1/L2, half a light second out.
    const BALANCE_POINT_ONE_WAY_S: f64 = 0.487;

    #[test]
    fn a_megabit_and_a_third_of_uplink_buys_what_the_balance_points_sell() {
        // The durable store's whole advantage is a fat link at the far end;
        // its whole disadvantage is being half a light second away. The
        // crossover is the uplink that trades one for the other -- and it
        // lands at 1.3 Mbps, roughly four hundred times the 3 kbps a
        // conversation itself costs. Any terminal that can hold a
        // conversation can beat the balance points at recovering one.
        let bytes = 32_768.0 * TRANSCRIPT_BYTES_PER_TOKEN;
        let crossover = replay_crossover_bps(bytes, BALANCE_POINT_ONE_WAY_S, EDGE_ONE_WAY_S);
        assert_close(crossover, 1.320e6, 1e-3);
    }

    #[test]
    fn edge_replay_of_the_reference_session_is_under_three_tenths_of_a_second() {
        // 180 ms of round trip over the access path, plus 105 ms of upload at
        // the stated 10 Mbps -- against 974 ms of light alone to L1/L2.
        let bytes = 32_768.0 * TRANSCRIPT_BYTES_PER_TOKEN;
        let edge = edge_replay_time(bytes, TERMINAL_UPLINK_BPS, EDGE_ONE_WAY_S);
        assert_close(edge, 0.2845, 1e-3);
        assert!(edge < 2.0 * BALANCE_POINT_ONE_WAY_S);
    }

    #[test]
    fn a_slow_enough_uplink_loses_to_the_balance_points() {
        // Pin the losing side too. At 1 Mbps the upload alone outruns the
        // light time it saves, and the rejected architecture would have been
        // the quicker one -- at the price of a spacecraft.
        let bytes = 32_768.0 * TRANSCRIPT_BYTES_PER_TOKEN;
        let edge = edge_replay_time(bytes, 1e6, EDGE_ONE_WAY_S);
        let remote = 2.0 * BALANCE_POINT_ONE_WAY_S + transfer_time(bytes, 100e9);
        assert!(edge > remote, "edge {edge} s, remote {remote} s");
    }

    #[test]
    fn no_uplink_wins_against_a_store_that_is_no_further_away() {
        let bytes = 32_768.0 * TRANSCRIPT_BYTES_PER_TOKEN;
        assert!(replay_crossover_bps(bytes, EDGE_ONE_WAY_S, EDGE_ONE_WAY_S).is_infinite());
    }
}
