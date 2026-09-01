# Contributing to terminus

Bug reports, reproductions, and questions are welcome — open an issue.

## Licensing of contributions

terminus code is licensed under the
[GNU Affero General Public License v3.0](LICENSE), and EventHelix.com Inc.
holds the copyright in it. Because the project may later relicense individual
crates under permissive terms when they are split into their own repositories,
we can only accept code contributions from people willing to assign copyright
in their contribution to EventHelix.com Inc., or to license it to us under
MIT OR Apache-2.0 — either of which lets us include the work both in this
AGPL repository and in a future permissive release.

Documents under `docs/terminus/` are the Terminus series canon, licensed
CC BY-NC-ND 4.0. We do not accept outside edits to them; please open an issue
if you spot an error.

By opening a pull request you confirm that you own the work you are submitting
and agree to those terms. If you would rather not, please open an issue
describing the change instead of sending code; that is genuinely useful and
carries no licensing question.

## Working on the code

- Read [`CLAUDE.md`](CLAUDE.md) first. It records the invariants — real bytes
  everywhere, determinism, fail-fast configuration, and engine/series
  separation — that reviews will hold you to.
- Read [`docs/runbook.md`](docs/runbook.md) for build, test, capture
  inspection, and dissector regeneration.
- `cargo test --workspace` must pass. The determinism and golden-digest tests
  compare exact bytes; if one fails, the change is not yet correct.
- Conventional-commit subjects (`feat:`, `fix:`, `docs:`, `chore:`).
- Do not put Terminus or Proxima concepts, constants, or narrative into any
  crate's library source. Planets, stars, and constellations are configuration.
  `crates/orbits/examples/` is the exception.

## Security

Please report suspected vulnerabilities privately via
[SECURITY.md](SECURITY.md) rather than in a public issue.
