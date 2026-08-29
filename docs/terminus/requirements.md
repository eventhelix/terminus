# Terminus Requirements Baseline

The Alien AI's RFP requirements. Every proposal post traces its design
decisions to these IDs. Values are in-universe procurement targets; where a
trade study later shows a target should move, the change is recorded as an
ADR and the baseline is updated.

## Mission and service

| ID | Requirement |
|---|---|
| TER-REQ-001 | Provide continuous service over the inhabited band: all surface points within ±20° great-circle arc of the terminator. |
| TER-REQ-002 | The service is interactive access to LLM inference hosted on the provider's space infrastructure. No planetary datacenter or ground relay network may be assumed. |
| TER-REQ-003 | First-token latency ≤ 300 ms (p95) under nominal operation; ≤ 600 ms (p95) under degraded operation (a feeder telescope dark, session served over the plane link). Steady-state token stall ≤ 100 ms (p99). |
| TER-REQ-004 | Service availability ≥ 99.9% per settlement per Earth year, measured as the fraction of time TER-REQ-003 is met. |
| TER-REQ-005 | Support 10,000 terminals across ~100 settlements at initial service; scale to 1,000,000 terminals without constellation redesign. |

## Ground segment

| ID | Requirement |
|---|---|
| TER-REQ-006 | Terminals are delivered by parachute, fully self-contained (power, satellite antenna, radio, WiFi base station), and require no assembly. |
| TER-REQ-007 | Terminals operate for ≥ 10 Earth years with no field service and no skilled operators. |
| TER-REQ-008 | Terminal cold start — power-on with no stored almanac, time, or position — to full service in ≤ 15 minutes. Reacquisition after an outage in ≤ 30 seconds. |
| TER-REQ-009 | Terminals shall not perform blind timing or Doppler search. The space segment presents a time- and frequency-precorrected air interface (e.g. per-beam precompensation); residual offsets at the terminal must fall within the waveform's guard budgets. The proposal's beam-size trade study sets the residual budget values. |
| TER-REQ-010 | End users access the service with provided WiFi touch devices only; the terminal is their WiFi base station. No other user equipment may be required. |

## Environment

| ID | Requirement |
|---|---|
| TER-REQ-011 | Primary service links shall avoid, or demonstrably tolerate, the star's coherent radio emission band (~1–3 GHz, strongest near 1.6 GHz). |
| TER-REQ-012 | During major stellar flares the service may degrade in rate but shall not drop established sessions; PNT integrity alerts must reach users within 10 s. |

## Continuity and reliability

| ID | Requirement |
|---|---|
| TER-REQ-013 | Satellite handover is a routing event: no session restart, no transport reconnection. Handover interruption ≤ 100 ms (consistent with TER-REQ-003 stall budget). |
| TER-REQ-014 | No single satellite failure may interrupt service to any settlement for more than 60 s. Loss of a compute node mid-conversation loses at most the in-flight exchange. |
| TER-REQ-015 | Provide PNT throughout the service region: horizontal position ≤ 10 m (95%), time ≤ 100 ns (95%), with ≥ 4 navigation satellites always visible. PNT must remain available during communications-service outages. |

## Programmatics

| ID | Requirement |
|---|---|
| TER-REQ-016 | The provider manufactures and deploys all spacecraft. Proposals are evaluated on total system mass (constellation + launch + propellant + power/thermal hardware), satellite count, latency margin, robustness, and growth path — not on satellite count alone. State replacement cadence and design lifetime. |
