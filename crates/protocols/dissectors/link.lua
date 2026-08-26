-- Utils section
function enforce_len_limit(num, limit, tree)
    if num == nil then
        return limit
    end
    if num > limit then
        tree:add_expert_info(PI_MALFORMED, PI_ERROR,
            "Expected " .. num .. " bytes, but only " .. limit .. " bytes remaining")
        return limit
    end
    return num
end

function sum_or_nil(...)
    local sum = 0
    local params = table.pack(...)
    for i = 1, params.n do
        if params[i] == nil then
            return nil
        end
        sum = sum + params[i]
    end
    return sum
end

function get_ftype(bit_offset, bitlen)
    local effective_len = bit_offset % 8 + bitlen
    if effective_len <= 8 then
        return ftypes.UINT8, 8
    elseif effective_len <= 16 then
        return ftypes.UINT16, 16
    elseif effective_len <= 32 then
        return ftypes.UINT32, 32
    elseif effective_len <= 64 then
        return ftypes.UINT64, 64
    else
        return ftypes.BYTES, effective_len
    end
end

function get_value(buf, is_little_endian)
    local len = buf:len()
    if len >= 1 and len <= 4 then
        if is_little_endian then
            return buf:le_uint()
        else
            return buf:uint()
        end
    elseif len >= 5 and len <= 8 then
        if is_little_endian then
            return buf:le_uint64()
        else
            return buf:uint64()
        end
    else
        return buf:bytes()
    end
end

AlignedProtoField = {}
function AlignedProtoField:new(o)
    local o = o or {
        name = nil,
        abbr = nil,
        ftype = nil,
        valuestring = nil,
        base = nil,
        is_little_endian = nil,
        description = nil, -- optional
    }
    o.field = ProtoField.new(o.name, o.abbr, o.ftype, o.valuestring, o.base, nil, o.description)
    setmetatable(o, self)
    self.__index = self
    return o
end

function AlignedProtoField:dissect(tree, buffer, runtime_len)
    local subtree
    if self.is_little_endian then
        subtree = tree:add_le(self.field, buffer(i, runtime_len))
    else
        subtree = tree:add(self.field, buffer(i, runtime_len))
    end
    return subtree, get_value(buffer(i, runtime_len), self.is_little_endian), runtime_len * 8
end

UnalignedProtoField = {}
function UnalignedProtoField:new(o)
    local o = o or {
        name = nil,
        abbr = nil,
        ftype = nil,
        bitoffset = nil,
        bitlen = nil, -- optional
        valuestring = nil, -- optional
        description = nil, -- optional
    }
    o.field = ProtoField.new(o.name, o.abbr, o.ftype, nil, nil, nil, o.description)
    setmetatable(o, self)
    self.__index = self
    return o
end
-- Adds dissection info into `tree`, and returns (value, bit_length)
function UnalignedProtoField:dissect(tree, buffer, runtime_len)
    local bitlen = nil_coalesce(self.bitlen, runtime_len * 8)
    local numbytes = math.ceil((bitlen + self.bitoffset) / 8)
    local buf = buffer(0, numbytes)
    local value = buf:bitfield(self.bitoffset, bitlen)
    local label = string.rep(".", self.bitoffset) -- First add `offset` number of dots to represent insignificant bits
    for i = self.bitoffset, self.bitoffset + bitlen - 1 do
        label = label .. buf:bitfield(i, 1) -- Then add the binary value
    end
    -- Then add the remaining insignificant bits as dots
    label = label .. string.rep(".", numbytes * 8 - bitlen - self.bitoffset)
    label = format_bitstring(label) .. " = " .. self.name
    label = label .. ": " .. self:get_value_display_string(value) -- Print out the string label
    local subtree = tree:add(self.field, buf, value, label)
    return subtree, value, bitlen
end

function UnalignedProtoField:get_value_display_string(value)
    if self.valuestring ~= nil then
        for _, range in ipairs(self.valuestring) do
            if range[1] <= value and value <= range[2] then
                return range[3] .. " (" .. value .. ")"
            end
        end
    end
    return value
end

ProtoEnum = {}
function ProtoEnum:new()
    local o = {
        by_value = {},
        matchers = {}
    }
    setmetatable(o, self)
    self.__index = self
    return o
end

-- name: string
-- value: number | {min, max} (Range) | nil (Remaining)
function ProtoEnum:define(name, value)
    if value == nil then
        setmetatable(self.by_value, {
            __index = function()
                return name
            end
        })
        table.insert(self.matchers, {0, 2 ^ 1024, name})
    elseif type(value) == "table" then
        table.insert(self.matchers, {value[1], value[2], name})
    else
        self.by_value[value] = name
        table.insert(self.matchers, {value, value, name})
    end
end

function ProtoEnum:match(enum_name, value)
    for k,matcher in pairs(self.matchers) do
        if matcher[1] <= value and value <= matcher[2] then
            return matcher[3] == enum_name
        end
    end
    return false
end

-- Add a space every 4 characters in the string
-- Example: 0010010101 -> 0010 0101 01
function format_bitstring(input)
    return input:gsub("....", "%0 "):gsub(" $", "")
end

function nil_coalesce(a, b)
    if a ~= nil then
        return a
    else
        return b
    end
end

ARRAY_MAX_COUNT = 65536

-- End Utils section
FrameType_enum = ProtoEnum:new()
FrameType_enum:define("DATA", 0)
FrameType_enum:define("CONTROL", 1)
function LinkFrame_protocol_fields(fields, path)
    fields[path .. "._fixed_0"] = AlignedProtoField:new({
        name = "Fixed value",
        abbr = path .. "._fixed_0",
        ftype = ftypes.UINT8,
        bitlen = 8,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. ".version"] = UnalignedProtoField:new({
        name = "version",
        abbr = path .. ".version",
        ftype = ftypes.UINT8,
        bitoffset = 0,
        bitlen = 4,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. "._reserved_0"] = UnalignedProtoField:new({
        name = "Reserved",
        abbr = path .. "._reserved_0",
        ftype = ftypes.UINT8,
        bitoffset = 4,
        bitlen = 4,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. ".frame_type"] = AlignedProtoField:new({
        name = "frame_type",
        abbr = path .. ".frame_type",
        ftype = ftypes.UINT8,
        valuestring = FrameType_enum.matchers,
        base = base.RANGE_STRING,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. ".src"] = AlignedProtoField:new({
        name = "src",
        abbr = path .. ".src",
        ftype = ftypes.UINT16,
        bitlen = 16,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. ".dst"] = AlignedProtoField:new({
        name = "dst",
        abbr = path .. ".dst",
        ftype = ftypes.UINT16,
        bitlen = 16,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. ".seq"] = AlignedProtoField:new({
        name = "seq",
        abbr = path .. ".seq",
        ftype = ftypes.UINT32,
        bitlen = 32,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. "._body__size"] = AlignedProtoField:new({
        name = "Size(_body_)",
        abbr = path .. "._body__size",
        ftype = ftypes.UINT16,
        bitlen = 16,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. "._body_"] = AlignedProtoField:new({
        name = "Body",
        abbr = path .. "._body_",
        ftype = ftypes.BYTES,
        bitlen = nil,
        is_little_endian = false,
        description = nil,
    })
    DataFrame_protocol_fields(fields, path .. ".DataFrame")
    ControlFrame_protocol_fields(fields, path .. ".ControlFrame")
end
-- Sequence: LinkFrame (9 fields, 2 children, 0 constraints)
function LinkFrame_dissect(buffer, pinfo, tree, fields, path)
    local i = 0
    local field_values = {}
    -- Scalar: Fixed value
    local field_len = enforce_len_limit(1, buffer(i):len(), tree)
    subtree, field_values[path .. "._fixed_0"], bitlen = fields[path .. "._fixed_0"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
    local value = field_values[path .. "._fixed_0"]
    if not (value == 72) then
        subtree:add_expert_info(PI_MALFORMED, PI_WARN, "Error: Expected `value == 72` where value=" .. tostring(value))
    end
    -- Scalar: version
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
    -- Typedef: frame_type
    local field_len = enforce_len_limit(math.ceil(1), buffer(i):len(), tree)
    subtree, field_values[path .. ".frame_type"], bitlen = fields[path .. ".frame_type"]:dissect(tree, buffer(i), field_len)
    if FrameType_enum.by_value[field_values[path .. ".frame_type"]] == nil then
        tree:add_expert_info(PI_MALFORMED, PI_WARN, "Unknown enum value: " .. field_values[path .. ".frame_type"])
    end
    i = i + bitlen / 8
    -- Scalar: src
    local field_len = enforce_len_limit(2, buffer(i):len(), tree)
    subtree, field_values[path .. ".src"], bitlen = fields[path .. ".src"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
    -- Scalar: dst
    local field_len = enforce_len_limit(2, buffer(i):len(), tree)
    subtree, field_values[path .. ".dst"], bitlen = fields[path .. ".dst"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
    -- Scalar: seq
    local field_len = enforce_len_limit(4, buffer(i):len(), tree)
    subtree, field_values[path .. ".seq"], bitlen = fields[path .. ".seq"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
    -- Scalar: Size(_body_)
    local field_len = enforce_len_limit(2, buffer(i):len(), tree)
    subtree, field_values[path .. "._body__size"], bitlen = fields[path .. "._body__size"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
    -- Payload: Body
    local field_len = enforce_len_limit(sum_or_nil(0, field_values[path .. "._body__size"]), buffer(i):len(), tree)
    if DataFrame_match_constraints(field_values, path) then
        local subtree = tree:add("DataFrame")
        local dissected_len = DataFrame_dissect(buffer(i, field_len), pinfo, subtree, fields, path .. ".DataFrame")
        i = i + dissected_len
    elseif ControlFrame_match_constraints(field_values, path) then
        local subtree = tree:add("ControlFrame")
        local dissected_len = ControlFrame_dissect(buffer(i, field_len), pinfo, subtree, fields, path .. ".ControlFrame")
        i = i + dissected_len
    else
        subtree, field_values[path .. "._body_"], bitlen = fields[path .. "._body_"]:dissect(tree, buffer(i), field_len)
        i = i + bitlen / 8
    end
    return i
end
function LinkFrame_match_constraints(field_values, path)
    return true
end
function DataFrame_protocol_fields(fields, path)
    fields[path .. ".flow_id"] = AlignedProtoField:new({
        name = "flow_id",
        abbr = path .. ".flow_id",
        ftype = ftypes.UINT8,
        bitlen = 8,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. "._payload_"] = AlignedProtoField:new({
        name = "Payload",
        abbr = path .. "._payload_",
        ftype = ftypes.BYTES,
        bitlen = nil,
        is_little_endian = false,
        description = nil,
    })
end
-- Sequence: DataFrame (2 fields, 0 children, 1 constraints)
function DataFrame_dissect(buffer, pinfo, tree, fields, path)
    local i = 0
    local field_values = {}
    -- Scalar: flow_id
    local field_len = enforce_len_limit(1, buffer(i):len(), tree)
    subtree, field_values[path .. ".flow_id"], bitlen = fields[path .. ".flow_id"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
    -- Payload: Payload
    local field_len = enforce_len_limit(sum_or_nil(0, field_values[path .. "._payload__size"]), buffer(i):len(), tree)
    subtree, field_values[path .. "._payload_"], bitlen = fields[path .. "._payload_"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
    return i
end
function DataFrame_match_constraints(field_values, path)
    return FrameType_enum:match("DATA", field_values[path .. ".frame_type"])
end
function ControlFrame_protocol_fields(fields, path)
    fields[path .. ".opcode"] = AlignedProtoField:new({
        name = "opcode",
        abbr = path .. ".opcode",
        ftype = ftypes.UINT8,
        bitlen = 8,
        is_little_endian = false,
        description = nil,
    })
    fields[path .. ".args"] = AlignedProtoField:new({
        name = "args",
        abbr = path .. ".args",
        ftype = ftypes.UINT8,
        bitlen = 8,
        is_little_endian = false,
        description = nil,
    })
end
-- Sequence: ControlFrame (2 fields, 0 children, 1 constraints)
function ControlFrame_dissect(buffer, pinfo, tree, fields, path)
    local i = 0
    local field_values = {}
    -- Scalar: opcode
    local field_len = enforce_len_limit(1, buffer(i):len(), tree)
    subtree, field_values[path .. ".opcode"], bitlen = fields[path .. ".opcode"]:dissect(tree, buffer(i), field_len)
    i = i + bitlen / 8
    -- ScalarArray: args
    local initial_i = i
    while i < buffer:len() do    -- ScalarArray: args
        local field_len = enforce_len_limit(1, buffer(i):len(), tree)
        subtree, field_values[path .. ".args"], bitlen = fields[path .. ".args"]:dissect(tree, buffer(i), field_len)
        i = i + bitlen / 8
    end
    return i
end
function ControlFrame_match_constraints(field_values, path)
    return FrameType_enum:match("CONTROL", field_values[path .. ".frame_type"])
end
-- Protocol definition for "LinkFrame"
LinkFrame_protocol = Proto("LinkFrame",  "LinkFrame")
LinkFrame_protocol_fields_table = {}
function LinkFrame_protocol.dissector(buffer, pinfo, tree)
    pinfo.cols.protocol = "LinkFrame"
    local subtree = tree:add(LinkFrame_protocol, buffer(), "LinkFrame")
    local i = LinkFrame_dissect(buffer, pinfo, subtree, LinkFrame_protocol_fields_table, "LinkFrame")
    if buffer(i):len() > 0 then
        local remaining_bytes = buffer:len() - i
        if math.floor(remaining_bytes) == remaining_bytes then
            subtree:add_expert_info(PI_MALFORMED, PI_WARN, "Error: " .. remaining_bytes .. " undissected bytes remaining")
        else
            subtree:add_expert_info(PI_MALFORMED, PI_WARN, "Error: " .. (remaining_bytes * 8) .. " undissected bits remaining")
        end
    end
end
LinkFrame_protocol_fields(LinkFrame_protocol_fields_table, "LinkFrame")
for name,field in pairs(LinkFrame_protocol_fields_table) do
    LinkFrame_protocol.fields[name] = field.field
end
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
