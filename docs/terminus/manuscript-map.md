# Terminus Manuscript Map

Post → book chapter tracking. One row per post; update when a post lands.

| # | Post (working title) | Series | Status | Evidence (scenario/example + tag) | Book chapter |
|---|---|---|---|---|---|
| 1 | The RFP | 1 | drafting | — (in-universe document) | 1.1 |
| 2 | Know your planet | 1 | drafted | helixsim-orbits example `terminator_drift` (tag terminus-post-2) | 1.2 |
| 3 | The elegant trap | 1 | drafted | helixsim-orbits example terminator_tracking (ADR-0001, tag terminus-post-3) | 1.3 |
| 4 | Shelves of the sky | 1 | drafted | helixsim-orbits example regime_survey (ADR-0002, tag terminus-post-4) | 1.4 |
| 5 | Rings over twilight | 1 | drafted | helixsim-orbits example access_constellation (ADR-0003, tag terminus-post-5) | 1.5 |
| 6 | Where the mind lives | 1 | drafted | helixsim-orbits example compute_placement (ADR-0004, tag terminus-post-6) | 1.6 |
| 7 | Talking past a flaring red star | 1 | drafted | helixsim-orbits example frequency_plan (ADR-0005, tag terminus-post-7) | 1.7 |
| 8 | Beams, not blankets | 1 | drafted | helixsim-orbits example spot_beams (ADR-0006, tag terminus-post-8) | 1.8 |
| 9 | First contact | 1 | drafted | helixsim-orbits example first_contact (ADR-0007, tag terminus-post-9) | 1.9 |
| 10 | The backbone | 1 | drafted | helixsim-orbits examples backbone + clock_rates (ADR-0008/0009, tags terminus-post-11, terminus-post-12) | 1.10 |
| 11 | The unbroken thread | 1 | drafted | helixsim-orbits example unbroken_thread (ADR-0010, tag terminus-post-13) | 1.11 |
| 12 | The proposal rests | 1 | drafted | compliance-matrix.md (aggregates tags post-2..post-13; tags terminus-post-10..13) | 1.12 |

Series 1 drafting complete (posts 1–11 staged on the site's `terminus`
branch, pending pre-publish pass and the publish preconditions below).
Tag note: `terminus-post-10` pins the original summary increment;
`terminus-post-11` pins the backbone increment plus the amended summary —
both posts 10 and 11 cite it.

## Publish preconditions

- Make the helixsim repository public before the site's `terminus` branch
  merges: posts quote Rust excerpts and promise runnable examples, and every
  evidence tag must be publicly resolvable.
