# Terminus Manuscript Map

Post → book chapter tracking. One row per post; update when a post lands.

| # | Post (working title) | Series | Status | Evidence (scenario/example + tag) | Book chapter |
|---|---|---|---|---|---|
| 1 | The RFP | 1 | published | — (in-universe document) | 1.1 |
| 2 | Know your planet | 1 | published | terminus-orbits example `terminator_drift` (tag terminus-post-2) | 1.2 |
| 3 | The elegant trap | 1 | published | terminus-orbits example terminator_tracking (ADR-0001, tag terminus-post-3) | 1.3 |
| 4 | Shelves of the sky | 1 | published | terminus-orbits example regime_survey (ADR-0002, tag terminus-post-4) | 1.4 |
| 5 | Rings over twilight | 1 | published | terminus-orbits example access_constellation (ADR-0003, tag terminus-post-5) | 1.5 |
| 6 | Where the mind lives | 1 | published | terminus-orbits example compute_placement (ADR-0004, tag terminus-post-6) | 1.6 |
| 7 | Talking past a flaring red star | 1 | published | terminus-orbits example frequency_plan (ADR-0005, tag terminus-post-7) | 1.7 |
| 8 | Beams, not blankets | 1 | published | terminus-orbits examples spot_beams + terminal_aperture (ADR-0006/0013, tags terminus-post-8 through -8d, terminus-post-16) | 1.8 |
| 9 | First contact | 1 | published | terminus-orbits example first_contact (ADR-0007, tags terminus-post-9 through -9c) | 1.9 |
| 10 | The backbone | 1 | published | terminus-orbits examples backbone + clock_rates (ADR-0008/0009, tags terminus-post-11, terminus-post-12) | 1.10 |
| 11 | The unbroken thread | 1 | published | terminus-orbits example unbroken_thread (ADR-0010/0011, tags terminus-post-13, terminus-post-14) | 1.11 |
| 12 | The proposal rests | 1 | published | compliance-matrix.md (aggregates tags post-2..post-13; tags terminus-post-10..13) | 1.12 |

Series 1 published 2026-09-01 — all twelve posts are live under
https://www.eventhelix.com/terminus/, alongside the constellation explorer
and the-algorithms companion pages (neither is a book chapter, so neither
carries a row here). The site repo merged its `terminus` branch to `main`
and tagged the release `v1.5.0`; the author's detailed read follows against
the live pages, so corrections are still expected.

Tag note: `terminus-post-10` pins the original summary increment;
`terminus-post-11` pins the backbone increment plus the amended summary —
both posts 10 and 11 cite it. `terminus-post-16` pins the terminal-aperture
increment (ADR-0013), cited by posts 8 and 12; it adds a scan-loss term to
the link budget but changes no previously published number, since every one
of them was a boresight or ratio figure.

## Publish preconditions

All three were met before the 2026-09-01 publish; kept as the record.

- **Done (2026-09-01):** make the terminus repository public before the
  site's `terminus` branch merges: posts quote Rust excerpts and promise
  runnable examples, and every evidence tag must be publicly resolvable.
- Illustrations per the issue #2 conventions (dark-space technical
  infographics; satellites visibly sitting on their orbital shells;
  consistent orbit labels and colors — LEO access 2,200 km per ADR-0003,
  MEO service/compute/PNT 20,000 km; FEC and PNT diagram conventions as
  specified): at least the core architecture figure per post before the
  branch merges.
  - **Done (2026-08-30):** every post has an OG image and a body figure
    (posts 1–12; rings-over-twilight carries a mermaid diagram, post 12
    the architecture infographic via shortcode). Sources are held offline;
    the published WebP renditions live in the site repo.
  - Architecture figure regenerated with the corrected "LEO access
    (~2,200 km)" label (2026-08-21) — resolved.
- **Done (2026-08-30):** editorial review passes per issue #3 (cold read +
  continuity/fact-check) before the branch merges. Issue #3 closed
  2026-09-01. Caveat on the record there: the wavefront-compass rework of
  posts 8 and 9 (tag terminus-post-9d, ADR-0027/0028) postdates both
  passes and has not had an equivalent fresh-context review.
