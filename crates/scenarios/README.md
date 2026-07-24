# Scenarios

Scenario definitions and channel-trace data for helixsim runs. This directory
holds data, not a Rust crate — each scenario is a TOML config plus the CSV trace
files it references.

The first vertical slice is a small LEO satellite constellation testbed
(2 ground terminals, 3 satellites, 1 gateway) that demonstrates a mid-run
handover, a degraded-SINR loss window, and a compute-queue burst. See
`../../docs/specs/2026-07-23-network-simulator-design.md` §3.4.
