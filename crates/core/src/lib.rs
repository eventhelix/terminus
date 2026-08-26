//! Core simulation abstractions for terminus.
//!
//! This crate defines the building blocks that scenarios are assembled from —
//! packets, nodes, media, channel traces, compute models, and packet capture —
//! all implemented as [nexosim](https://github.com/asynchronics/nexosim) models.
//!
//! See `docs/specs/2026-07-23-network-simulator-design.md` for the design of
//! record. Implementation lands per the slice-1 plan; this is the scaffold.

pub mod behaviors;
pub mod bler;
pub mod capture;
pub mod compute;
pub mod conformance;
pub mod medium;
pub mod netif;
pub mod node;
pub mod packet;
pub mod records;
pub mod rng;
pub mod simtime;
pub mod trace;
