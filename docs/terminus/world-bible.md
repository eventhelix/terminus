# Terminus World Bible

Canonical parameters for the Terminus series. Every post and simulation cites
these values; changes require an ADR in `decisions/`.

## The planet

An Earth-sized planet tidally locked to a Proxima-like red dwarf. Modeled on
Proxima Centauri b; where Proxima b's properties are uncertain (it does not
transit, so its radius is inferred), the reference model uses Earth values.

| Parameter | Canonical value | Notes |
|---|---:|---|
| Radius | 6,371 km | Earth mean radius |
| Mass | 1.0 Earth mass | μ = GM = 3.986004418e14 m³/s² |
| Rotation period | 11.2 Earth days (967,680 s) | Synchronous: rotation = orbital period |
| Orbital distance | 0.0485 AU ≈ 7.256e6 km | |
| Surface gravity | 9.82 m/s² | Follows from radius and mass |
| Atmosphere | Earth-like pressure and composition | Working assumption; drives link and drag models |
| J2 (oblateness) | 2.15e-5 | ~50x rounder than Earth; see below (`planet_figure`) |
| C22 (tidal bulge) | 6.44e-6 | J2/C22 = 10/3, the hydrostatic synchronous figure |
| Polar flattening | 1/15,066 | Earth is 1/298; the synchronous figure is triaxial, longest toward the star (equator 1/12,556 out of round) |

### Figure of the planet

A body's bulge scales with `q = omega^2 R^3 / mu` and with how centrally
condensed it is (the fluid Love number `k2`). Calibrating `k2 = 0.9414` by
inverting Earth's own `J2 = k2 q / 3` gives the reference planet's figure
directly. Two effects pull opposite ways:

- The planet spins **11.2x slower** than Earth, and `q` goes as `omega^2`, so
  `q` is **126x smaller**.
- But it is **locked**, so it carries a permanent tidal bulge facing the star.
  Synchronous hydrostatic equilibrium gives `J2 = 5 k2 q / 6` and
  `C22 = k2 q / 4` — **2.5x** the spin-only figure.

Net: `J2 = 2.15e-5`, about **50x smaller than Earth's**, not 126x.

The shape rides the same potential through `h2 = 1 + k2`: semi-axes
`a = R(1 + 3h2q/2)` toward the star, `b = R` across, `c = R(1 - h2q/2)` at the
pole, so the polar flattening is `5h2q/4 = 1/15,066` and the equator is
`3h2q/2 = 1/12,556` out of round — 66 µm on a one-meter globe. (The
free-rotator relation `f = 3J2/2 + q/2` does not apply to a locked body; it
gave 1/21,800 until 2026-09-04.)

This is load-bearing for the whole access architecture. Nodal regression goes
as `cos(inclination)`, so a perfectly polar ring does not drift at all; what
moves a ring is injection inclination error. At 0.1 deg off polar, a 2,200 km
ring's node drifts **0.45 deg per decade** against 30 deg of ring spacing. On an
Earth-J2 planet the same error would cost 22.5 deg per decade and the fixed-ring
architecture of ADR-0001 would not survive. Evidence: `planet_figure`.

Because the planet is synchronously locked, the terminator is fixed on the
surface but rotates in inertial space at 360°/11.2 days ≈ 32.14°/day
(6.4930e-6 rad/s).

### Climate

Radiative screening only; evidence: `climate_screen` (ADR-0029).

| Quantity | Canonical value | Notes |
|---|---:|---|
| Instellation | 897 W/m² | 0.66× Earth; the module's calibration |
| Substellar equilibrium, no transport | 324.4 K (+51.2 °C) | Bracket, upper |
| Equilibrium shared over the sphere | 229.4 K (−43.8 °C) | Bracket, lower |
| **Twilight band surface** | **281.15 K (+8 °C)** | Inside the bracket |
| Greenhouse increment the band implies | 51.8 K | Earth's is 33 K — the band's air works harder |
| Scale height at band temperature | 8.22 km | Earth 8.43 km; sets the cell's return-branch altitude |
| Night side with no transport | 35.2 K (−238 °C) | Radiating against 0.087 W/m² of interior heat |
| CO₂ frost point (400 ppm of a bar) | 131.2 K (−142 °C) | |
| N₂ condensation point (0.78 bar) | 75.3 K (−197.8 °C) | The temperature at which the sky stops being sky |
| Surface wind | 5–15 m/s, dayward, never reversing | **Not computed.** GCM literature for tidally locked M-dwarf planets; the return branch runs several times faster |

The cold trap is the reason the band exists. Stalled, the night side sits
below both condensation points and the atmosphere freezes onto it; running,
the dark hemisphere sits near 229 K, well clear of either. Wind speed is the
one climate quantity the toolkit refuses to produce — an overturning velocity
is a GCM result, and a closed-form number here would be false precision.

Derived orbital reference points (from the mass, rotation period, star mass,
and orbital distance; evidence: `regime_survey` example):

| Quantity | Value |
|---|---:|
| Synchronous orbit radius | ≈ 211,500 km (altitude ≈ 205,000 km) |
| Hill radius | ≈ 146,000 km |
| Prograde orbit stability limit (≈ ½ Hill) | ≈ 73,000 km |
| L1/L2 distance | ≈ 146,000 km from the planet (≈ Hill radius) |
| L4/L5 distance | ≈ 7.25e6 km (orbital radius scale) |

The synchronous orbit radius is ≈ 2.9× the prograde stability limit of the
planet's small Hill sphere: no stationary satellite exists around this
planet (ADR-0002).

## The star

A Proxima Centauri-like M dwarf of 0.122 solar masses and 0.00155 solar
bolometric luminosities (ADR-0029). At 0.0485 AU that delivers **897 W/m²**,
0.66× Earth's — matching the published 0.65 S_Earth for Proxima b, which is
the calibration test for the climate module.

- Produces coherent radio emission in roughly 1–3 GHz, with strong activity
  near 1.6 GHz. L/S band is therefore hostile spectrum.
- Flares (UV/X-ray) disturb the ionosphere and raise the radio noise floor;
  the system must degrade gracefully during major flares.
- From the terminator, the star sits approximately on the local horizon;
  satellites above a minimum elevation (reference: 25°) are angularly
  separated from it.

## The civilization

- Pre-industrial ("budding") civilization concentrated in a habitable band
  within ±20° great-circle arc of the terminator.
- Initial service population: about 100 settlements, 10,000 ground terminals.
- No planetary industry, datacenters, launch capability, or field service.
  All space and ground hardware is manufactured and deployed by the Alien AI:
  satellites placed in orbit, terminals delivered by parachute.
- End users interact with the LLM through WiFi touch devices provided with
  the terminals; a terminal is a WiFi base station backhauled by satellite.

## The gift

The LLM the AI provides, as canon reference parameters for inference-state
arithmetic (KV-cache sizing, migration bandwidth, compute placement):

| Parameter | Canonical value |
|---|---:|
| Model class | 70B-scale transformer |
| Layers | 80 |
| KV heads (grouped-query attention) | 8 |
| Head dimension | 128 |
| KV precision | 2 bytes (fp16) |
| KV cache per token | 320 KiB (327,680 B) |
| Reference long context | 32,768 tokens ⇒ ≈ 10.7 GB KV cache |

Evidence: `compute_placement` example (placement.rs `KvCacheModel`).

## Time units

The planet's day and year are the same 11.2-Earth-day period. To avoid
ambiguity, all engineering durations in canon, requirements, and posts are
stated in Earth units (seconds, days, years); in-universe this is rationalized
as the AI's native time standard.

## Reference orbit labels (illustrations and posts)

- LEO access: 2,200 km (6 polar rings × 12 satellites; ADR-0003 — the
  earlier ~1,800 km working label is superseded)
- MEO service / compute / PNT: ~20,000 km (24 satellites: 6 planes × 4 at
  55° inclination, nodes spread over 360°; ADR-0014 — the shell is sized by
  navigation, not by the anchors)
- Regime comparisons add: VLEO ~300 km, stationary ~205,000 km ("likely
  unstable")

## Access-link rhythm (posts and illustrations)

- A town's serving satellite changes every **11.0 min** — the in-plane
  spacing (131.6 min period / 12 satellites), not the 16.6 min best-case
  zenith pass (ADR-0015). Quote 16.6 min only as "the longest a single
  satellite could serve", never as the handover interval.
- Duty ring changes every **22.4 h** — 30° of terminator rotation at
  32.14°/day (`access_constellation`). From an epoch with a ring on the
  terminator, the first change comes at **11.2 h**: half a plane spacing of
  drift, after which the incumbent and its neighbour are equidistant.
- Not every satellite is radiating. The fleet runs a precomputed activation
  plan (ADR-0017): duty ring on as a block, other rings patched in only where
  the band would be unserved, then a prune pass that switches off whatever
  the later picks made redundant — chiefly the pole overlap where the rings
  converge. Mean **23 of 72 lit** (32%), peak 30; the proved
  minimum is 21.4. Roughly **70% of the fleet is dark at any moment**, and all
  72 spacecraft are still required.
- The duty ring is the ring carrying the **most** traffic, never the only ring
  serving (ADR-0016). Below 15° latitude it supplies about **half** the
  visible satellites and a town sees **1-2 rings**; above 70° all six rings
  are in reach and the duty ring's share falls to **17%**. Coverage never
  depends on inter-ring phasing.
