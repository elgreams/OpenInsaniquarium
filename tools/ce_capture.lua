-- Read-only per-tick state capture from the original, for Cheat Engine.
-- Writes a harness trace (JSON Lines, format v1; see crates/harness/README.md).
--
-- Usage (from CE's Lua engine or the MCP bridge), after attaching to the game:
--   CAPTURE_OUT = "/tmp/trace.jsonl"; CAPTURE_TICKS = 300
--   dofile("/home/three/fsf/aquarium/tools/ce_capture.lua")
--
-- Read-only by design: no allocations, hooks, or code injection (injection
-- previously tripped the game's crash handler; see docs/re/dynamic-analysis.md).
-- Polls faster than the 28 ms tick and records each new update_count once,
-- so a tick can occasionally be missed; the diff tolerates gaps.

local APP_PTR      = 0x5eb680
local GLOBAL_RAND  = 0x5eb928
local OFF = {
  update_count = 0x484, board = 0x730, app_rand = 0x7b0,
  money = 0x3f0,
  obj_type = 0x8c,
}
-- Board vector offsets -> list names (docs/re/sim/board.md).
local LISTS = {
  {0xa0,"guppy"},{0xa4,"oscar"},{0xd4,"ultra"},{0xcc,"gekko"},{0xd0,"penta"},
  {0xc4,"grubber"},{0xc0,"breeder"},{0xb0,"pet_other"},{0xb4,"pet_fish"},
  {0xb8,"alien"},{0xbc,"alien_boss"},{0xf4,"bilaterus"},{0xa8,"coin"},
  {0xe8,"coin_b"},{0xec,"coin_c"},{0xac,"food"},{0xc8,"larva"},{0xdc,"missile"},
  {0xe0,"missile_b"},{0xd8,"shot"},{0x98,"dead_fish"},{0x9c,"dead_alien"},
  {0xe4,"shadow"},{0xf0,"warp"},{0xf8,"special"},
}

-- Lists dumped as raw bytes: {board offset, name, bytes}.
local RAW_LISTS = {
  {0xac,"food",0x200},{0xa8,"coin",0x200},{0xe8,"coin_b",0x200},{0xec,"coin_c",0x200},
}

local out_path = CAPTURE_OUT or "/tmp/trace.jsonl"
local max_ticks = CAPTURE_TICKS or 300

local function u32(a) local v = readInteger(a); return v and (v & 0xffffffff) end
local function f64(a) local v = readDouble(a); return v end
local function num(v) if v == nil then return "null" end
  if math.type(v) == "integer" then return tostring(v) end
  return string.format("%.17g", v) end
local function i32(a) return readInteger(a, true) end
-- Full object bytes as hex, so harness tests can rebuild exact state.
local function hex_block(a, n)
  local b = readBytes(a, n, true)
  if not b then return "" end
  local t = {}
  for i = 1, #b do t[i] = string.format("%02x", b[i]) end
  return table.concat(t)
end

local function rand_state(base)
  local b = readBytes(base, 0x9c4, true)
  if not b then return nil end
  local mt = {}
  for i = 0, 623 do
    mt[#mt+1] = b[4*i+1] | (b[4*i+2] << 8) | (b[4*i+3] << 16) | (b[4*i+4] << 24)
  end
  local mti = b[0x9c1] | (b[0x9c2] << 8) | (b[0x9c3] << 16) | (b[0x9c4] << 24)
  return mt, mti
end

local function rand_json(mt, mti)
  return string.format('{"mt":[%s],"mti":%d}', table.concat(mt, ","), mti)
end

local function vec_ptrs(vec)
  if not vec or vec == 0 then return {} end
  local first, last = u32(vec + 4), u32(vec + 8)
  local res = {}
  if not first or not last or last < first or (last - first) > 4096 then return res end
  for p = first, last - 4, 4 do res[#res+1] = u32(p) end
  return res
end

local app = u32(APP_PTR)
if not app or app == 0 then error("app not found; attach to the game first") end

local f = assert(io.open(out_path, "w"))
local function emit(s) f:write(s, "\n") end

local board = u32(app + OFF.board)
local start_tick = i32(app + OFF.update_count)
local amt, ami = rand_state(u32(app + OFF.app_rand))
local gmt, gmi = rand_state(GLOBAL_RAND)
emit(string.format(
  '{"format":"insaniquarium-trace","version":1,"source":"original","build":"f54c2c6ee54b00ae6da7f4bde15feb90d0a867aba6b4151b0277c1f81da3ff66","start_tick":%d,"rng":{"app":%s,"global":%s},"fields":["board.money","counts","guppies","rng_mti"]}',
  start_tick, rand_json(amt, ami), rand_json(gmt, gmi)))

local last = start_tick - 1
local captured, polls = 0, 0
while captured < max_ticks and polls < max_ticks * 40 do
  polls = polls + 1
  local t = i32(app + OFF.update_count)
  if t and t ~= last then
    last = t
    board = u32(app + OFF.board)
    local counts = {}
    if board and board ~= 0 then
      for _, l in ipairs(LISTS) do
        local n = #vec_ptrs(u32(board + l[1]))
        if n > 0 then counts[#counts+1] = string.format('"%s":%d', l[2], n) end
      end
    end
    -- Per-guppy state (docs/re/objects/fish-guppy.md).
    local fish = {}
    if board and board ~= 0 then
      for _, p in ipairs(vec_ptrs(u32(board + 0xa0))) do
        fish[#fish+1] = string.format(
          '{"x":%s,"y":%s,"vx":%s,"vy":%s,"div":%s,"hunger":%s,"mode":%s,"mode_t":%s,"reroll_t":%s,"stage":%s,"coin_t":%s,"coin_iv":%s,"raw":"%s"}',
          num(f64(p+0x158)), num(f64(p+0x160)), num(f64(p+0x168)), num(f64(p+0x170)), num(f64(p+0x180)),
          num(i32(p+0x9c)), num(i32(p+0x1ac)), num(i32(p+0x1b0)), num(i32(p+0x1b4)), num(i32(p+0x1a0)),
          num(i32(p+0x220)), num(i32(p+0x224)), hex_block(p, 0x230))
      end
    end
    -- Raw bytes of other modelled objects, for state rebuild and offset RE.
    local raws = {}
    if board and board ~= 0 then
      for _, l in ipairs(RAW_LISTS) do
        local items = {}
        for _, p in ipairs(vec_ptrs(u32(board + l[1]))) do
          items[#items+1] = '"' .. hex_block(p, l[3]) .. '"'
        end
        if #items > 0 then raws[#raws+1] = string.format('"%s":[%s]', l[2], table.concat(items, ",")) end
      end
    end
    -- Optional: publish the first guppy's widget position for a scripted
    -- clicker (e.g. to keep it fed during long captures).
    if CAPTURE_GPOS and board and board ~= 0 then
      local g = vec_ptrs(u32(board + 0xa0))[1]
      if g then
        local gf = io.open(CAPTURE_GPOS, "w")
        if gf then
          gf:write(string.format("%d %d %d\n", i32(g + 0x30), i32(g + 0x34), readBytes(board + 0x94, 1) or 0))
          -- Uncollected coins, one "x y" per line, for a collecting clicker.
          for _, c in ipairs(vec_ptrs(u32(board + 0xa8))) do
            if (readBytes(c + 0x198, 1) or 1) == 0 then
              gf:write(string.format("%d %d\n", i32(c + 0x30), i32(c + 0x34)))
            end
          end
          gf:close()
        end
      end
    end
    local _, ami2 = rand_state(u32(app + OFF.app_rand))
    local _, gmi2 = rand_state(GLOBAL_RAND)
    -- Board scalars the sim needs to resume mid-game: update clock (+0x28)
    -- and the per-sample last-played table (+0x1a4, 64 ints).
    local bclock, samples = "null", ""
    if board and board ~= 0 then
      bclock = tostring(i32(board + 0x28)) .. string.format(',"tank":%d,"level":%d,"paused":%s', i32(board + 0x3c8), i32(board + 0x3cc),
        (readBytes(board + 0x94, 1) or 0) ~= 0 and "true" or "false")
      samples = hex_block(board + 0x1a4, 0x100)
      bclock = bclock .. string.format(',"eggs":%d,"revealed":"%s","flash":%d',
        i32(board + 0x43c), hex_block(board + 0x3a4, 12), i32(board + 0x450))
    end
    emit(string.format('{"tick":%d,"board":{"money":%s,"clock":%s,"samples":"%s"},"counts":{%s},"guppies":[%s],"raw":{%s},"rng_mti":{"app":%d,"global":%d}}',
      t, tostring(board and i32(board + OFF.money) or "null"), bclock, samples, table.concat(counts, ","), table.concat(fish, ","),
      table.concat(raws, ","), ami2 or -1, gmi2 or -1))
    captured = captured + 1
  end
  sleep(5)
end
f:close()
return string.format("captured %d ticks (%d polls) to %s", captured, polls, out_path)
