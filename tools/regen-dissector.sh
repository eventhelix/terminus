#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 EventHelix.com Inc.

# Regenerate the checked-in Wireshark dissector from the PDL source.
# Prereq: cargo install pdl-dissector   (v0.1.0)
# Run from anywhere; on Windows use Git Bash.
set -euo pipefail
cd "$(dirname "$0")/.."

pdl-dissector crates/protocols/pdl/link.pdl LinkFrame \
    > crates/protocols/dissectors/link.lua

# --- Patch step: pdl-dissector v0.1.0 nibble-offset codegen bug ------------
# Stock codegen advances the byte cursor with `i = i + bitlen / 8` after
# EVERY field, including sub-byte ones. LinkFrame packs two 4-bit fields
# (version, _reserved_) into one byte, so the cursor lands on the
# non-integer offset 1.5 and the next buffer(i) call faults in tshark's
# Lua bindings ("number has no integer representation") -- an Expert Info
# (Error/Dissector bug) on every single frame. The transform below rewrites
# that one block; it FAILS the script if the generated text no longer
# matches (e.g. a future pdl-dissector changes codegen or fixes the bug
# upstream), so regen can never silently produce a broken or double-patched
# dissector. If it fails: inspect the new codegen, then update or delete
# this step, and re-verify with
#   cargo test -p terminus --test dissection -- --ignored
python - crates/protocols/dissectors/link.lua <<'PYEOF'
import sys

path = sys.argv[1]
with open(path, encoding="utf-8", newline="") as f:
    text = f.read()

OLD = '''    -- Scalar: version
    local field_len = enforce_len_limit(0.5, buffer(i):len(), tree)
    subtree, field_values[path .. ".version"], bitlen = fields[path .. ".version"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
    -- Scalar: Reserved
    local field_len = enforce_len_limit(0.5, buffer(i):len(), tree)
    subtree, field_values[path .. "._reserved_0"], bitlen = fields[path .. "._reserved_0"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
'''

NEW = '''    -- Scalar: version
    -- PATCH (applied by tools/regen-dissector.sh; do not hand-edit):
    -- pdl-dissector v0.1.0 codegen bug. "version" and "Reserved" are 4-bit
    -- fields packed into the SAME byte (bitoffset 0 and 4, see the
    -- UnalignedProtoField definitions above). Stock codegen advances `i` by
    -- `bitlen / 8` (= 0.5) after each nibble, so after "version" `i` is the
    -- non-integer 1.5 and the very next `buffer(i)` -- "Reserved"'s
    -- enforce_len_limit call -- faults ("number has no integer
    -- representation"), which tshark surfaces as an Expert Info
    -- (Error/Dissector bug) on every frame. Fix: do not advance `i` after
    -- "version" ("Reserved" reads the same byte via its own bitoffset),
    -- then advance one whole byte after "Reserved" once both nibbles of
    -- the shared byte are consumed.
    local field_len = enforce_len_limit(0.5, buffer(i):len(), tree)
    subtree, field_values[path .. ".version"], bitlen = fields[path .. ".version"]:dissect(tree, buffer(i), field_len)
    -- Scalar: Reserved
    local field_len = enforce_len_limit(0.5, buffer(i):len(), tree)
    subtree, field_values[path .. "._reserved_0"], bitlen = fields[path .. "._reserved_0"]:dissect(tree, buffer(i), field_len)
    i = i + 1
'''

n = text.count(OLD)
if n != 1:
    sys.stderr.write(
        f"regen-dissector: nibble-offset patch target found {n} times in "
        f"{path} (expected exactly 1).\n"
        "pdl-dissector codegen has changed (new version, or the v0.1.0 "
        "nibble-offset bug is fixed upstream). Refusing to write a possibly "
        "broken dissector -- review the generated output and update or "
        "remove the patch step in tools/regen-dissector.sh, then re-verify "
        "with: cargo test -p terminus --test dissection -- --ignored\n"
    )
    sys.exit(1)

with open(path, "w", encoding="utf-8", newline="") as f:
    f.write(text.replace(OLD, NEW, 1))
print("patched nibble-offset bug in crates/protocols/dissectors/link.lua")
PYEOF
# --- End patch step --------------------------------------------------------

cat crates/protocols/dissectors/link_glue.lua \
    >> crates/protocols/dissectors/link.lua
echo "regenerated crates/protocols/dissectors/link.lua"
