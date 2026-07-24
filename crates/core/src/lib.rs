//! Core simulation abstractions for helixsim.
//!
//! This crate defines the building blocks that scenarios are assembled from —
//! packets, nodes, media, channel traces, compute models, and packet capture —
//! all implemented as [nexosim](https://github.com/asynchronics/nexosim) models.
//!
//! See `docs/specs/2026-07-23-network-simulator-design.md` for the design of
//! record. Implementation lands per the slice-1 plan; this is the scaffold.

// Modules are introduced by the slice-1 implementation plan. Declared here as a
// map of the intended surface; each becomes a file as it is built.
//
//   pub mod packet;    // Packet = wire bytes + observability-only metadata
//   pub mod node;      // NodeModel / NetIf assembly traits
//   pub mod medium;    // per-link-domain medium model
//   pub mod channel;   // ChannelTrace: delay + SINR->BLER over time
//   pub mod compute;   // N-core bounded-queue processing-latency model
//   pub mod capture;   // PcapTap: per-node PCAPNG writer

pub mod rng;       // master-seed -> per-model deterministic RNG
pub mod packet;
pub mod records;
pub mod simtime;
pub mod bler;
pub mod trace;
pub mod compute;
