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
| TER-REQ-003 | ≤300 ms first token p95; ≤100 ms stall p99 | Designed | ADR-0004/0005: worst-geometry propagation RTT 180 ms leaves 120 ms inference margin | Statistical p95/p99 verification with traffic model (Series 2) |
| TER-REQ-004 | 99.9% availability per settlement | Open | Geometric coverage continuous (ADR-0003) | Availability analysis vs weather, flares, failures (Series 2 + economics post) |
| TER-REQ-005 | 10,000 → 1,000,000 terminals | Open | Spot beams give spatial reuse (ADR-0006) | Quantitative capacity and beam-reuse study |
| TER-REQ-006 | Parachuted, self-contained terminals | Designed | ADR-0006/0007: all aging knowledge lives in orbit; terminal never told anything stale | — |
| TER-REQ-007 | ≥10 Earth years, no field service | Open | Design philosophy supports it (nothing stale, no moving parts asked of terminal) | Terminal hardware, power, and environment engineering |
| TER-REQ-008 | Cold start ≤15 min; reacquisition ≤30 s | Designed | ADR-0007; `first_contact` (tag post-9): 3.3 min worst case, 4.5× margin; warm start in seconds | — |
| TER-REQ-009 | No blind timing/Doppler search | Designed | ADR-0006; `spot_beams` (tag post-8): residual budgets ±1.2 kHz, ±60 µs | — |
| TER-REQ-010 | WiFi end-user devices only | Designed | Terminal is the WiFi base station (RFP post; ADR-0006 keeps satellite side complex, terminal simple) | — |
| TER-REQ-011 | Avoid/tolerate stellar 1–3 GHz | Designed | ADR-0005; `frequency_plan` (tag post-7): band conceded, Ka/X clear, +25.5 dB aperture win | — |
| TER-REQ-012 | Flare: degrade, never drop; alert ≤10 s | Partial | Band-fallback plan (ADR-0005) | Rate-adaptation and coding machinery (Series 2); PNT integrity alert (Series 3) |
| TER-REQ-013 | Handover = routing event, ≤100 ms | Partial | Session-anchor principle (ADR-0004); concrete routing path via duty ring and feeder links (ADR-0008) | Make-before-break machinery and interruption measurement (Series 2) |
| TER-REQ-014 | Single failure ≤60 s; bounded compute loss | Partial | Vault replication bounds compute-node loss (ADR-0004) | Access redundancy above min-visible-1 (debt declared in ADR-0003) |
| TER-REQ-015 | PNT: 10 m, 100 ns, ≥4 satellites | Partial | ADR-0008: PNT collocated on the MEO shell (clocks beside the minds); two-way time-transfer fabric designed; nav band inherits ADR-0005 exclusion | Waveform, geometry/GDOP, and integrity design (Series 3) |
| TER-REQ-016 | Evaluate on total system mass/power/robustness | Partial | Trades argued in its currency throughout (altitude vs hardware, ADR-0003; anchors vs 72 minds, ADR-0004) | Full mass/power/propellant/replacement optimization (economics post) |

Tally: 8 Designed, 5 Partial, 3 Open.

Open and Partial items are the committed follow-on volumes: Series 2
(transport and reliability: handover machinery, end-to-end FEC, flare
response, availability), Series 3 (PNT service design on the ADR-0008
timing fabric), and the constellation-economics post (anchor count,
redundancy sizing, total-mass optimization). Statuses as of tag
`terminus-post-11` (backbone, ADR-0008, folded in).
