// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Protocol encoders/decoders for terminus.
//!
//! `link` is generated from `pdl/link.pdl` at build time; the matching
//! Wireshark Lua dissector is generated offline from the SAME file by
//! `tools/regen-dissector.sh` and checked in under `dissectors/`.

pub mod link {
    #![allow(
        missing_docs,
        clippy::all,
        unused_parens,
        unreachable_patterns,
        unused_imports
    )]
    include!(concat!(env!("OUT_DIR"), "/link_gen.rs"));
}

pub mod udp;

pub use pdl_runtime::{DecodeError, EncodeError, Packet as PdlPacket};
