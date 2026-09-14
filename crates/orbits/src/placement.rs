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
#[derive(Debug, Clone, Copy)]
pub struct KvCacheModel {
    pub layers: usize,
    pub kv_heads: usize,
    pub head_dim: usize,
    pub bytes_per_value: usize,
}

impl KvCacheModel {
    /// Bytes of KV cache appended per token of context.
    pub fn bytes_per_token(&self) -> f64 {
        (2 * self.layers * self.kv_heads * self.head_dim * self.bytes_per_value) as f64
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
/// the stall a *dead* anchor costs: recovery from the vault has nothing to
/// stream, so the successor re-reads the whole context at this rate before it
/// can say another word. Nothing about a *planned* move depends on it -- a
/// planned move streams the working memory make-before-break and never
/// prefills (ADR-0022). Halve or double it and the recovery stall moves with
/// it; the planned lane does not move at all.
pub const PREFILL_TOKENS_PER_SECOND: f64 = 10_000.0;

/// Bytes of transcript text per token, for sizing what the vault sends back.
/// Stated, and it barely matters: a 32k-token transcript is a tenth of a
/// megabyte, and its transfer vanishes next to the half-light-second each way.
pub const TRANSCRIPT_BYTES_PER_TOKEN: f64 = 4.0;

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
            kv_heads: 8,
            head_dim: 128,
            bytes_per_value: 2,
        };
        assert_eq!(m.bytes_per_token(), 327_680.0);
        assert_close(m.bytes(32_768), 1.0737e10, 1e-3);
    }

    #[test]
    fn kv_migration_takes_about_a_second_at_100_gbps() {
        let m = KvCacheModel {
            layers: 80,
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
        // The vault sends text back, not working memory: 32k tokens of it
        // is 131 kB, and its transfer vanishes next to the light time.
        let bytes = 32_768.0 * TRANSCRIPT_BYTES_PER_TOKEN;
        assert_eq!(bytes, 131_072.0);
        assert!(transfer_time(bytes, 100e9) < 1e-4);
    }
}
