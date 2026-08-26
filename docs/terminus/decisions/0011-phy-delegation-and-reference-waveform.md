# ADR-0011: PHY delegation contract and reference waveform

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-003, TER-REQ-006, TER-REQ-007, TER-REQ-012, TER-REQ-016
Evidence: `cargo run -p terminus-orbits --example unbroken_thread` (tag: terminus-post-14)

## Decision

1. **The physical layer is delegated to the patron under an interface
   contract.** The AI manufactures both ends of every radio link and can
   iterate the PHY freely across satellite generations. The proposal
   binds only the interface the layers above depend on: after PHY FEC and
   HARQ, the link presents residual packet erasures of **≤1% in calm
   conditions and ≤5% under flare/handover stress, with bounded per-hop
   latency** — exactly the loss regime ADR-0010's reliability ladder is
   sized against. PHY internals may change; the contract may not, without
   re-running the reliability arithmetic.
2. **Reference waveform (contract-satisfying baseline, NTN-flavored):**
   LDPC codes for data and polar codes for control; CP-OFDM on the
   downlink; DFT-spread-OFDM on the uplink for low peak-to-average power
   (terminal PA efficiency and thermal margin — a ten-year-lifetime
   concern, TER-REQ-006/007); per-hop HARQ with soft combining
   (Chase/incremental redundancy), pipelined across the 24.3 ms access
   feedback loop (~25 slots in flight ⇒ ≥32 parallel HARQ processes, as
   Earth NTN practice).
3. **Combining lives where soft information lives.** HARQ soft-combining
   (MRC of receptions) operates per radio hop, inside the one receiver
   that holds both copies' soft symbols. End-to-end ARQ retransmissions
   (ADR-0010) are fresh packets that may traverse a different satellite,
   band, or anchor — they gain path diversity instead, and the transport
   layer's own "combining" is FEC's any-k-of-n algebra. No soft state
   crosses the PHY boundary.
4. **No exotic waveform.** Doppler/delay-hostile designs (e.g.
   OTFS-class) solve channel dynamics that ADR-0006's per-beam
   precompensation has already removed: terminals see a quasi-static
   channel, so the conventional waveform is sufficient — and simplicity
   is a reliability feature for unattended decade-scale terminals.

## Why

- The series' scope is the layers above PHY; the fiction and the
  engineering agree on the same remedy — the one contractor who builds
  both radio ends owns the waveform, and the architecture pins only what
  the ladder above consumes (the residual-erasure contract).
- The 1%/5% figures make ADR-0010's stall arithmetic a requirements
  flow-down rather than an assumption: if a future PHY misses the
  contract, the FEC overhead schedule — not the architecture — absorbs it.

## Consequences

- Series 2's packet simulator models the PHY as an erasure process
  honoring this contract (plus burst structure), not as waveform DSP;
  BLER-to-erasure mapping stays a PHY-team deliverable.
- Terminal transmit chain assumes DFT-s-OFDM PAPR in its power budget.
- HARQ buffer depth (≥32 processes) is a satellite modem sizing input.
