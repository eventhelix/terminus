// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! The wire unit. HARD RULE (design §3.1): `PacketMeta` is
//! observability-only — no model may branch on it. Behavior derives
//! exclusively from `bytes`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketMeta {
    /// `(origin_node as u64) << 48 | per-node counter` — unique per run.
    pub id: u64,
    /// Sim time of creation, ns since sim t0.
    pub birth_ns: u64,
    /// Originating node id.
    pub origin: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Packet {
    /// The exact frame as it would appear on the wire.
    pub bytes: Vec<u8>,
    pub meta: PacketMeta,
}

/// NetIf → Medium.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transmission {
    pub tx_node: u16,
    pub packet: Packet,
}

/// NetIf → Compute → NodeModel (rx path).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RxFrame {
    pub if_index: u8,
    pub packet: Packet,
}
