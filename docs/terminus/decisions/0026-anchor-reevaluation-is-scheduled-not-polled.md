# ADR-0026: Anchor re-evaluation is scheduled, not polled

Status: accepted
Date: 2026-08-29
Requirements: TER-REQ-003, TER-REQ-013, TER-REQ-014
Evidence: `cargo run --release -p terminus-orbits --example feeder_terminals` (section H); `cargo run --release -p terminus-orbits --example failure_timeline` (section C)

## Decision

The anchor policy (ADR-0020) runs at two kinds of instant, and **no others**:

1. **Scheduled.** Every orbit is known years ahead, so for a session holding
   anchor A the instant at which any rival first beats A by the re-anchor
   margin is computable in advance. Those crossing instants are **timetable
   entries**, precomputed and distributed like every other handover in this
   architecture (ADR-0009). The **12.70 moves per day** section H measures at
   the adopted 5,000 km margin are not discoveries made by a poller — they
   are that schedule, counted.
2. **Event-triggered.** The instants no timetable can carry are the ones
   failures cause. The policy re-evaluates immediately at: a keep-alive
   declaration (three missed 100 ms beats, +300 ms — ADR-0009), a spare
   telescope locking (`routing::ISL_REACQUIRE` — ADR-0024), and a hold-off
   window expiring (ADR-0024).

**There is no periodic poll and no cadence parameter.** "At its next
evaluation" in every failure story means: at the failure event itself, unless
the hold-off suppresses it.

## Why

**The question "how often does the policy re-evaluate" was wrongly posed for
this system, and leaving it unanswered had become load-bearing.** The failure
timeline's race — a 5 s acquisition against a policy already moving everyone —
depends on when the policy acts, and until now "its next evaluation" had no
defined time. Under this decision the semantics are exact: absent a hold-off,
a detoured session is re-evaluated the instant the failure is declared
(+300 ms), finds itself 6.3× over the margin (ADR-0019), and leaves — which is
why the spare cannot win without the hold-off, and why ADR-0024's two halves
are one purchase. The race is decided at +300 ms, not at some poller's tick.

**Scheduling is the architecture's own answer.** ADR-0009's thesis is that
routing runs on a precomputed timetable with pre-assigned alternates; a
periodic evaluation loop would be the one discovery process in a system that
otherwise has none, with a tunable interval that trades staleness against
polling load at a million terminals. The timetable computes the crossings
instead: the same sweep section H already runs to *count* them is the
computation that *schedules* them.

**Retuning stays possible.** ADR-0020 requires the margin to be an operating
parameter. Changing it in flight recomputes the crossing schedule and
distributes it as a timetable update — the mechanism that already exists —
rather than reconfiguring a poller fleet-wide.

## Consequences

- The scheduled entries carry each session's **next switch instant and its
  target** — the anchor column of the timetable ADR-0009 already requires,
  made explicit. Simulated behavior equals flown behavior, including
  migrations.
- The failure-story presentations gain defined semantics: the policy's
  evaluation mark belongs at the declaration instant. (The 2D failure plate's
  rail currently places its evaluation tick by presentation pacing, recorded
  as such under ADR-0025; if that tick is ever to be measured, this ADR is
  the number's source — declaration at +300 ms.)
- Series 2 protocol work inherits scheduled anchor-switch entries alongside
  the keep-alive interval and alternate-switching model it already inherits
  from ADR-0009.
- The hold-off (ADR-0024) is now precisely a **suppression of event-triggered
  evaluation during the acquisition window**; scheduled entries falling
  inside the window are deferred to its end, which is the "sat on the detour
  for nothing" outcome when no spare is fitted.
- Failure multiplicity remains open: this ADR schedules the healthy sky and
  reacts to single failures; what happens when failures compose is the
  routing-protocol question, deliberately not yet decided.
