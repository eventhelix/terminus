# helixsim

A discrete-event simulator for networking and wireless systems — satellite,
mobile, and terrestrial — written in Rust.

helixsim serves two missions:

- **Test and integration platform.** Exercise system software against simulated
  nodes and links before hardware is available. Node models are designed so real
  production components can replace them one at a time (model-in-the-loop →
  software-in-the-loop).
- **Performance simulation.** Study latency, throughput, loss, and load behavior
  under time-varying link conditions — changing propagation delay between moving
  nodes, and changing link quality via SINR→BLER mapping.

## What makes it different

Every exchange between simulated nodes is **real protocol bytes**. helixsim
captures each run to standard PCAPNG with matching Wireshark dissectors, so any
simulation can be:

- opened directly in **Wireshark**, and
- analyzed by AI through the [VisualEther](https://www.eventhelix.com/visualether/)
  MCP tools — turning a simulated run into sequence diagrams and explanations.

Custom protocol layers are defined once in [Google PDL](https://github.com/google/pdl),
which generates both the Rust encoder/decoder and the Wireshark dissector from the
same source — the bytes on the virtual wire and the dissector that reads them can
never drift apart. Standard layers (IP/UDP/TCP) are real encodings via
[`etherparse`](https://crates.io/crates/etherparse).

## Status

Early development. The design of record is
[`docs/specs/2026-07-23-network-simulator-design.md`](docs/specs/2026-07-23-network-simulator-design.md).
The first vertical slice is a small LEO satellite constellation testbed
(~10–100 nodes) demonstrating variable-delay links, SINR→BLER loss, node compute
queueing, and a mid-run handover — all reproducible byte-for-byte from a master
seed.

## Build, run, test

Full instructions — inspecting captures and metrics, test suites, golden-digest
updates, dissector regeneration, reproducing runs — live in the
[runbook](docs/runbook.md). Quick start:

```bash
cargo build --release
cargo run -p helixsim -- run crates/scenarios/leo-testbed/scenario.toml --out out
cargo test --workspace                                  # unit + determinism + golden + smoke
cargo test -p helixsim --test dissection -- --ignored   # needs tshark on PATH
```

Each run writes a self-describing directory: per-node PCAPNG captures on
LINKTYPE_USER0, the exact matching Wireshark Lua dissectors, a
`visualether.toml` the VisualEther MCP tools pick up automatically,
`metrics.ndjson`, and a full config snapshot. Open any capture with:

```bash
wireshark -X lua_script:<run-dir>/dissectors/link.lua <run-dir>/nodes/term-a.pcapng
```

## Engine

helixsim is built directly on [nexosim](https://github.com/asynchronics/nexosim),
a high-performance, deterministic, parallel discrete-event simulation framework.
Nodes, protocol layers, and transmission media are all nexosim models.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
