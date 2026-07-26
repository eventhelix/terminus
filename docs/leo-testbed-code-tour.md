# LEO testbed — a guided code tour

A learning walkthrough of the slice-1 LEO testbed codebase, starting from the
discrete-event simulation (DES) foundation and building up to a full packet
trace. Companion to the design of record
(`docs/specs/2026-07-23-network-simulator-design.md`) and the slice plan
(`docs/superpowers/plans/2026-07-23-slice-1-leo-testbed.md`).

---

## Part 1 — The discrete-event simulation (DES) foundation

### What a discrete-event simulator is

A DES doesn't advance time in fixed ticks. Instead it keeps a **priority queue of
future events ordered by timestamp**, and repeatedly pops the earliest one, runs
its handler, and lets that handler schedule *new* future events. Simulated time
jumps directly from one event to the next — if nothing happens between t=1.0s and
t=5.0s, the simulator leaps straight there. This is why you can simulate 60
seconds of a satellite network in milliseconds of wall-clock: work is proportional
to the *number of events*, not the length of time.

The key consequence: **everything that "takes time" is modeled as scheduling an
event in the future.** A propagation delay of 3.2 ms isn't a `sleep` — it's
"deliver this packet as an event stamped 3.2 ms from now."

### nexosim: the engine this project uses

helixsim uses [nexosim](https://github.com/asynchronics/nexosim) 1.x as its DES
engine. Once you see its core concepts, every file in `crates/core/` reads the
same way:

- **Model** — a stateful object that reacts to inputs. Every simulated thing here
  is a nexosim model: `NodeModel`, `NetIf`, `Medium`, `FifoCompute`, `Recorder`.
  A model is a Rust struct with an `#[Model]` impl block.
- **Input method** — an `async fn` on the model that other models call to send it
  something (`Medium::transmit`, `NetIf::rx`, `FifoCompute::submit`). These are
  the event handlers.
- **Output port** (`Output<T>`) — a typed "wire" a model sends on. You
  `.connect()` an output to another model's input method at assembly time.
  `output.send(x).await` delivers `x` to every connected input. A model never
  names its peers directly — it just sends on a port wired up externally. This
  decoupling lets the same models form different topologies.
- **Mailbox** — each model instance has one; it queues incoming messages so the
  model processes them one at a time (never re-entered concurrently).
- **`cx.schedule_event(duration, method, arg)`** — *this is where time comes
  from.* A model schedules one of its own methods to fire `duration` from now.
  Propagation delay, service latency, and periodic timers are all this call.
- **`Context<Self>` (`cx`)** — passed to every handler; gives `cx.time()` and
  `cx.schedule_event(...)`.

The simplest real model, `FifoCompute` (`crates/core/src/compute.rs`), shows all
of it:

```rust
pub async fn submit(&mut self, item: RxFrame, cx: &Context<Self>) {   // an input method
    if self.busy < self.cores {
        self.busy += 1;
        cx.schedule_event(                                            // time passes HERE
            Duration::from_nanos(self.service_ns),
            schedulable!(Self::complete),                             // fire this method...
            item.clone(),                                             // ...with this arg
        ).expect(...);
    } ...
}

#[nexosim(schedulable)]
async fn complete(&mut self, item: RxFrame, cx: &Context<Self>) {     // fires service_ns later
    self.done.send(item).await;                                       // push out an output port
    ...
}
```

"Processing a packet takes 2.5 ms" becomes: on `submit`, schedule `complete` for
2.5 ms later; on `complete`, emit the packet on `done`. No threads, no sleeps —
just events.

### How a run actually executes

Assembly (`crates/cli/src/assemble.rs`) builds all models, wires their ports, and
hands them to nexosim via `SimInit`. Then:

```rust
let mut simu = bench.init(t0)...;              // t0 = MonotonicTime::EPOCH; runs each model's init
simu.step_until(t0 + duration)...;             // pop events until sim-clock reaches t0+60s
simu.process_event(&flush, ())...;             // one final event to flush files
drop(simu);                                    // closes pcapng/ndjson files
```

`step_until` is the event loop: pop earliest event → run its handler → the handler
may `send()` on ports (enqueues messages into other mailboxes as *now*-events) and
`schedule_event()` (future events) → repeat until the clock passes 60 s. `init` is
nexosim's startup hook; helixsim uses it to fire each node's `on_start` (e.g. a
terminal schedules its first send timer — `NodeModel::init`, `node.rs:152`).

One determinism-critical detail: `SimInit::with_num_threads(1)`
(`assemble.rs:39`). Multi-threading makes the ordering of same-timestamp events
nondeterministic; single-threaded gives a fixed, reproducible order that the whole
project depends on.

---

## Part 2 — The big picture

Four crates form a dependency stack:

```
crates/protocols/   real wire bytes: PDL link framing + IPv4/UDP (etherparse)
      ^
crates/core/        the nexosim models: Node, NetIf, Medium, Compute, Recorder
      ^
crates/cli/         config loading + bench assembly + output writing; the binary
      ^
crates/scenarios/   data only: scenario.toml + channel-trace CSVs (LEO testbed)
```

**The data flow through one node**, which the whole simulator is built around
(from the `assemble.rs` header comment):

```
NodeModel.to_ifs[i] -> NetIf.tx -> Medium.transmit -> (delay) -> deliveries[j]
   -> NetIf.rx -> FifoCompute.submit -> (service) -> done -> NodeModel.frame_in
                                                               |
                        (behavior decides: reply / forward / drop)
     all NetIf.capture + every model's .metrics ------------> Recorder
```

Read it as: a node's **behavior** decides to transmit -> the **interface** taps it
for capture and hands it to the **medium** -> the medium consults the **channel
trace** for reachability/delay and the **BLER curve** for loss, then schedules
delivery -> the receiver's interface taps it again and passes it to its
**compute** model for processing latency -> compute hands the frame to the
receiving node's behavior. Everything observable is copied to the **Recorder**,
which writes the PCAPNG and metrics files.

The one rule that governs the entire design (`CLAUDE.md` invariants +
`packet.rs:1`): **the inter-node unit is always real encoded bytes.**
`PacketMeta` (id, birth time, origin) rides along purely for observability — no
model may branch on it. Behavior derives *only* from decoding the actual `bytes`.
That is what makes captures real and what will later let you swap a simulated node
for real software.

---

## Part 3 — The core crate, module by module

Bottom-up, in dependency order.

### `packet.rs` (36 lines) — the wire unit
Defines `Packet { bytes: Vec<u8>, meta: PacketMeta }` and the two envelopes that
move between models: `Transmission` (NetIf->Medium, carries `tx_node`) and
`RxFrame` (NetIf->Compute->Node, carries `if_index`). The module doc states the
hard rule about meta being observability-only. `PacketMeta.id` is
`(origin_node << 48) | per-node counter` — globally unique within a run and
*preserved across relay hops*, so you can follow one packet end-to-end in the
metrics.

### `simtime.rs` (31 lines) — time helpers
`now_ns(cx)` converts nexosim's clock to "ns since sim start"; `secs_to_ns(f64)`
converts config seconds to integer ns at a **single rounding point** (so float
quirks can't differ between call sites — a determinism concern). The sim always
starts at `EPOCH`; the real-world epoch (`epoch_unix_s`) is applied only later
when the Recorder stamps PCAPNG timestamps.

### `rng.rs` (60 lines) — deterministic randomness
The determinism backbone. `derive_seed(master, path)` hashes a model's stable path
string (e.g. `"medium:access"`) with a **hand-rolled FNV-1a + splitmix64** and
mixes it with the scenario master seed. `model_rng` turns that into a
`ChaCha12Rng`. Every stochastic draw (only BLER drops, in slice 1) flows through
one of these. The hash is hand-rolled deliberately — Rust's `DefaultHasher` isn't
stable across releases/platforms, which would break byte-identity. Same seed +
same path => same random stream, everywhere, forever.

### `records.rs` (65 lines) — observability types
`CaptureRecord` (a tapped frame: node, if, time, direction, bytes) and
`MetricRecord` (one NDJSON line: time, source, event, optional
packet_id/queue_len/value_ns). The doc comment (`records.rs:22`) is the **full
event vocabulary** the sim emits, grouped by source prefix (`medium:`, `compute:`,
`netif:`, `node:`). `MetricRecord` uses a builder pattern (`.packet(id).queue(n)`)
so behaviors attach only the fields they have.

### `trace.rs` (197 lines) — channel traces (connectivity as data)
How "physics" enters the sim without being hard-coded. A `ChannelTrace` maps each
directed `(tx, rx)` pair to a **time-ascending series of `{delay_ns, sinr_db}`
samples**. Lookups are **step-hold** (`query`, `trace.rs:93`): the last sample
with `t <= now` governs; no interpolation. Two special cases encode physics as
data:
- A row with **both value fields empty** = "unreachable from here on" sentinel.
- A pair with **no rows at all** = never reachable.

`from_csv` is strict — it rejects out-of-order rows, half-empty rows, zero delays,
negative times, unknown node names. That strictness is the "config errors die at
startup" invariant in action. Parsing turns node *names* into ids via a
`name_to_id` map.

### `bler.rs` (59 lines) — SINR->loss curve
A tiny step table: rows sorted by ascending SINR; `bler(sinr)` returns the last
row with `sinr <= x`, and below the first row BLER is 1.0 (no link). The pluggable
"how likely is this frame to be corrupted at this signal quality" function. The
LEO scenario gives `access` and `feeder` different curves
(`scenario.toml:64,69`).

### `compute.rs` (196 lines) — processing latency
`FifoCompute`: N cores + a bounded FIFO. `submit` -> if a core's free, schedule
`complete` at `now + service_ns`; else queue; else (queue full) drop and emit
`drop_overflow`. `complete` emits the frame on `done` and pulls the next queued
item. This turns "the satellite's CPU can handle ~400 frames/s" into real queue
overflow under a burst. Its header comment is the project's canonical example of
the nexosim self-scheduling pattern. The "compute port contract" idea: later you
can replace this with a software-architecture model or real measured timings by
exposing the same ports.

### `netif.rs` (160 lines) — the network interface
Deliberately dumb (like a NIC). Two jobs: (1) be the **capture tap** — every `tx`
and `rx` emits a `CaptureRecord` (your tcpdump vantage point), and (2) hold
**up/down** state. Crucial asymmetry (`netif.rs:1`): the **tx tap records every
send even if the medium later drops it**, but the **rx tap only records
deliveries** — so packet loss is visible by *diffing the two vantages*, exactly as
on real hardware. A down interface swallows traffic and emits `tx_down`/`rx_down`.

### `medium.rs` (222 lines) — the shared channel (the physics engine)
One `Medium` per link domain — where trace + BLER + RNG combine. On `transmit(tx)`
at time t (`medium.rs:63`), for each attached receiver != sender:
1. `trace.query(tx, rx, t)` -> reachable? If not, skip (counted as `unreachable`
   if *nobody* was reachable).
2. Compute `bler(sample.sinr_db)`; draw `rng.random::<f64>()`; if below BLER ->
   `drop_bler`, skip.
3. Otherwise `schedule_event(delay_ns, deliver, packet)` — the propagation delay.

`deliver` (`medium.rs:98`) fires after the delay and pushes the packet onto that
receiver's delivery port. **Handover falls out for free**: it's just reachability
changing between one packet and the next as the trace advances. The `feeder`
medium is a broadcast domain — *every* satellite hears the gateway's replies (see
`smoke.rs:48`).

### `node.rs` (171 lines) — the node assembly and the behavior boundary
The conceptual heart. A **node = behavior + one NetIf per interface + a compute
model**, composed at assembly time. The important move is the split between
`NodeModel` (the nexosim model) and `NodeBehavior` (where "what this node does"
lives):

- **`NodeBehavior` trait**: `on_start`, `on_frame`, `on_timer`. The entire
  contract.
- **`BehaviorCtx`**: the *only* thing a behavior may touch. Exposes `now_ns`, a
  seeded `rng`, and buffered action methods: `transmit_new(if, bytes)` (mint a
  packet), `forward(if, packet)` (relay preserving meta), `timer_in(id, delay)`,
  `metric(event)`. A behavior **cannot** touch ports, the clock, or files
  directly.
- **`Actions`**: behaviors don't *do* things; they *return* a buffered list of
  transmits/timers/metrics. `NodeModel::apply` (`node.rs:134`) performs them
  against nexosim.

Why the indirection matters (`node.rs:1`): a behavior is a pure function of
`(its state, ctx inputs) -> Actions`, with no I/O and no clock access. So you can
(a) unit-test it with zero simulation, (b) check it for determinism, and (c)
**later replace the hand-written behavior with wrapped real production software**
exposing the same trait — the "model-in-the-loop -> software-in-the-loop" path.
`drive_behavior` (`node.rs:79`) runs a behavior against a `BehaviorCtx` outside any
simulation — used by `NodeModel`, unit tests, and the conformance suite alike.

`NodeModel` plumbs nexosim events into the behavior: `init`->`on_start`,
`frame_in`->`on_frame`, `timer`->`on_timer`, each wrapped by `drive` (build ctx) +
`apply` (flush actions). Zero-delay timers are clamped to 1 ns because nexosim
rejects zero-delay scheduling (`node.rs:139`).

### `conformance.rs` (37 lines) — the determinism check for behaviors
`assert_behavior_deterministic` runs two clones of a behavior through the same
`on_start -> on_timer(SEND) -> on_timer(TELEMETRY)` sequence with identical seeded
RNG and asserts the `Actions` match. The reusable guard any future behavior —
including wrapped real code — must pass.

### `behaviors.rs` (405 lines) — the three slice-1 node behaviors
Concrete `NodeBehavior` implementations, unified by a `BehaviorKind` enum (so
`NodeModel` holds one serializable type):

- **`TerminalApp`** — UDP echo client. On a `SEND` timer it builds a real IPv4/UDP
  packet (`build_udp_ipv4`) wrapped in a `DataFrame`, stamps a 4-byte big-endian
  sequence number into the payload, records the send time in `sent`, and
  reschedules by `1e9 / rate`. Supports a **burst window** (`rate_at`) that
  temporarily raises the rate. On `on_frame`, decodes the link frame, checks it's
  addressed here, matches the reply's seq against `sent`, and emits `echo_rtt`.
- **`Relay`** (satellite) — On a frame *not* addressed to it, forwards out the
  "other" interface via a static `if_map` (`[1,0]` = access<->feeder) using
  `forward` (meta preserved). On a `TELEMETRY` timer, emits a `ControlFrame`
  toward the gateway periodically. Undecodable bytes -> `decode_error`, dropped.
- **`GatewayEcho`** — UDP echo server. On a `DataFrame` to its port, swaps src/dst
  IP and port and echoes the payload back byte-identically in a new `DataFrame`.
  `ControlFrame`s (telemetry) are counted as `telemetry_rcvd`.

Addressing scheme (`behaviors.rs:3`): IPs are the fixed `10.0.0.<node_id>`, which
is why config validates `id <= 250`. Error discipline: **decode failures on
received bytes are counted metrics; encode failures are `.expect()`** (a decode
failure is a simulated network reality; an encode failure is a model bug).

### `capture.rs` (208 lines) — the Recorder (run artifacts)
The sink model that writes the files. It's a nexosim `ProtoModel` (a builder that
opens files at bench-build time, in `ProtoRecorder::build`) producing a `Recorder`
model with a `RecorderEnv` holding one `PcapNgWriter` per node. `capture` writes an
`EnhancedPacketBlock` (with inbound/outbound flags); `metric` appends an NDJSON
line; `flush` flushes at the end. The header comment lists three
**byte-identity-critical** requirements CI enforces: (1) `with_endianness(..,
Little)` never `new()`, (2) every interface block carries `IfTsResol(9)` for
nanosecond timestamps, (3) frames use `LINKTYPE_USER0` (147), which the generated
Lua dissector registers on. The `epoch_ns` offset is added here — the only place
sim-time meets wall-clock.

---

## Part 4 — protocols, cli, and the scenario

### `crates/protocols/` — real bytes on the wire
The distinctive property of helixsim: **one `.pdl` file is the single source of
truth for both the Rust codec and the Wireshark dissector**, so they can't drift.

- **`pdl/link.pdl`** (35 lines) — the link-layer framing. A `LinkFrame` has a
  fixed `0x48` magic byte, a version nibble, a `frame_type` enum, 16-bit src/dst
  node ids, a 32-bit seq, a 16-bit body size, then a `_body_`. `DataFrame`
  (frame_type=DATA) adds `flow_id` + `_payload_` (the real IP/UDP bytes);
  `ControlFrame` (CONTROL) adds `opcode` + `args`. A tagged-union wire format:
  decode the parent, look at `frame_type`, specialize to the child.
- **`build.rs`** (11 lines) — at compile time, runs `pdl-compiler` to generate the
  Rust codec into `OUT_DIR/link_gen.rs`, which `src/lib.rs` `include!`s as the
  `link` module. `DataFrame`, `LinkFrame`, `LinkFrameChild`, `.encode_to_vec()`,
  `.decode_full()`, `.specialize()` are all generated.
- **`src/udp.rs`** (74 lines) — `build_udp_ipv4` / `parse_udp_ipv4` using
  `etherparse`. Real IPv4+UDP headers with real checksums, no Ethernet (the
  LinkFrame *is* the link layer). `parse` returns `Option` — malformed bytes are
  `None` (a counted reality), never an error.
- **`dissectors/link.lua`** (456 lines) — the Wireshark dissector, generated
  **offline** from the same `.pdl` by `tools/regen-dissector.sh` and checked in.
  It registers on `LINKTYPE_USER0` and chains onward to Wireshark's built-in IP
  dissector.

### `crates/cli/` — turning config into a run
- **`config.rs`** (524 lines) — loads and *aggressively validates* the scenario.
  The "config errors die at startup" invariant made concrete: ~15 typed error
  variants (dangling medium/peer refs, kind<->section<->interface-count mismatch,
  double-attach, unattached trace pairs, orphan media, bad values...). Check
  ordering is deliberate and commented — e.g. duplicate ids are caught *before*
  peer-ref checks so you get the right error. It also SHA-256s each trace file for
  the snapshot. Anything that survives `load()` is guaranteed simulatable.
- **`assemble.rs`** (200 lines) — the wiring. Builds the Recorder, then all media,
  then per node a `NodeModel`, a `FifoCompute`, and one `NetIf` per interface —
  connecting every port per the data-flow diagram. `build_behavior` maps config ->
  `BehaviorKind`. Then it moves everything into the bench, runs
  `step_until(duration)`, flushes, and drops. `MBOX = 4096` (`assemble.rs:29`):
  the graph is cyclic (node->if->medium->if->compute->node), and bounded mailboxes
  + cycles can deadlock under bursts, so capacities are generous.
- **`output.rs`** (145 lines) — writes the **self-describing run directory**:
  `dissectors/link.lua` (embedded at compile time via `include_str!`, so it
  matches this build's codec), a `visualether.toml` pointing tshark at that
  dissector (absolute, forward-slashed path — a Windows detail), and
  `scenario.snapshot.toml` (version + trace hashes + full scenario). The point
  (design 3.3): open any run in Wireshark / the VisualEther MCP tools with zero
  manual setup.
- **`main.rs`** (44 lines) — the `helixsim run <scenario> --out <dir>` CLI. The
  only wall-clock read in the codebase is here, naming the output dir
  `run-<unix-secs>` — explicitly *outside* the simulation so run contents stay
  byte-deterministic (`main.rs:30`).

### `crates/scenarios/leo-testbed/` — the demo
`scenario.toml` defines 6 nodes: two terminals (`term-a`, `term-b`), three
satellites (`sat-1/2/3`), one gateway (`gw`), across two media (`access` =
terminal<->satellite, `feeder` = satellite<->gateway). The trace CSVs script three
demo features the smoke test verifies:
- **(a) Handover** — `access.csv:10-13`: at t=30s `term-a<->sat-1` closes
  (sentinel) and `term-a<->sat-2` opens. Echoes keep completing => handover works.
- **(b) Degraded SINR** — `access.csv:6`: at t=18s `term-a->sat-1` SINR drops to
  0 dB (BLER 0.30) until t=24s => visible `drop_bler` losses.
- **(c) Compute burst** — `scenario.toml:24`: `term-b` bursts to 500 pps during
  40-45s, overflowing `sat-3`'s single 2.5 ms/frame core (~400 frames/s capacity)
  => `drop_overflow`.

---

## Part 5 — Determinism, and how it's all enforced

The headline invariant: **same scenario + same seed => byte-identical outputs.**
Every design choice serves it:

| Mechanism | Where | Why |
|---|---|---|
| Single-threaded engine | `assemble.rs:39` | fixed same-timestamp event ordering |
| Path-derived seeded RNGs | `rng.rs` | reproducible draws; stable cross-platform hash |
| No wall-clock in models | only `main.rs`, for the dir name | models can't observe real time |
| Single rounding point for time | `secs_to_ns` | float->int identical everywhere |
| Little-endian pcapng + `IfTsResol(9)` | `capture.rs` | byte-identical files across platforms |
| `BTreeMap`/`BTreeSet` everywhere | config, medium, capture | ordered iteration, not hash-random |

Three integration tests guard it:
- **`determinism.rs`** — runs the LEO scenario twice, asserts every artifact is
  byte-identical (excluding `visualether.toml`, which embeds an absolute path by
  design).
- **`golden.rs`** — SHA-256s the canonical run against committed digests in
  `golden.sha256`; on an intentional behavior change you regenerate with
  `UPDATE_GOLDEN=1`.
- **`smoke.rs`** — asserts the three demo features (a/b/c) are visible in the
  metrics, plus RTTs land in a sane ~10-60 ms band and pcaps are non-trivial.

Plus `dissection.rs` (feeds a capture through tshark with the generated dissector)
and the per-crate unit tests inline in each module.

---

## How to explore it

1. **Run it** — `cargo run -p helixsim -- run crates/scenarios/leo-testbed/scenario.toml`
   produces `out/leo-testbed/run-<ts>/`. Open `metrics.ndjson` and watch the
   echo/forward/drop events; open a `nodes/*.pcapng` in Wireshark with the bundled
   dissector.
2. **Read in this order** — `packet.rs` -> `node.rs` (the behavior boundary) ->
   `behaviors.rs` (what nodes do) -> `medium.rs` (the physics) -> `assemble.rs`
   (how it's wired) -> `config.rs` (what's validated).
3. **Trace one packet** end-to-end: `TerminalApp::send_echo` -> `NodeModel::apply`
   sends on `to_ifs[0]` -> `NetIf::tx` (captures + forwards) -> `Medium::transmit`
   (trace+BLER+delay) -> `Medium::deliver` -> `NetIf::rx` (captures) ->
   `FifoCompute::submit`/`complete` (latency) -> `NodeModel::frame_in` -> the
   relay's `on_frame` forwards it feeder-side -> ... -> gateway echoes -> the
   reverse path home -> `TerminalApp::on_frame` records `echo_rtt`.

For the deeper "why," see the design of record
(`docs/specs/2026-07-23-network-simulator-design.md`) and the slice plan
(`docs/superpowers/plans/2026-07-23-slice-1-leo-testbed.md`).
