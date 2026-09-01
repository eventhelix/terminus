# terminus — Project Context

## What this is

terminus is a source-available Rust discrete-event simulator for networking and
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

## The Terminus blog series

The flagship application: a blog series (→ book) designing a constellation
for a civilization on a tidally locked planet, staged on the eventhelix.com
site repo's `terminus` branch (the `site` repo, checked out alongside this one).
Series design of record: `docs/specs/2026-08-21-terminus-series-design.md`.
Canon — world bible, requirements baseline (`TER-REQ-*`), ADRs, compliance
matrix, manuscript map, and style rules — lives in `docs/terminus/`; read it
before touching series content. Every number a post quotes must be
reproducible from a tagged run (`terminus-post-N` tags pin the evidence).

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
crates/orbits/     terminus-orbits: closed-form orbital/link/reliability
                   screening math; its examples are the Terminus posts'
                   evidence artifacts
crates/protocols/  PDL sources + generated codecs/dissectors; etherparse builders
crates/scenarios/  scenario TOML + trace files (LEO testbed is the first slice)
crates/cli/        the `terminus` binary: run a scenario -> output directory
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
- **The crates stay generic and independently usable.** The repository is named
  after the Terminus series, but no Terminus/Proxima concepts, constants, or
  narrative may appear in any crate — planets, stars, and constellations are
  configuration. The series lives only in `docs/terminus/`, scenario data, and
  the site repo. This is load-bearing for the licensing plan below: a crate can
  only be carved out and released on its own if it carries no series narrative.
  `crates/orbits/examples/` is the one exception — those examples are the posts'
  evidence artifacts and may name Terminus concepts freely. Library code under
  any `src/` may not.

## Licensing

Two licenses, split by artifact type:

- **Code — AGPL-3.0-only** (`LICENSE`). Every Rust source file carries an
  `SPDX-License-Identifier: AGPL-3.0-only` header; keep it when adding files.
  AGPL is an OSI-approved open-source license, so terminus *is* open source —
  but do not describe it as "permissively licensed" or imply MIT/Apache terms.
- **Terminus series canon — CC BY-NC-ND 4.0** (`docs/terminus/LICENSE`).
  Applies to everything under `docs/terminus/`. Do not add SPDX headers there.

`NOTICE` records the license of record and explains why `LICENSE-MIT` and
`LICENSE-APACHE` appear in the git history; leave it in place.

Dependencies are all permissive (MIT, Apache-2.0, BSD, Zlib, Unlicense) and
therefore AGPL-compatible. **Before adding a dependency, check its license** —
anything more restrictive than Apache-2.0 needs a decision, not a `cargo add`.

The plan is to **carve individual crates out into their own repositories** when
they are ready to publish, relicensing each carve-out permissively at that
point. EventHelix.com Inc. holds all copyright, so nothing blocks that; keeping
the crates generic (above) is the whole preparation. Note the crates.io name
`terminus` is already taken by an unrelated package, so the CLI package needs a
new name before any publication.

## Commit conventions

- **No AI/Claude attribution in commits or PRs.** Do not add `Co-Authored-By:
  Claude ...` trailers or "Generated with Claude Code" lines. This is enforced by
  `.claude/settings.json` (`includeCoAuthoredBy: false`); do not re-enable it and
  do not add attribution manually.
- Conventional-commit style subjects (`feat:`, `fix:`, `docs:`, `chore:`).
