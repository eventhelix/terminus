# ADR-0023: Latency counts relays, and routing compares time

Status: accepted
Date: 2026-08-27
Requirements: TER-REQ-003
Evidence: `cargo run --release -p terminus-orbits --example feeder_terminals` (section H); `routing::RELAY_DELAY`, `placement::one_way_latency`

## Decision

A path's cost is the **time** it takes, not the distance it covers. Every
satellite that forwards a packet — rather than originating or terminating it —
is charged `RELAY_DELAY`, **stated at 0.5 ms**, on top of the light time.

`routing::exit_gateway` picks the gateway with the shorter *time*. The round
trips reported by the anchor-policy sweep (`feeder_terminals` section H)
include the relays: the serving access satellite, plus one for each necklace
hop. The anchor terminates the packet and is not a relay.

## Why

The payload is regenerative. A relay demodulates the frame, decodes its forward
error correction, switches it, re-encodes and re-modulates — work that takes
real time on a spacecraft, and half a millisecond is a fair allowance for it.
Counting only light time made every quoted round trip optimistic by however
many satellites the packet passed through, which is exactly the quantity the
routing gets to choose.

**The figure is a stated guess and every latency downstream moves linearly in
it.** It is not measured and it is not a constant of the sky. It is stated in
one place so that everything quotes one number rather than inventing its own,
the same way `REANCHOR_MARGIN` is.

The scale is worth holding on to. A necklace hop costs 14.8 ms of light and
0.5 ms of processing, so relays are about a thirtieth of what a hop costs. They
change no conclusion by themselves:

| | light only | with relays |
|---|---:|---:|
| p95 round trip at the adopted margin | 177 ms | **178 ms** |
| p95 round trip at 25,000 km | 287 ms | **290 ms** |

What they do is settle the near-ties. Comparing gateways in metres and
comparing them in seconds disagree in a band about 150 km wide per extra hop —
`relay_delay × c` — where a nearer gateway one place further along loses more
to processing than it saves in light. Inside that band the metres answer is
simply wrong, and a test pins the case.

They also put a floor under a path that no geometry can dig through, which
matters when the thinking-time budget is being argued over a few milliseconds
(ADR-0020).

## Consequences

- `exit_gateway` takes a `relay_delay` argument. Passing zero recovers the
  pure-geometry comparison, which is what most of its unit tests want.
- `Gateway` carries the one-way `latency` it was chosen on, alongside `path`.
- `select_anchor` still argues in **metres**, because the re-anchor margin is a
  stated distance and relays are a thirtieth of a hop. Relays decide which door
  a session leaves by; the margin decides which anchor it holds. If the margin
  is ever restated in milliseconds, this is the seam to revisit.
- Any port of the routing — including the site's `terminus-orbits.js` — has to
  carry the relay delay too, or it will draw a route the crate would not have
  chosen.
- Sensitivity is one multiplication: at 1.0 ms per relay the adopted margin's
  p95 round trip becomes 179 ms and the 25,000 km row 293 ms. The decision in
  ADR-0020 does not turn on it.
- **Two older examples still quote pure propagation and have not been
  converted.** `compute_placement` reports a worst-geometry round trip of
  180 ms leaving 120 ms to think, and `unbroken_thread` prices an ARQ
  retransmission at a hardcoded 180 ms. Both understate by about 1 ms — one
  relay each way — and neither conclusion turns on it: 1.8x the stall budget
  stays 1.8x. They are listed here so the inconsistency is recorded rather than
  discovered. Converting them is a separate change, because `unbroken_thread`
  arguably wants the worst round trip the *policy* produces (193 ms at the
  adopted margin) rather than a 60-degree geometry bound, and that is a
  modelling choice rather than a correction.
