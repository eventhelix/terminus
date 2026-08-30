# Terminus Compliance Matrix

Canonical requirement-by-requirement accounting of the proposal against the
RFP baseline (`requirements.md`). Updated as later series close debts;
statuses as of the end of Series 1 (tag `terminus-post-10`).

Status meanings — **Designed**: decision made and evidenced by a
reproducible run; **Partial**: principle decided, machinery or sizing
outstanding; **Open**: committed follow-on work, not yet designed.

| ID | Requirement (short) | Status | Decision / evidence | Outstanding |
|---|---|---|---|---|
| TER-REQ-001 | Continuous ±20° band coverage | Designed | ADR-0003; `access_constellation` (tag post-5): min visible ≥ 1 over a full rotation | — |
| TER-REQ-002 | LLM service from space infrastructure | Designed | ADR-0004; `compute_placement` (tag post-6) | — |
| TER-REQ-003 | ≤300 ms first token p95 (nominal), ≤600 ms (degraded); ≤100 ms stall p99 | Designed | ADR-0004/0005 latency budget; ADR-0010 FEC-first reliability keeps stalls off the retransmission round trip (5 s → 18 min stall interval at 12.5% overhead); ADR-0023 fixes what a round trip is made of — every satellite that *forwards* is charged `routing::RELAY_DELAY`, stated at 0.5 ms, and `routing::exit_gateway` compares time rather than distance — which is what every latency figure in this row and in TER-REQ-013 and TER-REQ-014 rests on: the p95 round trip at the adopted margin is **178 ms** (177 ms counting light alone), and the 394 ms detour floor below is four terms, the fourth of them a pair of relays; TER-REQ-003 amended to split the first-token budget nominal/degraded — ADR-0019's `feeder_terminals` run finds the cheapest plane-link detour has a 394 ms round-trip floor (can't meet 300 ms by arithmetic, not sampling) and every measured detour clears the 600 ms degraded budget with room to spare (460 ms worst case) | Statistical p95/p99 verification with traffic model and burst-loss channels (Series 2) |
| TER-REQ-004 | 99.9% availability per settlement | Open | Geometric coverage continuous (ADR-0003) | Availability analysis vs weather, flares, failures (Series 2 + economics post) |
| TER-REQ-005 | 10,000 → 1,000,000 terminals | Open | Spot beams give spatial reuse (ADR-0006) | Quantitative capacity and beam-reuse study |
| TER-REQ-006 | Parachuted, self-contained terminals | Designed | ADR-0006/0007: all aging knowledge lives in orbit; terminal never told anything stale; ADR-0013 fixes the aperture as a 0.5 m planar array, no assembly and no aiming | — |
| TER-REQ-007 | ≥10 Earth years, no field service | Open | Design philosophy supports it (nothing stale); ADR-0013 + `terminal_aperture` (tag post-16) make "no moving parts" a decision, not an assumption — electronic steering, −4.5 dB worst-case scan loss | Terminal power, thermal, and environment engineering (aperture closed) |
| TER-REQ-008 | Cold start ≤15 min; reacquisition ≤30 s | Designed | ADR-0007; `first_contact` (tag post-9): 3.3 min worst case, 4.5× margin; warm start in seconds | — |
| TER-REQ-009 | No blind timing/Doppler search | Designed | ADR-0006; `spot_beams` (tags post-8 through post-8d): residual budgets ±6 kHz (every spot alike — beam Doppler spread is the invariant (f/c)·v·β) and ±310 µs (the rim beam, whose elongated spot is farther, flatter, fatter: 5.3×) | — |
| TER-REQ-010 | WiFi end-user devices only | Designed | Terminal is the WiFi base station (RFP post; ADR-0006 keeps satellite side complex, terminal simple) | — |
| TER-REQ-011 | Avoid/tolerate stellar 1–3 GHz | Designed | ADR-0005; `frequency_plan` (tag post-7): band conceded, Ka/X clear, +25.5 dB aperture win | — |
| TER-REQ-012 | Flare: degrade, never drop; alert ≤10 s | Partial | Band-fallback plan (ADR-0005); timetable-breathing FEC overhead architecture for flare/handover loss (ADR-0010); PHY residual-erasure contract 1%/5% (ADR-0011) | Codec machinery and burst-loss experiments (Series 2); PNT integrity alert (Series 3) |
| TER-REQ-013 | Handover = routing event, ≤100 ms | Partial | Session-anchor principle (ADR-0004); routing path (ADR-0008); transport connection migration + ARQ node-failure recovery chain (ADR-0010); ADR-0013 gives the terminal microsecond electronic repointing, so the aperture cannot be the bottleneck; ADR-0015 + `handover_cadence` fix the rate the budget is charged against — 11.0 min between handovers (the in-plane spacing, not the 16.6 min zenith pass), 5.5/h, 0.015% of session time at the 100 ms ceiling; ADR-0020 makes anchor selection a ring-wide policy rather than a visibility constraint — every ring reaches every anchor at every instant, but the adopted 5,000 km re-anchor margin still moves a session to a shorter path **12.70 times a day** (~every 113 minutes), so the anchor handover stays in the steady state; ADR-0022 (superseding ADR-0021) ships make-before-break context transfer — 10.7 GB in 0.86 s on a 100 Gbps link — in the first release because every one of those 12.70 daily migrations happens inside a live conversation | Interruption measurement for make-before-break, now Series 1 machinery per ADR-0022 rather than Series 2 |
| TER-REQ-014 | Single failure ≤60 s; bounded compute loss | Partial | Vault replication bounds compute-node loss (ADR-0004); keep-alive liveness + timetable alternates switch in ~300 ms for anchors and links (ADR-0009); ADR-0019 gives each anchor two frozen plane links so a failed feeder telescope degrades the busiest (ring, anchor) bucket — **113** sessions, not the 134 an earlier run reported before the necklace hop correction — at the 600 ms degraded budget rather than stranding it — the plane mate forwards rather than terminates, so the detour pays an extra `RELAY_DELAY` on top of the link's 124 ms of light (ADR-0023); the session still migrates (the detour costs 6.3x ADR-0020's re-anchor margin), so what the plane link buys is continuity of service through the move, not stillness; ADR-0024 adds a seventh, steerable, cold-spare feeder telescope per anchor plus a `routing::ISL_REACQUIRE` = 5 s hold-off that suppresses re-anchoring during acquisition, restoring the nominal 300 ms budget once the spare locks — neither the spare nor the hold-off works without the other; ADR-0022 (superseding ADR-0021) ships make-before-break context transfer in the first release, so vault replay remains the recovery path only for an anchor that fails outright, not a planned move | Access redundancy above min-visible-1 — the coverage consequence of a lost LEO satellite (sizing in economics post); capacity cost of a plane mate carrying two rings' traffic during the spare's acquisition window (ADR-0019/0024, unpriced) |
| TER-REQ-015 | PNT: 10 m, 100 ns, ≥4 satellites | Partial | ADR-0008: PNT collocated on the MEO shell (clocks beside the minds); two-way time-transfer fabric designed; nav band inherits ADR-0005 exclusion; ADR-0014 + `navigation_shell` size the shell — 24 satellites, 6×4 at 55°, worst case 4 in view / typical 7.2 over the band, so the ≥4 floor is met | Accuracy half only: waveform, geometry/GDOP, and integrity design; hybrid MEO+LEO Doppler-assisted ranging evaluation (ADR-0012) — Series 3 |
| TER-REQ-016 | Evaluate on total system mass/power/robustness | Partial | Trades argued in its currency throughout (altitude vs hardware, ADR-0003; anchors vs 72 minds, ADR-0004) | Full mass/power/propellant/replacement optimization (economics post) |

Tally: 8 Designed, 5 Partial, 3 Open.

Open and Partial items are the committed follow-on volumes: Series 2
(transport and reliability: handover interruption measurement, end-to-end
FEC, flare response, availability), Series 3 (PNT service design on the
ADR-0008 timing fabric), and the constellation-economics post (anchor count,
redundancy sizing, total-mass optimization). Statuses as of tag
`terminus-post-11` (backbone, ADR-0008, folded in), with ADR-0012
(MEO-direct declined) and ADR-0013 (terminal aperture) folded in at tags
`terminus-post-15` and `terminus-post-16`, and ADR-0014 (navigation shell
sizing) and ADR-0015 (handover cadence and selection) folded in from the
constellation-explorer work, and ADR-0018 to ADR-0021 (laser power, shell
plane links, anchor selection, deferred context transfer) folded in from the
backbone topology work, with ADR-0022 (context transfer ships in the first
release, superseding ADR-0021), ADR-0023 (latency counts relays, and routing
compares time), the TER-REQ-003 amendment (nominal/degraded
first-token budget), and ADR-0024 (cold spare and the re-anchor hold-off)
folded in from the 2026-08-28 amendment round, alongside ADR-0020's and
ADR-0019's corrected figures (5,000 km re-anchor margin, 12.70 anchor
changes/day, 113 sessions on the busiest feeder telescope).
