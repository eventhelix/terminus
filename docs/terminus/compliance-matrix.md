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
| TER-REQ-003 | ≤300 ms first token p95; ≤100 ms stall p99 | Designed | ADR-0004/0005 latency budget; ADR-0010 FEC-first reliability keeps stalls off the retransmission round trip (5 s → 18 min stall interval at 12.5% overhead) | Statistical p95/p99 verification with traffic model and burst-loss channels (Series 2) |
| TER-REQ-004 | 99.9% availability per settlement | Open | Geometric coverage continuous (ADR-0003) | Availability analysis vs weather, flares, failures (Series 2 + economics post) |
| TER-REQ-005 | 10,000 → 1,000,000 terminals | Open | Spot beams give spatial reuse (ADR-0006) | Quantitative capacity and beam-reuse study |
| TER-REQ-006 | Parachuted, self-contained terminals | Designed | ADR-0006/0007: all aging knowledge lives in orbit; terminal never told anything stale; ADR-0013 fixes the aperture as a 0.5 m planar array, no assembly and no aiming | — |
| TER-REQ-007 | ≥10 Earth years, no field service | Open | Design philosophy supports it (nothing stale); ADR-0013 + `terminal_aperture` (tag post-16) make "no moving parts" a decision, not an assumption — electronic steering, −4.5 dB worst-case scan loss | Terminal power, thermal, and environment engineering (aperture closed) |
| TER-REQ-008 | Cold start ≤15 min; reacquisition ≤30 s | Designed | ADR-0007; `first_contact` (tag post-9): 3.3 min worst case, 4.5× margin; warm start in seconds | — |
| TER-REQ-009 | No blind timing/Doppler search | Designed | ADR-0006; `spot_beams` (tag post-8): residual budgets ±1.2 kHz, ±60 µs | — |
| TER-REQ-010 | WiFi end-user devices only | Designed | Terminal is the WiFi base station (RFP post; ADR-0006 keeps satellite side complex, terminal simple) | — |
| TER-REQ-011 | Avoid/tolerate stellar 1–3 GHz | Designed | ADR-0005; `frequency_plan` (tag post-7): band conceded, Ka/X clear, +25.5 dB aperture win | — |
| TER-REQ-012 | Flare: degrade, never drop; alert ≤10 s | Partial | Band-fallback plan (ADR-0005); timetable-breathing FEC overhead architecture for flare/handover loss (ADR-0010); PHY residual-erasure contract 1%/5% (ADR-0011) | Codec machinery and burst-loss experiments (Series 2); PNT integrity alert (Series 3) |
| TER-REQ-013 | Handover = routing event, ≤100 ms | Partial | Session-anchor principle (ADR-0004); routing path (ADR-0008); transport connection migration + ARQ node-failure recovery chain (ADR-0010); ADR-0013 gives the terminal microsecond electronic repointing, so the aperture cannot be the bottleneck | Make-before-break machinery and interruption measurement (Series 2) |
| TER-REQ-014 | Single failure ≤60 s; bounded compute loss | Partial | Vault replication bounds compute-node loss (ADR-0004); keep-alive liveness + timetable alternates switch in ~300 ms for anchors and links (ADR-0009) | Access redundancy above min-visible-1 — the coverage consequence of a lost LEO satellite (sizing in economics post) |
| TER-REQ-015 | PNT: 10 m, 100 ns, ≥4 satellites | Partial | ADR-0008: PNT collocated on the MEO shell (clocks beside the minds); two-way time-transfer fabric designed; nav band inherits ADR-0005 exclusion | Waveform, geometry/GDOP, and integrity design; hybrid MEO+LEO Doppler-assisted ranging evaluation (ADR-0012) — Series 3 |
| TER-REQ-016 | Evaluate on total system mass/power/robustness | Partial | Trades argued in its currency throughout (altitude vs hardware, ADR-0003; anchors vs 72 minds, ADR-0004) | Full mass/power/propellant/replacement optimization (economics post) |

Tally: 8 Designed, 5 Partial, 3 Open.

Open and Partial items are the committed follow-on volumes: Series 2
(transport and reliability: handover machinery, end-to-end FEC, flare
response, availability), Series 3 (PNT service design on the ADR-0008
timing fabric), and the constellation-economics post (anchor count,
redundancy sizing, total-mass optimization). Statuses as of tag
`terminus-post-11` (backbone, ADR-0008, folded in), with ADR-0012
(MEO-direct declined) and ADR-0013 (terminal aperture) folded in at tags
`terminus-post-15` and `terminus-post-16`.
