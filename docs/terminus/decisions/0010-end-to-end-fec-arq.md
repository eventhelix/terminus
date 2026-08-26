# ADR-0010: End-to-end reliability — FEC-first, ARQ-guaranteed, timetable-aware

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-003, TER-REQ-012, TER-REQ-013, TER-REQ-014
Evidence: `cargo run -p terminus-orbits --example unbroken_thread` (tag: terminus-post-13)

## Decision

1. **The reliability domain is end to end: ground terminal ↔ session
   anchor.** A QUIC-descended transport with connection migration and an
   FEC extension runs between those two endpoints only; satellites in
   between route and never retransmit. Radio-side PHY FEC/HARQ layers
   beneath it and is designed separately (Series 2).
2. **FEC first, ARQ second.** Systematic application-layer FEC
   (generations of k source + r repair packets) rides proactively with the
   token stream, healing losses in zero extra round trips. Losses
   exceeding the repair budget escalate: first a request for additional
   repair symbols (one round trip), then classic ARQ retransmission — the
   floor that never fails.
3. **The timetable breathes the overhead.** Repair overhead is scheduled,
   not reactive: lean (~5–12.5%) in calm skies, raised ahead of scheduled
   handovers and during flares — a scheduled outage is erasures paid for
   in advance (ADR-0009's timetable is the schedule's source).
4. **Node failure recovery is transport + architecture, not FEC.** The
   connection's identity binds terminal↔anchor, so route changes
   (including access handover) are invisible (ADR-0004's principle at the
   transport layer). Anchor death: 300 ms keep-alive detection
   (ADR-0009), migration to the pre-assigned backup anchor, vault
   re-seed (ADR-0004), and ARQ retransmission of everything
   unacknowledged — bounded loss, no session restart (TER-REQ-013/014).
5. **Codecs are implementations behind the project `FecCodec` trait**
   (RaptorQ, Reed-Solomon, sliding-window, RLNC — see
   `references.md`); the preferred codec is derived experimentally in
   Series 2, never baked into the architecture.

## Why

- **ARQ alone cannot meet the stall budget.** A retransmission costs one
  terminal↔anchor round trip: 180 ms worst geometry, 1.8× the 100 ms p99
  stall budget. A 20-packet/s token stream at 1% residual loss stalls
  every 5 s forever; at flare-time 5%, every second.
- **Modest FEC transforms the arithmetic.** Two repair packets per
  sixteen (12.5% overhead) stretch the stall interval from 5 s to
  ≈18 min (residual 7.3e-4, ≈220×); four repair packets reach ≈7 days.
  At 5% loss the same ladder shows why overhead must breathe: 16+2
  degrades to a stall every 14 s, while 16+8 holds ≈7 days.
- **FEC and ARQ fail in complementary ways.** FEC is probabilistic and
  blind to dead endpoints; ARQ is certain but slow. The tandem gives the
  interactive stream FEC's latency and the session ARQ's guarantee.

## Consequences

- Series 2 implements the machinery in the packet-level simulator: the
  `FecCodec` trait and candidate codecs, generation sizing and timeouts,
  the extra-repair request protocol, burst-loss (non-independent) models
  replacing this first-order screening model, cross-band repair (Ka
  source / X repair, ADR-0005), and measured token-stall distributions
  against TER-REQ-003.
- The 12.5% cruise overhead becomes a capacity-planning input
  (TER-REQ-005 study).
- Terminal transport must implement FEC decode and ARQ state — bounded,
  static logic consistent with the nothing-stale terminal law (ADR-0007).
