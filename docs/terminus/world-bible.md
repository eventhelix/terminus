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
| Surface gravity | 9.81 m/s² | Follows from radius and mass |
| Atmosphere | Earth-like pressure and composition | Working assumption; drives link and drag models |

Because the planet is synchronously locked, the terminator is fixed on the
surface but rotates in inertial space at 360°/11.2 days ≈ 32.14°/day
(6.4928e-6 rad/s).

Derived orbital reference points (from the mass and rotation period):

| Quantity | Value |
|---|---:|
| Synchronous orbit radius | ≈ 211,300 km (altitude ≈ 205,000 km) |
| L1/L2 distance | ≈ 150,000 km from the planet |
| L4/L5 distance | ≈ 7.25e6 km (orbital radius scale) |

The synchronous orbit lies near or beyond the prograde stability limit of the
planet's small Hill sphere; a stationary satellite must not be assumed
available.

## The star

A Proxima Centauri-like M dwarf.

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

## Time units

The planet's day and year are the same 11.2-Earth-day period. To avoid
ambiguity, all engineering durations in canon, requirements, and posts are
stated in Earth units (seconds, days, years); in-universe this is rationalized
as the AI's native time standard.

## Reference orbit labels (illustrations and posts)

- LEO access: ~1,800 km
- MEO service / compute / PNT: ~20,000 km
- Regime comparisons add: VLEO ~300 km, stationary ~205,000 km ("likely
  unstable")
