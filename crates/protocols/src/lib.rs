//! Protocol encoders/decoders for helixsim.
//!
//! `link` is generated from `pdl/link.pdl` at build time; the matching
//! Wireshark Lua dissector is generated offline from the SAME file by
//! `tools/regen-dissector.sh` and checked in under `dissectors/`.

pub mod link {
    #![allow(missing_docs, clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/link_gen.rs"));
}

pub use pdl_runtime::{DecodeError, EncodeError, Packet as PdlPacket};
