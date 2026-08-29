# ADR-0024: A cold spare per anchor, and the hold-off that makes it worth buying

Status: accepted
Date: 2026-08-28
Requirements: TER-REQ-003, TER-REQ-004, TER-REQ-013, TER-REQ-014, TER-REQ-016
Evidence: `cargo run --release -p terminus-orbits --example feeder_terminals` (section G); `routing::ISL_REACQUIRE`

## Decision

Every MEO anchor carries a **seventh feeder telescope: steerable, cold, and
unassigned.** When one of its six goes dark it repoints at whichever ring lost
it. That is nine telescopes per anchor — 6 feeder, 2 plane (ADR-0019), 1 spare
— **216 across the shell and 504 across the fleet**, against 480 for the plane
links alone.

And in the same decision, the half that is not hardware: **the anchor policy
holds off.** A session whose anchor is known to be reconfiguring a feeder
telescope must not be re-anchored during the acquisition window, stated as
`routing::ISL_REACQUIRE` = **5 s**. The telescope is bought on the condition
that the policy waits for it. Without the hold-off the seventh telescope buys
nothing at all.

## Why

**The spare is the only remedy that gives the nominal budget back.** It
restores the direct path, so it needs no routing of its own: nothing reroutes,
no neighbour is burdened, and once it locks the (ring, anchor) pair is simply
alive again. That matters because ADR-0019's detour cannot do it at any hour
over any pair — the cheapest detour this geometry admits has a floor of
**394 ms of round trip** against a 300 ms nominal budget, which is arithmetic
and not sampling. The plane link buys reachability at the 600 ms degraded
budget. Only the spare buys back 300 ms.

**But it has to win a race, and on the numbers it loses.** The detour adds
**31,501 km** of one-way path at the median against ADR-0020's **5,000 km**
re-anchor margin — **6.3x the margin**. A detoured session is beaten by any
rival anchor several times over, so the policy moves it at its next evaluation.
The spare takes 5 s to slew and lock. If the policy evaluates inside that
window, the sessions the telescope was bought to save have already left, and it
finishes acquiring a ring whose bucket is empty. A seventh telescope on every
anchor in the fleet then protects nothing.

**So the hold-off is a requirement on the policy, not a specification for the
hardware.** This is the part of the decision a reader would not predict from
the parts list, and it is the part that has to be built: while an anchor
reports a feeder telescope reconfiguring, the sessions on that pair are exempt
from re-anchoring for the acquisition window. `select_anchor` stays pure and
orbit-blind (ADR-0020) — the hold-off is a gate in front of it, deciding
whether a session is evaluated at all, not a new term inside its comparison.

**The two remedies are therefore complementary rather than alternatives**, and
that is a reversal of how ADR-0019 read them. The plane link covers the
acquisition window; the spare ends it. Take the spare alone and the bucket is
stranded for the 5 s it is being asked to wait. Take the plane link alone and
the bucket detours at the degraded budget until the policy moves every session
off it, and the pair never comes back to 300 ms. Fleet cost of taking both:

| | telescopes/anchor | fleet |
|---|---:|---:|
| plane links alone (ADR-0019) | 8 | 480 |
| **plane links and the spare** | **9** | **504** |

Twenty-four telescopes across the fleet is what the nominal budget costs after
a failure.

**`ISL_REACQUIRE` is a stated parameter, not a measured one.** It is stated in
one place, the way `RELAY_DELAY` is (ADR-0023), so that everything quotes the
same number rather than inventing its own. Nothing here has modelled a cold
telescope slewing to a new ring, searching, and locking.

What moves if the guess is wrong is the hold-off, not the telescope. The
hardware case does not turn on the value: a spare that restores the direct path
is worth having whether it takes one second or thirty. What the value sets is
how long a session can be asked to sit on a detour that runs to 460 ms of round
trip rather than move. At 5 s that sits comfortably inside TER-REQ-014's 60 s
bound on interruption from a single satellite failure — and with the plane link
under it the session is degraded, not interrupted, for all of that time. A
reacquisition that ran to minutes could not be held off: the sessions would
have to migrate, the hold-off would have to be broken, and the spare would be
back to protecting an empty bucket. **The crossover between
holding and moving is unmeasured, and it is the open item this decision
creates.**

## Consequences

- An anchor bus carries **nine laser terminals**. ADR-0019's count of eight is
  superseded on the count alone; its decision about which *classes* of link
  exist is untouched.
- **The anchor policy gains an obligation it did not have.** Re-anchoring is
  now suppressible per pair, which means the policy needs to be told that an
  anchor is reconfiguring — a piece of state that has to reach it from the
  spacecraft, and the first thing in the policy that is not derived from
  geometry.
- The spare adds **no routing logic and no new link class**. It is a repoint of
  a terminal the architecture already builds six of; nothing in `feeder_route`
  or `select_anchor` has to know it exists.
- **It protects one failure.** A second dark ring on the same anchor falls back
  to the plane link and the degraded budget, which is exactly the world
  ADR-0019 describes.
- The hold-off **bounds** ADR-0019's capacity debt — the relaying plane mate
  carries two rings' traffic for the acquisition window rather than until the
  policy drains the pair — but it **raises the peak**, because a held-off
  bucket does not drain. All 113 sessions on the busiest pair ride the
  neighbour's single telescope together for those seconds. That is still a
  capacity question this model does not price.
- Nothing here helps an anchor that dies outright. The spare repoints a
  telescope on a live machine; the vault (ADR-0004) still answers a dead one.
