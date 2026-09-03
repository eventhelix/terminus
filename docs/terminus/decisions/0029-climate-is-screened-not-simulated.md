# ADR-0029: Climate is screened, not simulated — and wind stays a citation

Status: accepted
Date: 2026-09-03
Requirements: TER-REQ-006, TER-REQ-007
Evidence: `cargo run --release -p terminus-orbits --example climate_screen`

## Decision

The reference planet's climate enters the toolkit as **radiative screening
only**: instellation, equilibrium temperature at a chosen heat-redistribution
factor, pressure scale height, vapour pressure over ice, and the night-side
cold-trap test. `crates/orbits/src/climate.rs`.

**Surface wind speed is explicitly out of scope and stays a literature
citation.** The series quotes 5–15 m/s at the surface, with a return branch
several times faster, from GCM studies of tidally locked M-dwarf planets. No
closed form in this crate produces that number, and the module says so in its
own documentation and in the example's final section.

Three claims that posts previously asserted now trace to a run:

1. **The twilight band's +8 °C is inside what the star can pay for.** The
   bracket is 324.4 K at the substellar point with no transport and 229.4 K
   shared over the whole sphere; the canonical band sits between them.
2. **The cell's nightward branch rides at 7–10 km because that is one scale
   height.** `H = RT/(Mg)` gives 8.22 km at the band temperature, against
   Earth's 8.43 km. The circulation plate's altitude was previously a drawing
   decision.
3. **A stalled circulation loses the sky.** With nothing carrying heat to the
   dark, the night side radiates against the planet's own interior heat and
   falls to 35.2 K — below the CO2 frost point (131.2 K at 400 ppm of a bar)
   and below the N2 condensation point (75.3 K at 0.78 bar). The atmosphere
   migrates to the night side and freezes onto it. With the cell running, the
   dark hemisphere sits near 229 K, nowhere near either.

## Why

The series' rule is that every number a post quotes is reproducible from a
tagged run. The climate paragraphs of "know your planet" and the RFP, and
both modes of the circulation plate, were the largest block of prose in
Series 1 with no run behind them.

The temptation was to fix that by modelling the circulation. That would have
been the wrong repair. This crate is closed-form screening math — the kind of
result a reader can check on paper — and an overturning velocity is not that.
A `wind_speed()` function fitted to reproduce 9 m/s would carry the authority
of the tagged-run rule while resting on nothing, which is strictly worse than
the citation it replaced: a citation announces its own provenance.

So the split is by what is derivable, not by what is wanted. Everything that
follows from an energy balance or the gas law is computed and asserted in
tests; the one quantity that needs a circulation model is named, sourced, and
left alone.

## Consequences

**The canonical band temperature is at the warm end.** +8 °C against a
fully-redistributed equilibrium of 229.4 K implies a **51.8 K** greenhouse
increment — 1.6× Earth's 33 K. This is defensible for this planet: published
habitability studies of Proxima b need CO2-rich atmospheres to keep the
terminator temperate at 0.65 S_Earth, and a ~50 K greenhouse is what that
looks like. It is recorded here because it is a real constraint the world
bible now carries: the band's air is doing substantially more work than
Earth's, and any future post about the atmosphere's composition has to be
consistent with that.

**Star luminosity becomes canon.** The world bible gave the star a mass
(0.122 solar) but no luminosity. Screening needs one, so 0.00155 solar
bolometric is added — the value that reproduces the published 0.65 S_Earth
for Proxima b at 0.0485 AU, which is the module's calibration test.

**Albedo is an argument, not a constant.** 0.3 is used for reporting. Nothing
in the series depends on it finely; the bracket is wide enough that the band
stays inside it across any plausible value.

## Alternatives considered

**A one-dimensional energy-balance model with latitudinal transport.** Would
have produced a band temperature rather than bracketing it. Rejected as scope:
it needs a diffusion coefficient that is itself tuned to GCM output, so the
"derivation" would import the same literature dependency while hiding it.

**Leave climate out entirely and mark the numbers as literature.** The
cheapest option, and it was live until the cold-trap claim was examined. That
claim — that this planet is habitable *because* its air circulates — is the
premise the inhabited band rests on, and it is closed-form. Leaving it
unevidenced while computing satellite node drift to four figures was the
wrong balance.
