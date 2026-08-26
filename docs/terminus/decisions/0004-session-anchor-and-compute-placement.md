# ADR-0004: Session anchoring — access relays, MEO compute, L1/L2 archive

Status: accepted
Date: 2026-08-21
Requirements: TER-REQ-002, TER-REQ-003, TER-REQ-013, TER-REQ-014
Evidence: `cargo run -p terminus-orbits --example compute_placement` (tag: terminus-post-6)

## Decision

Inference does not live on the access satellites. The architecture is
layered:

- **Access layer (2,200 km rings, ADR-0003):** radio, switching, optical
  inter-satellite links. Carries conversations; owns none of their state.
- **Compute layer (MEO, reference 20,000 km):** model weights,
  accelerators, active sessions, KV cache. Each conversation has one
  **session anchor** — the compute satellite that owns its inference state.
- **Durable layer (L1/L2, ≈146,000 km):** replicated conversation history,
  model repository, checkpoints, backups. Never in the interactive path.
  (L4/L5, ~24 light-seconds away, reserved for training/archive roles.)

Governing principle: **access handover is a routing event, not a
compute-state migration event.** The user's radio path changes every ~17
minutes; the session anchor does not.

## Why

- **KV cache is heavy.** The reference model (world bible, "The gift")
  accumulates 320 KiB of working memory per token; a 32k-token
  conversation carries ≈10.7 GB. Moving that at every access handover
  (~17 min dwell) is absurd; anchored on MEO (~207 min dwell) one session
  survives ~12 access handovers unmoved.
- **The latency budget closes.** Worst-geometry first-token propagation,
  user → access → MEO anchor and back: 2 × (12.1 + 77.7) ms = 180 ms,
  leaving 120 ms of inference time inside TER-REQ-003's 300 ms. L1/L2 at
  0.49 s one way can never converse; it archives (TER-REQ-002's state
  durability without touching the interactive path).
- **Compute handover is schedulable.** Orbits are clocks: anchor dwell
  ends are predictable, so KV migration is make-before-break over a laser
  link — 10.7 GB in ≈0.9 s at 100 Gbps (≈8.6 s at 10 Gbps) — streamed
  before the switch, invisible to the user (TER-REQ-013), and the durable
  layer's replica bounds loss if an anchor dies mid-conversation
  (TER-REQ-014).

## Consequences

- The optical inter-satellite backbone (access↔MEO, MEO↔MEO, MEO↔L1/L2)
  becomes load-bearing infrastructure; its design and the compute-fleet
  sizing (how many MEO anchors, their coverage of the access rings) are
  deliberately deferred to the backbone post.
- Transport must survive route changes under an anchored session — the
  Series 2 material (QUIC-style migration, end-to-end FEC between ground
  terminal and anchor).
- MEO spacecraft grow: accelerators, radiators, large arrays — the
  power/thermal trade returns in the constellation-economics post.
