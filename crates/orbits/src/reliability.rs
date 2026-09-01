// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! End-to-end reliability arithmetic: how often an ARQ-only token stream
//! stalls, and how far proactive FEC stretches the interval between stalls.
//!
//! Model: independent packet losses at rate `p` (a first-order screening
//! model; burst losses are Series 2 territory). A systematic FEC generation
//! sends `k` source packets plus `r` repair packets; the generation is
//! unrecoverable — a stall — only if more than `r` of the `k + r` packets
//! are lost.

/// P(at least `k_min` losses among `n` packets) at per-packet loss rate `p`.
pub fn binomial_tail(n: u64, k_min: u64, p: f64) -> f64 {
    let mut tail = 0.0;
    for i in k_min..=n {
        let mut term = 1.0;
        for j in 0..i {
            term *= (n - j) as f64 / (i - j) as f64;
        }
        term *= p.powi(i as i32) * (1.0 - p).powi((n - i) as i32);
        tail += term;
    }
    tail
}

/// Mean seconds between stalls for an ARQ-only stream: every loss costs a
/// retransmission round trip, so stalls arrive at the loss rate.
pub fn arq_stall_interval(packet_rate: f64, loss_rate: f64) -> f64 {
    1.0 / (packet_rate * loss_rate)
}

/// Probability that one FEC generation of `k_source` + `r_repair` packets
/// cannot be recovered (more than `r_repair` losses).
pub fn fec_residual_rate(k_source: u64, r_repair: u64, p: f64) -> f64 {
    binomial_tail(k_source + r_repair, r_repair + 1, p)
}

/// Mean seconds between stalls with proactive FEC: one generation lasts
/// `k_source / packet_rate` seconds and stalls with the residual rate.
pub fn fec_stall_interval(packet_rate: f64, k_source: u64, r_repair: u64, p: f64) -> f64 {
    (k_source as f64 / packet_rate) / fec_residual_rate(k_source, r_repair, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn binomial_tail_sanity() {
        assert_close(binomial_tail(10, 0, 0.3), 1.0, 1e-12);
        assert_close(binomial_tail(1, 1, 0.07), 0.07, 1e-12);
        // P(≥1 loss in 2) = 1 − (1−p)² = 2p − p².
        assert_close(binomial_tail(2, 1, 0.1), 0.19, 1e-12);
    }

    #[test]
    fn arq_only_stalls_every_five_seconds() {
        // 20 packets/s token stream at 1% residual loss.
        assert_close(arq_stall_interval(20.0, 0.01), 5.0, 1e-12);
    }

    #[test]
    fn two_repair_packets_stretch_stalls_to_eighteen_minutes() {
        // 16 + 2 at 1%: P(≥3 losses in 18) ≈ 7.292e-4; a generation lasts
        // 0.8 s, so stalls arrive every ≈ 1,097 s (18.3 min) — ~219× fewer
        // than ARQ-only, for 12.5% overhead.
        assert_close(fec_residual_rate(16, 2, 0.01), 7.292e-4, 1e-3);
        assert_close(fec_stall_interval(20.0, 16, 2, 0.01), 1.097e3, 1e-3);
    }
}
