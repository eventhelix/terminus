//! helixsim command-line entry point.
//!
//! Runs a scenario and produces a self-describing output directory (per-node
//! PCAPNG captures, matching Wireshark dissectors, a `visualether.toml` the
//! VisualEther MCP tools pick up automatically, run metrics, and a config
//! snapshot). See `docs/specs/2026-07-23-network-simulator-design.md`.
//!
//! This is the scaffold; command wiring lands per the slice-1 implementation plan.

fn main() {
    println!("helixsim — scaffold. See docs/specs/ for the design of record.");
}
