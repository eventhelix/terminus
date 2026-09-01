# ADR-0028: The rendezvous contract — the beacon PHY is frozen in terminal firmware

Status: accepted
Date: 2026-09-01
Requirements: TER-REQ-006, TER-REQ-007, TER-REQ-008
Evidence: `cargo run -p terminus-orbits --example first_contact` (tag: terminus-post-9d)

## Decision

The terminal ships from the factory with exactly one set of radio facts
hard-coded, the **rendezvous contract** — the minimum a cold receiver
must know to recognize the lantern and answer it:

| Field | Reference value | Note |
|---|---|---|
| Beacon carrier | 8.400 GHz (X diversity band, ADR-0005) | precompensated per spot (ADR-0006); the box never tunes |
| Beacon channel width | 50 kHz | holds the ±6 kHz residual plus the modulated beacon |
| Beacon dwell grammar | 10 ms per spot position | one complete message per dwell: satellite id, spot id, reply grant |
| Beacon waveform | fixed correlation sequence + message format | detection by correlation at bare-element gain (ADR-0027) |
| Reply carrier | beacon + 50.000 MHz fixed offset | one frozen number covers both directions; the box transmits relative to what it heard |
| Reply timing | within the grant, ±1.1 ms window | slack absorbed in orbit (ADR-0007) |
| Reference beacon power | 10 W behind the satellite's 0.7 m X aperture | the budget's stated anchor |

Everything else the box will ever use — almanacs, schedules, spot maps,
service waveforms, even better beacon waveforms — is *told to it over
this contract* after first contact, and may evolve for a decade without
touching a fielded terminal.

## Why

- **ADR-0007 (and the manifesto of TER-REQ-006) says the terminal is
  never told anything that could go stale.** The contract is the sharp
  edge of that rule: carrier frequencies and channel widths are spectrum
  and physics — they go stale only if the *network* abandons them, which
  the network controls; orbits and schedules go stale by themselves.
  Freezing the former is what makes never storing the latter possible.
- **A fixed reply offset removes a second dial.** The box answers at
  "what I heard + 50 MHz," so precompensation on the downlink
  automatically pre-corrects the uplink to first order; the residual
  lands inside the satellite's wide acquisition window, where slack
  belongs.
- **Correlation detection requires a known sequence.** The wide-listen
  budget of ADR-0027 (18.9 dB SNR in 50 kHz at bare-element gain) is
  only usable because the receiver knows what it is listening for; the
  sequence must therefore be part of the contract, not of the updatable
  almanac.
- **Ten years, no visits (TER-REQ-007).** Any parameter that were
  updatable would need a working link to update — a circular dependency
  for the one channel whose job is to exist before any link does. The
  contract is deliberately the system's only unupgradable interface, so
  it is deliberately minimal.

## Consequences

- The satellite side must carry the contract's beacon forever, whatever
  else the waveform teams improve; a richer acquisition scheme can only
  be *added beside* it, never replace it, while contract-era terminals
  remain fielded.
- Series 2 waveform design receives the contract as a frozen input:
  sequence design, message coding, and the sustained-X service plan all
  build on these values.
- The compliance story for TER-REQ-008 now names its assumptions: the
  43 s budget rests on the contract's 10 W / 50 kHz / 10 ms anchors.
- Reference values above are the proposal's; if the RFP's spectrum
  grants shift them, the *shape* of the contract (one carrier, one
  offset, one sequence, one grammar) is the decision that stands.
