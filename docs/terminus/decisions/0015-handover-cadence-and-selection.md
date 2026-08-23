# ADR-0015: Handover cadence is the in-plane spacing; serving selection is highest-in-view

Status: accepted
Date: 2026-08-23
Requirements: TER-REQ-013, TER-REQ-003, TER-REQ-004
Evidence: `cargo run -p helixsim-orbits --example handover_cadence`

## Decision

A town's link moves every **11.0 minutes** at the baseline, and the
serving satellite is chosen as **the highest one in view**, with a small
elevation hysteresis retained only as a guard against noisy elevation
estimates. The 16.6-minute figure quoted as "dwell" is the best-case
zenith pass and is not the handover cadence; sizing anything against it
would understate the handover rate by half.

## Why

Two intuitions failed when measured, and both matter to the routing design.

**The interval is the in-plane spacing, not the pass.** Satellites in a
plane file past a town in a queue. The town is handed to the next satellite
in the same plane long before the one it has is finished, so the interval
is `period / sats_per_plane` = 131.6 min / 12 = **11.0 min**, against a
16.6 min zenith pass. Measured at four band towns over six hours: 32–33
handovers each, mean interval 10.8–11.0 min. The consequence for sizing is
direct: **adding satellites to a plane buys coverage and costs handovers in
exact proportion, while adding planes buys coverage and costs none.**

**Highest-in-view does not thrash.** The natural worry — with 72
satellites overhead, "always take the highest" would trade between
near-equal candidates — does not happen, because the constellation is a
queue rather than a crowd: the in-plane successor rises as the incumbent
sets, so "highest in view" changes at exactly the moment "hold until it
sets" would have changed anyway. Greedy and sticky selection produce
identical handover counts at every town tested, and greedy never returned
to a satellite it had just left (0 returns in 24 town-hours).

Hysteresis is therefore a guard, not a cure, and it is not free: at
1,200 km, where footprints barely overlap, holding a sinking satellite past
the floor means taking whatever remains when it finally goes — often
another satellite on its way down. Handovers rise from 92 to 154 over the
same six hours. The margin is a per-shell knob; at the 2,200 km baseline it
changes nothing.

## Consequences

- The handover budget of TER-REQ-013 is charged against **~5.5 handovers
  per hour per terminal**, not ~3.6. At the 100 ms interruption ceiling
  that is 0.015% of session time — comfortable, and now stated rather than
  assumed.
- Make-before-break machinery (Series 2) is designed to a known cadence,
  and the timetable that breathes FEC overhead ahead of scheduled handovers
  (ADR-0010) has an 11-minute rhythm to breathe against.
- Shell sizing gains a lever: coverage margin bought by adding planes is
  cheaper in handover terms than the same margin bought by adding
  satellites per plane. This is an input to the economics post.
- `helixsim-orbits` grows a generic `handover` module carrying the policy,
  the timeline, and the cadence statistic, with the two findings pinned as
  tests so a later change cannot quietly restore the wrong story.
