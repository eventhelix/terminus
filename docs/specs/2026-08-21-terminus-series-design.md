# Terminus Blog Series — Design

Status: approved in discussion, pending review of this document.
Date: 2026-08-21

## Purpose

Develop a blog series, eventually a book, that designs a satellite constellation
("Terminus") serving a budding civilization on a tidally locked Proxima-b-like
planet. The series is part science fiction, part real engineering: an Alien AI
issues an RFP to provide planet-wide LLM access (satellites it can manufacture
and deploy; ground terminals it can parachute in as WiFi base stations), and the
series is written as the winning contractor's technical proposal.

Every technical claim in the series is backed by a reproducible helixsim
simulation. The trade-study corpus in
[helixsim issue #2](https://github.com/eventhelix/helixsim/issues/2) is the raw
material.

## Governing decisions

These were settled in the brainstorming discussion:

1. **Sim-backed from the start.** Every technical post ships with a
   reproducible helixsim scenario or analysis run. The proposal cites only
   numbers the repo can regenerate.
2. **In-universe frame: we are the bidder.** The series is Terminus Systems'
   proposal responding to the AI's RFP. Posts are proposal sections; helixsim
   runs are the supporting analysis. The RFP post assigns requirement IDs that
   every later post traces to.
3. **Publishing venue.** Final posts are published on eventhelix.com (the Zola
   site at `repos/site`), in a new `content/terminus/` section. Prose follows
   that repo's `CLAUDE.md` and its `elements-of-style` skill: plain English,
   concise and direct, unfamiliar concepts introduced in plain language before
   specialized terminology, no coined terms or hype.
4. **helixsim stays independent.** helixsim remains a general-purpose,
   independently usable network/wireless simulator, publishable as crates.
   Terminus is its flagship application, not its identity.

## The independence principle

helixsim is the engine; Terminus is a consumer. Concretely:

- **No Terminus concepts in engine crates.** `core`, `protocols`, the CLI, and
  any new analysis crates carry no Terminus-specific types, names, constants,
  or narrative. A different user must be able to pick up the crates and
  simulate an Earth LEO constellation without ever meeting Proxima b.
- **Planet and star are configuration, not code.** The orbital-geometry work
  (below) is parameterized: body radius, mass, rotation period, stellar
  parameters all come from scenario/analysis config. Proxima b is one TOML
  file; Earth is another.
- **Terminus specifics live in data and docs.** Scenario definitions
  (`crates/scenarios/terminus-<slug>/`), canon documents (`docs/terminus/`),
  and blog prose (site repo) — none of which ship in a published crate.
- **Publishable-crate hygiene is a tracked work item.** Workspace crates get
  publishable names (e.g. `helixsim-core`), metadata, and docs before any
  crates.io release. Feature work for the series must not block or break
  standalone usability (`cargo test` green, determinism CI green, examples
  runnable without Terminus context).

## Two repos, one source of truth

**helixsim owns the facts; the site owns the prose.**

### Canon (in `helixsim/docs/terminus/`)

- **World bible** — planet, star, and civilization parameters. Reference
  model from issue #2: radius 6,371 km, 1 Earth mass, 11.2-day synchronous
  rotation, Proxima-like star with coherent radio emission near 1–3 GHz.
- **Requirements baseline** — the RFP's requirements, each with a stable ID
  (`TER-REQ-NNN`). Posts trace to IDs instead of restating requirements.
- **Decision log** — one short ADR per settled trade (fixed-plane over
  terminator tracking, Ka primary with X/Ku diversity, MEO compute with
  session anchoring, FEC behind a common trait, shared-MEO PNT, ...).

Canon lives in helixsim rather than the site because it must version-lock with
the scenarios that produce its numbers; `docs/` is not part of any published
crate, so this does not compromise independence.

### Reproducibility contract

Each post's frontmatter (site repo) records the helixsim scenario name, master
seed, and git tag that produced its numbers. helixsim's determinism invariant
(same scenario + seed ⇒ byte-identical outputs) is the series' fact-checking
mechanism. A post never cites a number the repo cannot regenerate.

## Series decomposition

Three series; each becomes a part of the book, with canon docs as appendices.

### Series 1 — The RFP and the Architecture Proposal (~8–10 posts)

1. **The RFP** — in-universe document from the Alien AI: mission, what the AI
   provides, service requirements with IDs (terminator-band coverage,
   interactive-LLM latency, availability, PNT, ground segment constraints),
   evaluation criteria. Load-bearing post: defines the traceability spine.
2. **Know your planet** — Proxima b reference model; why the civilization
   lives on the terminator; why Mercury (3:2 resonance) is a false analog.
3. **The seductive wrong answer** — active terminator-tracking orbits; the
   ~4–6 km/s/day Δv analysis, quantified and rejected.
4. **Orbital regime screening** — VLEO → stationary sweep; latency, footprint,
   handover burden; Hill-sphere stability check eliminates stationary orbit.
5. **The access constellation** — fixed-plane handoff, the 6 planes × 12
   satellites LEO seed at 1,800 km; coverage, dwell, and handover simulation.
6. **Where does the LLM live?** — access/compute/state layering, session
   anchoring, MEO compute, KV-cache migration economics.
7. **Talking past a flaring red star** — link budgets, frequency plan, why
   L/S band is excluded; Ka primary + X/Ku diversity.
8. **The proposal summary** — compliance matrix against the RFP, open risks,
   future work priced in.

### Series 2 — Transport and reliability

The ten-step FEC progression from issue #2 (§19): plain loss → ARQ → RaptorQ
generations → adaptive overhead → handover-aware bursts → Reed-Solomon →
sliding-window FEC → frequency diversity → satellite diversity → RLNC. One
engineering question per post; this is where helixsim's packet-level machinery
(real bytes, PCAPNG, dissectors) is the star.

### Series 3 — PNT and timing

Pseudorange → position/clock solver → GDOP/PDOP over the terminator band →
clock models → relativistic corrections → stellar flares → integrity.

## Per-post development loop

For every technical post, in order:

1. State the engineering question the post answers.
2. Extend helixsim as needed (TDD; real-bytes and determinism invariants
   intact; independence principle respected).
3. Run the scenario; capture metrics, traces, PCAPs.
4. Tag the commit.
5. Write the post in the site repo, citing tagged numbers, following the site's
   writing conventions (`CLAUDE.md`, `elements-of-style`).
6. Cross-check: every number in the prose matches sim output.
7. Publish.

Post and scenario land together.

## New engineering: orbital geometry

Series 1 needs analysis helixsim does not have: orbit propagation, footprint
and elevation geometry, coverage sweeps over the terminator band, handover/dwell
statistics, Δv screening, Hill-sphere checks. This becomes a new generic
analysis crate (working name `helixsim-orbits`), configured entirely by body
and constellation parameters.

Its outputs also feed the packet-level simulator: handover schedules and
time-varying delay/loss become channel-trace CSVs, the shape
`crates/scenarios/*/traces/` already supports. This is the main engineering
investment of Series 1.

## Book path

Posts are written to be chapters: consistent terminology from the canon docs,
requirement-ID traceability, and the illustration conventions recorded in
issue #2 (satellites visibly on their orbital shells; consistent orbit labels:
LEO access ~1,800 km, MEO service/compute/PNT ~20,000 km). A manuscript map in
`docs/terminus/` tracks post → chapter assignments as the series grows.

## First milestones

1. Canon skeleton: world bible + RFP requirements draft (`docs/terminus/`).
2. Orbital-geometry crate spike: reproduce the terminator-tracking Δv numbers
   for post 3.
3. RFP post draft in `site/content/terminus/`.

Detailed sequencing belongs to the implementation plan (next step after this
spec is approved).
