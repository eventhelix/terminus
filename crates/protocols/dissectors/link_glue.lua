-- SPDX-License-Identifier: AGPL-3.0-only
-- Copyright (C) 2026 EventHelix.com Inc.

-- ---------------------------------------------------------------------------
-- terminus glue (appended by tools/regen-dissector.sh — do not hand-edit
-- link.lua; edit link_glue.lua and regenerate).
--
-- Registers the generated LinkFrame dissector on LINKTYPE_USER0 (147),
-- which Wireshark exposes to Lua as wtap.USER0, and chains DATA-frame
-- payloads to the built-in IP dissector. The DATA payload offset (14 =
-- 13-byte LinkFrame header + 1-byte flow_id) is pinned by the Rust test
-- `data_payload_offset_is_14` in tests/link_roundtrip.rs.
-- ---------------------------------------------------------------------------
local ip_dissector = Dissector.get("ip")
local terminus_proto = Proto("terminus", "terminus link layer")

function terminus_proto.dissector(buffer, pinfo, tree)
    LinkFrame_protocol.dissector(buffer, pinfo, tree)
    -- frame_type is byte 2; DATA = 0. Chain the embedded IP packet.
    if buffer:len() > 14 and buffer(2, 1):uint() == 0 then
        ip_dissector:call(buffer(14):tvb(), pinfo, tree)
    end
end

DissectorTable.get("wtap_encap"):add(wtap.USER0, terminus_proto)
