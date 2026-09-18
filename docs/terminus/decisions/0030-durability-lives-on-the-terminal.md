# ADR-0030: Durability lives on the terminal; nothing is kept above the anchors

Status: accepted
Date: 2026-09-18
Requirements: TER-REQ-002, TER-REQ-006, TER-REQ-007, TER-REQ-010, TER-REQ-014, TER-REQ-016
Evidence: `cargo run -p terminus-orbits --example recovery_timeline` (sections B, B2, B3); `crates/orbits/src/placement.rs` unit tests

Supersedes the **durable layer** of ADR-0004 — the L1/L2 archive and the L4/L5
reservation. The rest of ADR-0004 stands: access relays own no state, MEO
anchors own the session, and an access handover remains a routing event.

## Decision

**The durable copy of a conversation is the terminal's own, and nothing above
the anchors keeps anything.**

- **Access layer (2,200 km):** unchanged.
- **Compute layer (MEO, 20,000 km):** unchanged, plus what the durable layer
  used to hold that is not user data. The model repository is the shell: all
  24 anchors carry the frozen weights already, and 24 copies of a static
  artifact *is* the replication. Checkpoints ride the same shell.
- **Durable layer:** deleted. No spacecraft at L1/L2, none at L4/L5, and no
  `MEO↔L1/L2` optical link class.
- **The terminal** appends each completed exchange to non-volatile storage. It
  already holds every token — it sent each question and received each answer —
  so durability costs storage, not traffic.
- **The anchor seals each exchange** with an authentication tag, so a successor
  verifies a replay rather than trusting ground-supplied text.
- **On anchor death** the timetable's pre-assigned backup (ADR-0009) takes
  over, the terminal replays the sealed transcript up the ordinary access path,
  the successor verifies and prefills.
- **Loss bound unchanged:** at most the in-flight exchange, TER-REQ-014.

## Why

The copy already exists. Every byte the vault was to hold was in the terminal
first, by construction, because the terminal is one end of the conversation.
The vault's job was to keep a second copy of something the user already had.

Recovery gets faster, not slower, and the reason is that the vault's advantage
was always a fat link at the far end while its disadvantage was half a light
second of distance. One uplink rate trades one against the other:

| | |
|---|---|
| stall, terminal replay at the stated 10 Mbps | **3.86 s** |
| stall, the superseded fetch from L1/L2 | 4.55 s |
| crossover uplink rate | **1.32 Mbps** — above it the terminal wins |
| the conversation itself | 3.1 kbps, so the crossover is **428×** a rate the terminal must already sustain |
| prefill, inside every row above | 3.28 s |

The last row is the one that settles it. Recovery is bounded by *re-reading*
32,768 tokens, not by fetching 131 kB of text, so where the text was kept could
never have been worth a spacecraft. `TERMINAL_UPLINK_BPS` is a stated guess —
nothing in this simulator prices a terminal's modem — and the crossover is what
makes the guess safe: any terminal able to hold a conversation clears 1.32 Mbps
by a factor of hundreds.

The fleet-wide burst is small for the same reason. At the million-terminal
ceiling with 10% concurrent, one dead anchor strands 4,167 sessions at once;
spread over 72 access satellites that is 58 sessions and **7.6 MB** of
transcript on one satellite's radio — five orders of magnitude under the 1.2 TB
a dark feeder telescope moves, because a transcript is text and that burst is
working memory.

And the layer being deleted was never designed. ADR-0004 asserted it; nothing
since sized it. There is no spacecraft count, no mass, power or thermal budget,
and no station-keeping propellant — though both L1 and L2 are *unstable*
equilibria (ADR-0002), so a vault there needs periodic nudges and therefore has
a finite life. Neither end of its link exists either: the telescope inventory
(ADR-0018, ADR-0019) gives an anchor six feeder telescopes, two plane links and
one cold spare, and none of them points at L1/L2. Deleting an undesigned layer
costs less than designing it, and TER-REQ-016 judges total system mass.

## The TER-REQ-002 question

TER-REQ-002 forbids assuming a planetary datacenter or ground relay network.
Terminal-held transcripts do not breach it, and the proposal should say so
rather than hope the evaluator misses it:

- inference remains hosted entirely on the provider's space infrastructure;
- the terminal is provider-supplied kit, delivered by parachute under
  TER-REQ-006 and already the user's base station under TER-REQ-010 — nothing
  is *assumed* that we do not ship;
- it stores only its own users' words, serves no other terminal, and routes
  nothing: a store of one's own conversation is not a datacenter, and one
  endpoint keeping its own record is not a relay network.

## Consequences

- **The terminal gains a durability role.** It needs non-volatile storage, an
  append that survives loss of power mid-exchange, and a wear budget that lasts
  the ten unattended years of TER-REQ-007. Declared here, not sized.
- **A session is tied to its terminal.** The vault would have made a
  conversation's history reachable from anywhere; the edge copy does not. Given
  TER-REQ-010, where the terminal *is* the user's base station, this is
  acceptable — but it is a real loss and belongs in the prose.
- **A dead terminal loses its transcripts**, and that failure is
  self-cancelling: the user it serves has no service either. The bound in
  TER-REQ-014 is about a satellite failure and is unaffected.
- **The model has no independent copy, and conversations no longer imply one.**
  Twenty-four anchors holding identical weights is replication against *losing*
  a copy, not against *corrupting* one: a bad model pushed to the whole shell
  leaves nothing outside the blast radius to roll back to, because the
  checkpoints now ride the shell that took the update. The vault at least sat
  outside it. We do not think that argues for keeping a spacecraft — the archive
  of record is the patron's supply line, the weights came from off-world and can
  come again — but the proposal must say so rather than leave the gap for a
  reader to find. A staged rollout (update a subset of anchors, verify, then the
  rest) buys the same protection in process rather than hardware, and is the
  cheaper answer if leaning on the patron is unacceptable.
- **The replay burst at the terminal end is unpriced.** An access satellite's
  radio capacity is modelled nowhere in this simulator, so the time to drain
  7.6 MB of transcript off one satellite's beams is an open item.
- **Authentication tags need a key.** All 24 anchors must verify what any one of
  them sealed. Key distribution across a fleet that already shares a timetable
  is not hard, but it is unspecified here.
- **ADR-0021 and ADR-0022 read "the vault"** where they now mean the terminal's
  transcript. Those records stand as written; the mechanism they describe —
  replay answers a *failed* anchor, streaming answers a *planned* one — is
  unchanged and is what ADR-0022 turns on.
- **The compliance matrix** rows for TER-REQ-002 and TER-REQ-014 change their
  evidence from the vault to this decision.
- **The manuscript** keeps a promise it can no longer make: `shelves-of-the-sky`
  tells the reader to remember L1 and L2 because the proposal will want them.
  It will not. The survey finding is unchanged — the balance points are real and
  free — and the sections that follow now explain why they stay empty.
