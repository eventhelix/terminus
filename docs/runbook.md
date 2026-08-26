# terminus Runbook

How to run, inspect, and reproduce simulations. Commands are run from the
repository root; `bash` examples work in Git Bash on Windows.

## Prerequisites

- **Rust** ≥ 1.79 (`cargo build` handles everything else).
- **Wireshark / tshark** — optional, for inspecting captures and the dissection
  acceptance test. On Windows: `C:\Program Files\Wireshark`.
- **pdl-dissector** — only needed when regenerating the Wireshark dissector
  after editing `crates/protocols/pdl/link.pdl` (`cargo install pdl-dissector`).

## Run a scenario

```bash
cargo run -p terminus -- run crates/scenarios/leo-testbed/scenario.toml --out out
```

Prints the run directory on success, e.g. `out/leo-testbed/run-1784906550`.
The run-id is a wall-clock stamp (naming only); the *contents* are fully
deterministic — same scenario + seed ⇒ byte-identical files, every time, on
every platform.

Configuration errors (dangling refs, bad values, malformed traces) fail
immediately at startup with a typed error. Network realities — loss,
unreachability, undecodable bytes — never fail a run; they are counted in
`metrics.ndjson`.

## Anatomy of a run directory

```
out/<scenario>/<run-id>/
  nodes/<node>.pcapng        one capture per node — the vantage tcpdump would
                             give on that box (tx side records sends including
                             frames later lost; rx side records only deliveries,
                             so losses are visible by diffing vantages)
  dissectors/link.lua        the exact Wireshark dissector matching these bytes
  visualether.toml           tshark args — VisualEther MCP tools pick this up
                             automatically (embeds an absolute lua path; a moved
                             run dir needs a rerun or a hand-edit of that path)
  metrics.ndjson             one JSON object per event (see vocabulary below)
  scenario.snapshot.toml     full resolved config + master seed + trace hashes
```

## Inspect a run

**Wireshark** (frames dissect as terminus → IP → UDP):

```bash
wireshark -X lua_script:<run-dir>/dissectors/link.lua <run-dir>/nodes/term-a.pcapng
```

**tshark one-liners:**

```bash
# Protocol chain per frame
tshark -r <run-dir>/nodes/term-a.pcapng -X lua_script:<run-dir>/dissectors/link.lua \
       -T fields -e frame.number -e frame.protocols | head

# Any malformed frames? (should print nothing)
tshark -r <run-dir>/nodes/gw.pcapng -X lua_script:<run-dir>/dissectors/link.lua \
       -Y '_ws.malformed || _ws.expert.severity == "Error"'
```

**metrics.ndjson** greps well. Event vocabulary by source prefix:

| Source | Events |
|---|---|
| `medium:<name>` | `tx`, `delivered`, `drop_bler`, `unreachable` |
| `compute:<node>` | `submit`, `done` (with `queue_len`), `drop_overflow` |
| `netif:<node>:<if>` | `tx_down`, `rx_down` |
| `node:<name>` | `echo_sent`, `echo_rtt` (`value_ns`), `echo_reply`, `forward`, `telemetry_sent`, `telemetry_rcvd`, `decode_error` |

```bash
grep echo_rtt metrics.ndjson | wc -l          # completed round trips
grep drop_bler metrics.ndjson | head          # radio losses (when/which packet)
grep drop_overflow metrics.ndjson | wc -l     # compute-queue drops
grep '"compute:sat-3"' metrics.ndjson | grep -o '"queue_len":[0-9]*' | sort -t: -k2 -n | tail -1
```

**VisualEther / AI analysis:** the `visualether.toml` at the run root means the
VisualEther MCP tools (`analyze_capture`, `explore`, `extract_sessions`) work on
any `nodes/*.pcapng` with no manual setup — point them at the pcap and go.

## Test suites

```bash
cargo test --workspace                                  # unit + determinism + golden + smoke
cargo test -p terminus --test dissection -- --ignored   # tshark acceptance (needs tshark on PATH)
```

- **determinism** — runs the LEO scenario twice, asserts byte-identical outputs.
  A failure here is a serious bug (a determinism leak), never "flaky".
- **golden** — sha256 of pcaps + metrics vs `crates/scenarios/leo-testbed/golden.sha256`.
  After an *intentional* behavior change:
  `UPDATE_GOLDEN=1 cargo test -p terminus --test golden`, then commit the
  regenerated file alongside the change.
- **smoke** — asserts the demo narrative: handover at t=30s, BLER losses in the
  18–24s window, compute overflow during the 40–45s burst.
- **dissection** — zero malformed frames + UDP chaining across all six pcaps.
  Runs in CI (Ubuntu installs tshark); locally it is `#[ignore]`d unless invoked
  with `-- --ignored`.

## Regenerate the Wireshark dissector

Only after editing `crates/protocols/pdl/link.pdl`:

```bash
cargo install pdl-dissector        # once
bash tools/regen-dissector.sh
cargo test -p terminus --test dissection -- --ignored   # re-verify
```

The script generates the Lua, applies a **fail-loud patch** for a pdl-dissector
v0.1.0 codegen bug (fractional byte-offset on packed nibble fields), and appends
the glue from `dissectors/link_glue.lua`. Never hand-edit
`crates/protocols/dissectors/link.lua` — regeneration overwrites it. If the
script aborts with a patch-mismatch error, upstream codegen changed: re-verify
against the dissection test before committing.

## Reproduce a past run

`scenario.snapshot.toml` in any run directory holds the fully resolved config,
master seed, and the sha256 of every trace file. To reproduce: check out a
commit where the trace hashes match, run the same scenario TOML, and the outputs
will be byte-identical. (Golden digests generated on Windows are verified on
Linux CI — reproduction is cross-platform.)

## Write a new scenario

Copy `crates/scenarios/leo-testbed/` as a template. One TOML + one trace CSV per
medium (`t_s,tx,rx,delay_us,sinr_db`; step-hold between samples; a row with both
value fields empty closes the pair from that time onward; an absent pair is
never reachable). The full schema and every validation rule live in
`crates/cli/src/config.rs`; the design of record is
`docs/specs/2026-07-23-network-simulator-design.md`.
