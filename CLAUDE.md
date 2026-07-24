# helixsim — Project Context

## What this is

helixsim is an open-source Rust discrete-event simulator for networking and
wireless systems (satellite, mobile, terrestrial). Two missions:

1. **Test and integration platform** — exercise system software against
   simulated nodes and links before hardware exists.
2. **Performance simulation** — latency, throughput, loss, and load behavior
   under time-varying link conditions.

Distinctive property: every simulated exchange is **real protocol bytes**,
captured to standard PCAPNG with matching Wireshark dissectors, so any run can be
opened in Wireshark and analyzed by AI through the VisualEther MCP tools.

The design of record is `docs/specs/2026-07-23-network-simulator-design.md`. Read
it before making architectural changes.

## Engine and key dependencies

- **DES engine:** [nexosim](https://github.com/asynchronics/nexosim) 1.x, used
  directly. Nodes, protocol layers, and media are all nexosim models.
- **Custom protocols:** [Google PDL](https://github.com/google/pdl) — one `.pdl`
  file generates both Rust codecs (`pdl-compiler`) and Wireshark Lua dissectors
  (`pdl-dissector`).
- **Standard protocols:** real IP/UDP/TCP via `etherparse`, carried inside the
  PDL framing.

## Workspace layout

```
crates/core/       Packet, node/interface traits, medium, channel trace,
                   compute model, capture — all nexosim models
crates/protocols/  PDL sources + generated codecs/dissectors; etherparse builders
crates/scenarios/  scenario TOML + trace files (LEO testbed is the first slice)
crates/cli/        the `helixsim` binary: run a scenario -> output directory
```

## Build commands

```bash
cargo check
cargo test
cargo build --release
```

## Invariants (do not break)

- **Real bytes everywhere.** The inter-node unit is always an encoded byte
  buffer. Packet metadata is observability-only — no model may branch on it.
- **Determinism.** One master seed per scenario; all randomness flows through
  per-model seeded RNGs. Same scenario + seed ⇒ byte-identical outputs. There is
  a CI test for this; keep it green. No wall-clock reads inside models.
- **Config errors die at startup; network realities are simulated.** Dangling
  refs and trace-coverage gaps fail fast. Loss, unreachability, and undecodable
  received bytes are counted, not errored.

## Commit conventions

- **No AI/Claude attribution in commits or PRs.** Do not add `Co-Authored-By:
  Claude ...` trailers or "Generated with Claude Code" lines. This is enforced by
  `.claude/settings.json` (`includeCoAuthoredBy: false`); do not re-enable it and
  do not add attribution manually.
- Conventional-commit style subjects (`feat:`, `fix:`, `docs:`, `chore:`).
