//! Protocol encoders and decoders for helixsim.
//!
//! Custom link/adaptation and control-plane layers are defined once in Google
//! [PDL](https://github.com/google/pdl) and generate, from the same source at
//! the same commit, both Rust codecs (`pdl-compiler`) and Wireshark Lua
//! dissectors (`pdl-dissector`) — so the bytes a scenario emits and the
//! dissector Wireshark loads can never drift apart.
//!
//! Standard layers (IPv4/IPv6, UDP, TCP) are built with `etherparse` and carried
//! inside the PDL-defined framing; Wireshark chains its built-in dissectors after
//! the PDL layers.
//!
//! See `docs/specs/2026-07-23-network-simulator-design.md`. This is the scaffold;
//! the PDL toolchain wiring lands per the slice-1 plan.
