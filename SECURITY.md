# Security Policy

terminus is a simulator: it parses scenario TOML, channel-trace CSV, and
protocol bytes it generated itself. It is not intended to be exposed to
untrusted network input, and it makes no security guarantees when fed
adversarial captures or traces.

That said, if you find a vulnerability — a memory-safety issue, a crash on
malformed input, or anything that would matter to someone embedding these
crates — please report it privately rather than opening a public issue.

**Contact:** use GitHub's private vulnerability reporting on this repository
(Security -> Report a vulnerability).

Please include a description, the affected version or commit, and a
reproduction if you have one. We will acknowledge your report and let you know
how we plan to address it.
