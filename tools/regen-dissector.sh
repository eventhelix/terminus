#!/usr/bin/env bash
# Regenerate the checked-in Wireshark dissector from the PDL source.
# Prereq: cargo install pdl-dissector   (v0.1.0)
# Run from anywhere; on Windows use Git Bash.
set -euo pipefail
cd "$(dirname "$0")/.."

pdl-dissector crates/protocols/pdl/link.pdl LinkFrame \
    > crates/protocols/dissectors/link.lua
cat crates/protocols/dissectors/link_glue.lua \
    >> crates/protocols/dissectors/link.lua
echo "regenerated crates/protocols/dissectors/link.lua"
