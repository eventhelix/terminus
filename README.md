# terminus

A discrete-event simulator for networking and wireless systems — satellite,
mobile, and terrestrial — written in Rust.

Two things live here: the simulator, and **Terminus**, the series it was built
to answer.

## The Terminus series

An alien AI offers a young civilization on a tidally locked planet a gift:
planet-wide access to a large language model, served from orbit. Terminus is
the engineering reply, written as the winning contractor's technical proposal —
a constellation designed for a world with an eternal sunset belt, no stationary
orbit, and a red sun that floods the beginner radio bands with noise.

It is science fiction with its arithmetic showing. **Every number a post quotes
comes out of a program in this repository**, run at a tagged commit, so a
reader can re-derive any claim rather than take it.

Read it at **[eventhelix.com/terminus](https://www.eventhelix.com/terminus)**.

| # | Post | What it settles | Evidence |
|---|------|-----------------|----------|
| 1 | [The RFP](https://www.eventhelix.com/terminus/rfp) | The planet, the constraints, and the sixteen requirements everything else answers to | in-universe document |
| 2 | [Know your planet](https://www.eventhelix.com/terminus/know-your-planet) | A terminator frozen on the ground but turning in space — and why Mercury is a false twin | `terminator_drift` |
| 3 | [The elegant trap](https://www.eventhelix.com/terminus/the-elegant-trap) | Riding the sunset line forever costs 3.9 km/s per day; the fleet burns down to 2% in a month | `terminator_tracking` |
| 4 | [Shelves of the sky](https://www.eventhelix.com/terminus/shelves-of-the-sky) | Every altitude from 300 to 205,000 km surveyed; the Hill sphere erases the fixed point in the sky | `regime_survey` |
| 5 | [Rings over twilight](https://www.eventhelix.com/terminus/rings-over-twilight) | Six polar rings on a 22.4-hour shift schedule, sized until no town falls silent | `access_constellation` |
| 6 | [Where the mind lives](https://www.eventhelix.com/terminus/where-the-mind-lives) | Working memory weighs 320 KiB per token, so a handover moves routes, never minds | `compute_placement` |
| 7 | [Talking past a flaring red star](https://www.eventhelix.com/terminus/talking-past-a-flaring-red-star) | Ka-band pencil beams win the link budget, with X-band riding shotgun through storms | `frequency_plan` |
| 8 | [Beams, not blankets](https://www.eventhelix.com/terminus/beams-not-blankets) | Spot beams collapse a ±460 kHz Doppler window to ±6 kHz; both ends steer by arithmetic | `spot_beams`, `terminal_aperture` |
| 9 | [First contact](https://www.eventhelix.com/terminus/first-contact) | A box wakes knowing nothing, is found in seconds, and learns nothing that can go stale | `first_contact` |
| 10 | [The backbone](https://www.eventhelix.com/terminus/the-backbone) | Six frozen laser necklaces, feeder links that compute their Doppler, and a timing fabric | `backbone`, `clock_rates` |
| 11 | [The unbroken thread](https://www.eventhelix.com/terminus/the-unbroken-thread) | A retransmission costs 180 ms against a 100 ms stall budget, so FEC heals and ARQ guarantees | `unbroken_thread` |
| 12 | [The proposal rests](https://www.eventhelix.com/terminus/the-proposal-rests) | The truthful ledger: sixteen requirements traced to their decisions and runs | compliance matrix |

Two companions sit alongside the twelve: the
[constellation explorer](https://www.eventhelix.com/terminus/explorer), a live
3D model of the finished system with animated satellite, ring, and anchor
handovers; and an appendix,
[the algorithms](https://www.eventhelix.com/terminus/the-algorithms), which
states the routing and anchor-selection procedures once, in full.

### Reproducing a number

Each post is pinned by a `terminus-post-N` tag, and its numbers come from an
example in [`crates/orbits`](crates/orbits/examples). Check out the tag, run
the example, read the figures the post quotes:

```bash
git checkout terminus-post-5
cargo run -p terminus-orbits --example access_constellation
```

### The canon

[`docs/terminus/`](docs/terminus/) is the design of record for the series, and
the place to start before changing anything it says:

- [`world-bible.md`](docs/terminus/world-bible.md) — the planet, the star, the
  civilization, and the constants every post shares
- [`requirements.md`](docs/terminus/requirements.md) — the sixteen `TER-REQ-*`
  requirements the RFP sets
- [`decisions/`](docs/terminus/decisions) — twenty-eight ADRs recording what was
  chosen, what was rejected, and on what evidence
- [`compliance-matrix.md`](docs/terminus/compliance-matrix.md) — requirement →
  decision → run, honest about what is still open
- [`manuscript-map.md`](docs/terminus/manuscript-map.md) — post → book chapter,
  with the evidence tag for each

## The simulator

terminus serves two missions:

- **Test and integration platform.** Exercise system software against simulated
  nodes and links before hardware is available. Node models are designed so real
  production components can replace them one at a time (model-in-the-loop →
  software-in-the-loop).
- **Performance simulation.** Study latency, throughput, loss, and load behavior
  under time-varying link conditions — changing propagation delay between moving
  nodes, and changing link quality via SINR→BLER mapping.

### What makes it different

Every exchange between simulated nodes is **real protocol bytes**. terminus
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

### Status

Early development. The design of record is
[`docs/specs/2026-07-23-network-simulator-design.md`](docs/specs/2026-07-23-network-simulator-design.md).
The first vertical slice is a small LEO satellite constellation testbed
(~10–100 nodes) demonstrating variable-delay links, SINR→BLER loss, node compute
queueing, and a mid-run handover — all reproducible byte-for-byte from a master
seed.

### Build, run, test

Full instructions — inspecting captures and metrics, test suites, golden-digest
updates, dissector regeneration, reproducing runs — live in the
[runbook](docs/runbook.md). Quick start:

```bash
cargo build --release
cargo run -p terminus -- run crates/scenarios/leo-testbed/scenario.toml --out out
cargo test --workspace                                  # unit + determinism + golden + smoke
cargo test -p terminus --test dissection -- --ignored   # needs tshark on PATH
```

Each run writes a self-describing directory: per-node PCAPNG captures on
LINKTYPE_USER0, the exact matching Wireshark Lua dissectors, a
`visualether.toml` the VisualEther MCP tools pick up automatically,
`metrics.ndjson`, and a full config snapshot. Open any capture with:

```bash
wireshark -X lua_script:<run-dir>/dissectors/link.lua <run-dir>/nodes/term-a.pcapng
```

### Engine

terminus is built directly on [nexosim](https://github.com/asynchronics/nexosim),
a high-performance, deterministic, parallel discrete-event simulation framework.
Nodes, protocol layers, and transmission media are all nexosim models.

## License

**Code** — GNU Affero General Public License v3.0 ([LICENSE](LICENSE)).

Use it, study it, modify it, and share it freely. If you distribute a modified
version, or run one as a network service, the AGPL requires you to offer your
users the corresponding source under the same terms. For anyone who wants to
build on terminus without that obligation, EventHelix.com Inc. holds the
copyright and can grant a separate commercial license — get in touch.

**Terminus series canon** — the world bible, requirements baseline, decision
log, and other documents under [`docs/terminus/`](docs/terminus/) are licensed
under [CC BY-NC-ND 4.0](docs/terminus/LICENSE), not the AGPL. They are the
source material for a written series; the code is what you are invited to build
on.

Individual crates in this workspace are written to stay generic and
independently usable, so any of them may later be split into its own repository
and released under permissive terms.

See [NOTICE](NOTICE) for the license of record and third-party components, and
[CONTRIBUTING.md](CONTRIBUTING.md) before sending code.
