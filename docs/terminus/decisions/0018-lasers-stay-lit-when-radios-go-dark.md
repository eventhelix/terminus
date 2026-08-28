# ADR-0018: Lasers stay lit when radios go dark

Status: accepted
Date: 2026-08-27
Requirements: TER-REQ-001, TER-REQ-013, TER-REQ-016
Evidence: `cargo run --release -p terminus-orbits --example feeder_terminals`

## Decision

When the activation plan (ADR-0017) calls a satellite **dark**, it means the
satellite's *radio* is off — it is serving no towns, and the power that would
have gone into a phased array pointed at the ground is saved. **Its laser
terminals stay powered.** A ring is a standing relay whether or not its members
are talking to anybody, so a session may borrow a feeder telescope from any
ring mate, hops chaining up to the three that cross a ring of twelve.

The fleet is built to one drawing: every access satellite carries **2 feeder
telescopes and 2 necklace terminals**, and every anchor **6 feeder telescopes**,
one per ring.

## Why

Each end of a laser link needs its own terminal — aperture, steering, laser,
detector, and the acquisition and tracking that keeps two moving telescopes
locked. Terminal count therefore drives mass and power long before capacity
does, and "which links exist" is a hardware question before it is a routing one.

The naive topology is the expensive one. Sessions hold their anchors for hours
(ADR-0020), so the conversations riding any one access satellite were anchored
at different moments from different places and are scattered across the shell.
Making every one of those pairings into hardware costs, over a day of 1,000
sessions:

| | per access satellite | per anchor |
|---|---:|---:|
| direct, no ring links | median 3, p90 5, **max 9** | max 18 |

One drawing means every satellite carries the worst case, so that is nine
telescopes on all seventy-two.

Pooling along the necklace fixes it, but only if the neighbours are awake. The
activation plan lights the duty ring as a block and scatters singles through
the other five rings, so a lit satellite outside the duty ring usually has no
lit ring mate at all:

| relaying policy | feeder telescopes on every access satellite |
|---|---:|
| only lit satellites relay | 7 |
| **lasers always powered** | **2** |

Of 33 satellites lit at any moment, 12 are the duty ring and 21 are isolated
singles; with the lasers on, that stops mattering, because a single still has
eleven ring mates relaying for it.

The saving is 288 telescopes on the wheel against 648. The cost is the bus
power to keep laser terminals alive on a satellite whose radio is off, which is
the smaller of the two loads: the phased array serving the ground dominates.

## Consequences

- "Dark" in ADR-0017 means radio-dark. The activation plan's power saving is a
  radio saving, and the proposal must not claim otherwise.
- The necklace is load-bearing for the **telescope count**, not for service: a
  broken necklace costs a session a migration it would otherwise have been
  spared, never its connection.
- Sizing the wheel from the busiest window rather than the busiest satellite is
  only valid while this holds. If laser power is ever cut with the radio, the
  wheel needs 9 telescopes per satellite, not 4.
