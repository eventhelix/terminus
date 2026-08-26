# Terminus Manuscript Map

Post → book chapter tracking. One row per post; update when a post lands.

| # | Post (working title) | Series | Status | Evidence (scenario/example + tag) | Book chapter |
|---|---|---|---|---|---|
| 1 | The RFP | 1 | drafting | — (in-universe document) | 1.1 |
| 2 | Know your planet | 1 | drafted | terminus-orbits example `terminator_drift` (tag terminus-post-2) | 1.2 |
| 3 | The elegant trap | 1 | drafted | terminus-orbits example terminator_tracking (ADR-0001, tag terminus-post-3) | 1.3 |
| 4 | Shelves of the sky | 1 | drafted | terminus-orbits example regime_survey (ADR-0002, tag terminus-post-4) | 1.4 |
| 5 | Rings over twilight | 1 | drafted | terminus-orbits example access_constellation (ADR-0003, tag terminus-post-5) | 1.5 |
| 6 | Where the mind lives | 1 | drafted | terminus-orbits example compute_placement (ADR-0004, tag terminus-post-6) | 1.6 |
| 7 | Talking past a flaring red star | 1 | drafted | terminus-orbits example frequency_plan (ADR-0005, tag terminus-post-7) | 1.7 |
| 8 | Beams, not blankets | 1 | drafted | terminus-orbits examples spot_beams + terminal_aperture (ADR-0006/0013, tags terminus-post-8, terminus-post-16) | 1.8 |
| 9 | First contact | 1 | drafted | terminus-orbits example first_contact (ADR-0007, tag terminus-post-9) | 1.9 |
| 10 | The backbone | 1 | drafted | terminus-orbits examples backbone + clock_rates (ADR-0008/0009, tags terminus-post-11, terminus-post-12) | 1.10 |
| 11 | The unbroken thread | 1 | drafted | terminus-orbits example unbroken_thread (ADR-0010/0011, tags terminus-post-13, terminus-post-14) | 1.11 |
| 12 | The proposal rests | 1 | drafted | compliance-matrix.md (aggregates tags post-2..post-13; tags terminus-post-10..13) | 1.12 |

Series 1 drafting complete (posts 1–11 staged on the site's `terminus`
branch, pending pre-publish pass and the publish preconditions below).
Tag note: `terminus-post-10` pins the original summary increment;
`terminus-post-11` pins the backbone increment plus the amended summary —
both posts 10 and 11 cite it. `terminus-post-16` pins the terminal-aperture
increment (ADR-0013), cited by posts 8 and 12; it adds a scan-loss term to
the link budget but changes no previously published number, since every one
of them was a boresight or ratio figure.

## Publish preconditions

- Make the terminus repository public before the site's `terminus` branch
  merges: posts quote Rust excerpts and promise runnable examples, and every
  evidence tag must be publicly resolvable.
- Illustrations per the issue #2 conventions (dark-space technical
  infographics; satellites visibly sitting on their orbital shells;
  consistent orbit labels and colors — LEO access 2,200 km per ADR-0003,
  MEO service/compute/PNT 20,000 km; FEC and PNT diagram conventions as
  specified): at least the core architecture figure per post before the
  branch merges.
  - Placed so far (source: `C:\Users\sande\OneDrive\Projects\Terminus\media`,
    converted to WebP): planet-and-star hero (post 2), full architecture
    infographic (post 12).
  - Architecture figure regenerated with the corrected "LEO access
    (~2,200 km)" label (2026-08-21) — resolved.
  - Remaining: figures for posts 1, 3–11.
