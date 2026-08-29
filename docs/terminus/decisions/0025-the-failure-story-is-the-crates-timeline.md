# ADR-0025: The failure story is presented as the crate's timeline, exported

Status: accepted
Date: 2026-08-29
Requirements: TER-REQ-003, TER-REQ-004
Evidence: `cargo run --release -p terminus-orbits --example failure_timeline`; `cargo run --release -p terminus-orbits --example feeder_terminals` (section G)

## Decision

The published account of a feeder-telescope failure is a **single canonical
timeline computed by the crate** — break, declaration at 3 × 100 ms of missed
heartbeats, the detour at the degraded budget, and a 5 s spare acquisition
racing the re-anchor policy — together with the four outcomes the two remedies
(spare, hold-off) produce. The `failure_timeline` example assembles and prints
it from `routing::feeder_route`, `routing::RELAY_DELAY`,
`routing::ISL_REACQUIRE` and the section-G walk. Presentations of the failure
story (prose, 2D figures, the 3D console) render this timeline; none may
animate a behaviour or quote a number the example does not produce.

## Why

**The first driven session of the 3D console showed the failure was invisible
in the scene.** The state machine was right — the readouts told the story —
but nothing in the canvas distinguished a broken sky from a healthy one. The
remedy is not a better scene; it is naming the thing the argument actually is:
a topology plus a timeline. A timeline is data, and data has one source of
truth. ADR-0023 already forced every latency to carry its relays; ADR-0024
already priced the spare and the hold-off as one purchase. This ADR closes the
loop: the composed story those decisions imply is itself computed, printed,
and traceable, rather than re-derived by every rendering that tells it.

## Consequences

- **A rendering that needs a number the example does not print must first
  extend the example.** The site's fixture mirrors the printed values under
  assertions; drift fails a test rather than publishing.
- **The 113-session figure stays with `feeder_terminals` section G**, which
  owns the session simulation; `failure_timeline` owns the detour, the race,
  and the outcomes.
- **The 3D console remains the full interactive model** (ADR-0024's toggles);
  the timeline does not replace it, it disciplines it.
