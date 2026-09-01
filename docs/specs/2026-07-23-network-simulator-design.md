# terminus — Network Discrete-Event Simulator — Design

**Date:** 2026-07-23
**Status:** Approved in brainstorming; name locked (`terminus`); ready for implementation planning
**Name:** `terminus` — crate `terminus`, repo `github.com/eventhelix/terminus`. Brand-coherent
with the EventHelix family (VisualEther, EventStudio); crate + repo names verified available
2026-07-23.
**Home:** This is the canonical copy, living in the `terminus` repo
(`github.com/eventhelix/terminus`, AGPL-3.0-only; the Terminus series canon under
`docs/terminus/` is CC BY-NC-ND 4.0). The design was brainstormed in the VisualEther
workspace on 2026-07-23; a copy of this spec remains there for provenance.

## 1. Purpose

A Rust discrete-event simulator for networking and wireless systems (satellite, mobile,
terrestrial) serving two missions:

1. **Test and integration platform** — exercise system software against simulated nodes
   and links before hardware exists.
2. **Performance simulation** — the conventional DES mission: latency, throughput,
   loss, and load behavior under time-varying link conditions.

Distinctive property: every simulated exchange is real protocol bytes, captured to
standard PCAPNG with matching Wireshark dissectors, so any run can be debugged in
Wireshark and analyzed by AI through the VisualEther MCP tools.

## 2. Decisions of record

| Topic | Decision |
|---|---|
| DES engine | [nexosim](https://github.com/asynchronics/nexosim) 1.x, used directly (Approach A: thin layer, no engine-abstraction crate) |
| Node fidelity | Staged: behavioral models first; interfaces designed so real production components replace models one at a time (model-in-the-loop → software-in-the-loop) |
| First vertical slice | LEO satellite constellation |
| Scale target (v1) | Testbed scale, ~10–100 nodes; per-packet events throughout |
| Packet representation | Real encoded bytes everywhere; metadata is observability-only |
| Custom protocols | [Google PDL](https://github.com/google/pdl): one `.pdl` file generates Rust codecs (`pdl-compiler`) and Wireshark Lua dissectors ([mauricelam/pdl-dissector](https://github.com/mauricelam/pdl-dissector)) |
| Standard protocols | Real IPv4/v6 + UDP/TCP encoded via `etherparse`, carried inside PDL-defined framing; Wireshark chains its built-in dissectors |
| Node compute model | Processing-latency model: N cores + bounded queue + configurable service times (trait, replaceable per node) |
| Channel dynamics | Precomputed trace files (delay, SINR vs. time); simulator core is orbit-agnostic |
| Positioning | Independent OSS project, VisualEther-friendly but not VisualEther-dependent |

### Rationale: why nexosim

- Component/actor model with typed ports maps naturally onto nodes, protocol layers,
  and media.
- Deterministic causal message ordering **with** transparent multi-threaded execution —
  reproducibility without giving up parallelism.
- Arbitrary-time event scheduling with cancellation (protocol timers, variable delay).
- Real-time clocks and event injection (1.0) — the door to hardware-in-the-loop later.
- gRPC control server for CI/test-harness orchestration.
- Mature (1.0), Apache/MIT, aerospace pedigree; satellite research groups (IRS
  Stuttgart) already use it.

What nexosim does not provide — and this project builds — is everything network-shaped:
packets, links, media, channel dynamics, compute models, capture.

Alternatives considered: ns-3 / OMNeT++ (C++, fights the Rust + AI-tooling vision),
Shadow (executes real Linux binaries; different niche), other Rust DES crates (less
mature), custom DES core (reimplements what nexosim does well).

### Rationale: why PDL and not Kaitai

Kaitai Struct is fundamentally a parser generator; byte *generation* (serialization) is
experimental, and its Wireshark path is a limited third-party converter. A simulator
must produce bytes. PDL generates Rust encoders and decoders (production-proven in
Android's Bluetooth stack) and Wireshark Lua dissectors from the same source file.
Known risk: `pdl-dissector` is v0.1.0, single-maintainer — plan to vendor or patch;
validating this pipeline is a declared slice-1 risk-retirement item.

## 3. Architecture

One Cargo workspace, four crates, nexosim used directly as the engine and visible in
the API:

```
terminus workspace
├─ core/       Packet, NodeModel/Interface traits, ComputeModel,
│              Medium model, ChannelTrace, PcapTap — all nexosim models
├─ protocols/  .pdl sources → build step → Rust codecs + Lua dissectors;
│              etherparse-based IP/UDP/TCP builders
├─ scenarios/  LEO testbed scenario(s): TOML configs + trace files
└─ cli/        run a scenario → self-describing output directory
```

### 3.1 Core abstractions (all nexosim models)

**Packet.** `Bytes` (the exact frame as it would appear on the wire) + `PacketMeta`
(unique id, birth time, originating node). Hard rule: metadata is observability-only —
no model may branch on it. Behavior derives exclusively from the bytes, which keeps the
real-code drop-in path honest.

**Node = assembly.** Composed at bench time from: one or more protocol/app models (the
user-authored behavior; later, the slot where real code goes), one `NetIf` model per
network interface, and one `ComputeModel`. `NetIf` is deliberately dumb: capture tap
attachment point (like tcpdump on a NIC) and interface up/down state.

**Medium.** One per link domain (ISL medium, ground-link medium…). Every `NetIf` wires
to its medium at assembly — wiring is static, connectivity is data. Per transmission at
time *t*, the medium consults the channel trace: which receivers are reachable, each
one's propagation delay, each one's SINR → BLER → seeded-RNG drop decision. Deliveries
are scheduled at *t + delay(t)*. Handover is simply reachability changing between
packets; nexosim's assembly-time wiring is never rewired.

**ChannelTrace.** Per directed (tx, rx) pair, a time series of `{delay, SINR}` from
versioned trace files. Step-hold between samples (no interpolation) in slice 1.
SINR→BLER is a pluggable curve table; one curve in slice 1, per-MCS later.

**ComputeModel.** N cores + bounded FIFO per node. Work items carry configurable
service times; free core → completion at *now + service_time*, else queue; overflow →
drop + counter. Trait-based so a software-architecture model or measured real-code
timings can replace it per node later.

**Determinism.** One master seed per scenario; every stochastic draw (BLER, jitter)
comes from a per-model RNG derived from it. Same scenario + seed ⇒ byte-identical
outputs. This property is load-bearing for the test-platform mission and is enforced
by CI (§3.5).

### 3.2 Protocol pipeline

- `protocols/` holds `.pdl` sources. Slice 1 defines two: link framing (version,
  src/dst node id, type, sequence, payload) and a small control message set (e.g.,
  link telemetry).
- Build generates, from the same `.pdl` at the same commit: Rust codecs
  (`pdl-compiler`) and Wireshark Lua dissectors (`pdl-dissector`) — codec and
  dissector cannot drift apart.
- IP/UDP/TCP payloads built with `etherparse`; Wireshark chains built-in dissectors
  after the PDL layers.

### 3.3 Capture and the VisualEther loop

- Per-node PCAPNG (one interface block per NetIf, direction flags set), written by the
  `NetIf` tap — the vantage tcpdump would give on real hardware.
- Frames recorded on `LINKTYPE_USER0`; the generated Lua dissector registers on that
  DLT and chains onward to IP.
- Timestamps: sim time mapped onto a configurable scenario epoch.
- Run output is a self-describing artifact:

```
out/<scenario>/<run-id>/
  nodes/<node>.pcapng        one capture per node
  dissectors/*.lua           exact dissectors matching these captures
  visualether.toml           tshark args loading the Lua → VisualEther MCP
                             tools pick it up automatically
  metrics.ndjson             queue depths, drops, link stats, compute occupancy
  scenario.snapshot.toml     full config + seed + trace-file hashes
```

- AI debugging loop: run → pcaps + dissectors + `visualether.toml` land together →
  `analyze_capture` / `explore` / `extract_sessions` work with no manual setup. FXT
  templates for the PDL protocols ship in-repo once field names stabilize. A merged
  view (mergecap) is an optional post-step; per-node captures are ground truth.

### 3.4 Scenarios and the first slice

**Scenario config.** One TOML per scenario: nodes (type, interfaces, compute params),
media and interface attachments, trace-file references, traffic apps (slice 1: CBR and
UDP-echo generators), duration, epoch, master seed. Traces: CSV per medium,
`t, tx, rx, delay_us, sinr_db`, step-hold.

**First-slice demo.** 2 ground terminals, 3 satellites, 1 gateway. A UDP echo flow
runs terminal → satellite → gateway while the hand-authored trace forces:
(a) a mid-run handover (serving satellite's reachability ends, another's begins),
(b) a degraded-SINR window producing visible BLER losses,
(c) a traffic burst that queues the relay satellite's compute model.

**Success criteria.**
1. Two runs with the same seed produce byte-identical pcaps.
2. Wireshark opens any node capture with a clean dissection chain (no
   malformed/unknown layers).
3. VisualEther `explore` renders the handover as a readable sequence diagram.

### 3.5 Error handling and testing

**Philosophy: configuration errors die at startup; network realities are simulated,
not errored.**

- Startup validation fails fast: dangling node refs, trace coverage gaps inside a
  pair's declared range.
- Pair absent from trace = unreachable (physics, counted, not an error).
- Undecodable received bytes = network reality: drop + counter; the capture still
  records the frame.
- Failure to *encode* = model bug: fail fast.

**Testing, three tiers.**
1. **Unit:** medium delay/BLER math, compute-queue behavior, trace step-hold, codec
   round-trips.
2. **Determinism (CI, always):** same scenario + seed twice → identical outputs.
3. **Golden-run acceptance:** canonical scenarios with committed output digests, plus
   a tshark check that generated dissectors dissect the golden captures with zero
   malformed/unrecognized layers (CI installs tshark).

`NodeBehavior` and `ComputeModel` traits get conformance suites so later real-code and
measured-timing implementations prove they honor the same contracts.

## 4. Out of scope for slice 1

- In-sim orbital propagation (SGP4) — traces are precomputed; an sgp4-backed trace
  generator is a natural slice-2+ item.
- Dynamic routing control plane — slice-1 routing is static/precomputed.
- Real-time execution / hardware-in-the-loop — enabled by nexosim's clocks and event
  injection, deliberately deferred.
- Mobile (4G/5G) and terrestrial verticals — the abstractions (Medium, ChannelTrace,
  SINR→BLER) are designed to host them, but no gNB/UE models in slice 1.
- Scale beyond ~100 nodes, aggregate-flow abstractions.
- 4G/5G UDP-framed capture encapsulations (VisualEther already handles those for
  real captures; relevant when the mobile vertical arrives).

## 5. Risks

| Risk | Mitigation |
|---|---|
| `pdl-dissector` immaturity (v0.1.0, single maintainer) | Validate in slice 1 (risk-retirement item); vendor/patch/contribute as needed |
| PDL expressiveness limits for exotic layers | PDL targets byte-oriented packet formats (its Bluetooth home turf matches link framing well); fall back to hand-written codec + hand-written Lua for any layer PDL can't express |
| nexosim API churn (1.x young) | Thin layer keeps surface small; pin versions; engine-abstraction (Approach B) remains a fallback refactor if churn proves painful |
| Determinism leaks (iteration order, parallel executor edge cases) | Determinism CI test from day 1; all randomness via seeded per-model RNGs; no wall-clock reads in models |

## 6. Open items

- pcapng vantage: per-node capture is decided; whether the tap also records
  medium-dropped frames (BLER losses visible at tx but not rx) — decide during
  implementation planning; leaning: tx-side tap records the send, rx-side tap only
  records deliveries, so losses are visible by diffing vantages, as in real life.
