-- Read-only per-tick state capture from the original, for Cheat Engine.
-- Writes a harness trace (JSON Lines, format v1; see crates/harness/README.md).
--
-- Usage (from CE's Lua engine or the MCP bridge), after attaching to the game:
--   CAPTURE_OUT = "/tmp/trace.jsonl"; CAPTURE_TICKS = 300
--   dofile("<repo>/tools/ce_capture.lua")
--
-- Read-only by design: no allocations, hooks, or code injection (injection
-- previously tripped the game's crash handler; see docs/re/dynamic-analysis.md).
-- Polls faster than the 28 ms tick and records each new update_count once,
-- so a tick can occasionally be missed; the diff tolerates gaps.

local APP_PTR      = 0x5eb680
local GLOBAL_RAND  = 0x5eb928
local OFF = {
  -- app+0x47c counts frames that slept (Process), app+0x484 counts logic
  -- updates (UpdateApp; framework/timing.md). Snapshots are keyed on 0x47c
  -- (one read per frame) and record 0x484 as "u", the demo's tick basis.
  update_count = 0x47c, updates = 0x484, board = 0x730, app_rand = 0x7b0,
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
  {0xb8,"alien",0x210},{0xbc,"alien_b",0x210},{0xd8,"shot",0x170},{0x9c,"dead_alien",0x1a8},
  {0x98,"dead_fish",0x1a8},{0xf0,"warp",0x170},{0xb0,"pet_other",0x1d0},{0xb4,"pet_fish",0x268},
  {0xe4,"shadow",0x170},{0xa4,"oscar",0x230},{0xd0,"penta",0x1b0},
  -- Pet-made objects (pet zoo): missiles/bombs and the misc list.
  {0xdc,"missile",0x200},{0xe0,"missile_b",0x200},{0xf8,"special",0x200},
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

-- Presentation state (docs/re/framework/presentation-capture.md): music
-- tracks (BassMusicInterface map at [app+0x374]+4: key +0x10, volume
-- +0x20, fade +0x28, max +0x30, stop-when-faded +0x38, handles +0x18/+0x1c),
-- the requested track/order (app+0x88c/+0x890), open dialogs (map at
-- app+0x324 (header at +0x328): key +0xc, dialog +0x10), and the screen widgets.
local SCREENS = {
  {0x730,"board"},{0x814,"hatch"},{0x80c,"pets"},{0x81c,"interlude"},
  {0x818,"bonus"},{0x738,"selector"},{0x834,"help"},
}

-- In-order walk of an MSVC std::map: header node at `head`, root at
-- head+4, left +0, right +8, isnil byte at `nil_off`.
local function map_nodes(head, nil_off)
  local res = {}
  if not head or head == 0 then return res end
  local function walk(n, depth)
    if not n or n == 0 or n == head or depth > 64 or (readBytes(n + nil_off, 1) or 1) ~= 0 then return end
    walk(u32(n), depth + 1)
    res[#res+1] = n
    walk(u32(n + 8), depth + 1)
  end
  walk(u32(head + 4), 0)
  return res
end

local function presentation(app)
  local parts = {}
  local music = u32(app + 0x374)
  local tracks = {}
  if music and music ~= 0 then
    for _, n in ipairs(map_nodes(u32(music + 8), 0x41)) do
      tracks[#tracks+1] = string.format('{"id":%d,"vol":%s,"fade":%s,"max":%s,"stop":%d,"h":%d}',
        i32(n + 0x10), num(f64(n + 0x20)), num(f64(n + 0x28)), num(f64(n + 0x30)),
        readBytes(n + 0x38, 1) or 0, u32(n + 0x18) or 0)
    end
  end
  parts[#parts+1] = string.format('"music":{"track":%d,"order":%d,"tracks":[%s]}',
    i32(app + 0x88c), i32(app + 0x890), table.concat(tracks, ","))
  local dialogs = {}
  for _, n in ipairs(map_nodes(u32(app + 0x328), 0x15)) do
    dialogs[#dialogs+1] = tostring(i32(n + 0xc))
  end
  parts[#parts+1] = string.format('"dialogs":[%s]', table.concat(dialogs, ","))
  local screens = {}
  for _, sc in ipairs(SCREENS) do
    local p = u32(app + sc[1])
    if p and p ~= 0 then screens[#screens+1] = '"' .. sc[2] .. '"' end
  end
  parts[#parts+1] = string.format('"screens":[%s]', table.concat(screens, ","))
  -- Hatch screen counter and pet (hatch-screen.md#2).
  local h = u32(app + 0x814)
  if h and h ~= 0 then
    parts[#parts+1] = string.format('"hatch":{"t":%d,"pet":%d,"jx":%d,"jy":%d}',
      i32(h + 0x94), i32(h + 0x98), i32(h + 0x9c), i32(h + 0xa0))
  end
  -- Pet screen (screens/pet-screen.md): its object raw, plus the profile's
  -- owned (+0x00) and chosen (+0x5a) pet bytes.
  local ps = u32(app + 0x80c)
  if ps and ps ~= 0 then
    local prof = u32(app + 0x8b8)
    -- Each pet button's chosen flag (button +0x130).
    local sel = {}
    for id = 0, 23 do
      local b = u32(ps + 0x9c + 4 * id)
      sel[#sel+1] = string.format("%02x", (b and b ~= 0) and (readBytes(b + 0x130, 1) or 0) or 0)
    end
    parts[#parts+1] = string.format('"petscreen":{"raw":"%s","owned":"%s","chosen":"%s","buttons":"%s"}',
      hex_block(ps, 0x200), hex_block(prof, 24), hex_block(prof + 0x5a, 24), table.concat(sel))
  end
  return "{" .. table.concat(parts, ",") .. "}"
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
          '{"x":%s,"y":%s,"vx":%s,"vy":%s,"div":%s,"hunger":%s,"mode":%s,"mode_t":%s,"reroll_t":%s,"stage":%s,"coin_t":%s,"coin_iv":%s,"ptr":%d,"raw":"%s"}',
          num(f64(p+0x158)), num(f64(p+0x160)), num(f64(p+0x168)), num(f64(p+0x170)), num(f64(p+0x180)),
          num(i32(p+0x9c)), num(i32(p+0x1ac)), num(i32(p+0x1b0)), num(i32(p+0x1b4)), num(i32(p+0x1a0)),
          num(i32(p+0x220)), num(i32(p+0x224)), p, hex_block(p, 0x230))
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
    -- Optional: publish board state for a scripted player (CAPTURE_GPOS).
    -- Line 1: "S paused money eggs revealed_guppy revealed_egg foods"
    -- then "G x y hunger" per guppy, "C x y collected" per coin.
    if CAPTURE_GPOS and board and board ~= 0 then
      local gf = io.open(CAPTURE_GPOS .. ".tmp", "w")
      if gf then
        gf:write(string.format("S %d %d %d %d %d %d\n", readBytes(board + 0x94, 1) or 0,
          i32(board + OFF.money), i32(board + 0x43c), readBytes(board + 0x3a4, 1) or 0,
          readBytes(board + 0x3a4 + 11, 1) or 0, #vec_ptrs(u32(board + 0xac))))
        for _, g in ipairs(vec_ptrs(u32(board + 0xa0))) do
          gf:write(string.format("G %d %d %d %d\n", i32(g + 0x30), i32(g + 0x34), i32(g + 0x9c), i32(g + 0x1a0)))
        end
        for _, c in ipairs(vec_ptrs(u32(board + 0xa8))) do
          gf:write(string.format("C %d %d %d %d\n", i32(c + 0x30), i32(c + 0x34), readBytes(c + 0x198, 1) or 0, i32(c + 0x194)))
        end
        for _, a in ipairs(vec_ptrs(u32(board + 0xb8))) do
          gf:write(string.format("A %d %d\n", i32(a + 0x30), i32(a + 0x34)))
        end
        -- Missiles: "M x y launch kind" (alien-kind5.md).
        for _, m in ipairs(vec_ptrs(u32(board + 0xdc))) do
          gf:write(string.format("M %d %d %d %d\n", i32(m + 0x30), i32(m + 0x34), i32(m + 0x17c), i32(m + 0x19c)))
        end
        -- Coins in the other coin lists (pearls etc.), carnivores' hunger.
        for _, off in ipairs({0xe8, 0xec}) do
          for _, c in ipairs(vec_ptrs(u32(board + off))) do
            gf:write(string.format("C %d %d %d\n", i32(c + 0x30), i32(c + 0x34), readBytes(c + 0x198, 1) or 0))
          end
        end
        for _, o in ipairs(vec_ptrs(u32(board + 0xa4))) do
          gf:write(string.format("O %d %d %d\n", i32(o + 0x30), i32(o + 0x34), i32(o + 0x9c)))
        end
        -- Fish-shaped pets: "P id x y timer interval presto recharge" (Amp's
        -- charge, Presto's flag and recharge).
        for _, o in ipairs(vec_ptrs(u32(board + 0xb4))) do
          gf:write(string.format("P %d %d %d %d %d %d %d\n", i32(o + 0x234), i32(o + 0x30), i32(o + 0x34),
            i32(o + 0x220), i32(o + 0x224), readBytes(o + 0x230, 1) or 0, i32(o + 0x238)))
        end
        -- Other pets: "Q id x y presto recharge".
        for _, o in ipairs(vec_ptrs(u32(board + 0xb0))) do
          gf:write(string.format("Q %d %d %d %d %d\n", i32(o + 0x1c4), i32(o + 0x30), i32(o + 0x34),
            readBytes(o + 0x1c8, 1) or 0, i32(o + 0x1ac)))
        end
        gf:write(string.format("R %s\n", hex_block(board + 0x3a4, 12)))
        -- Open dialogs: id, x, y, w, h (to click their button).
        for _, n in ipairs(map_nodes(u32(app + 0x328), 0x15)) do
          local dlg = u32(n + 0x10)
          gf:write(string.format("D %d %d %d %d %d\n", i32(n + 0xc), i32(dlg + 0x30), i32(dlg + 0x34), i32(dlg + 0x38), i32(dlg + 0x3c)))
        end
        gf:close()
        os.rename(CAPTURE_GPOS .. ".tmp", CAPTURE_GPOS)
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
      bclock = bclock .. string.format(',"eggs":%d,"revealed":"%s","flash":%d,"ticks":%d',
        i32(board + 0x43c), hex_block(board + 0x3a4, 12), i32(board + 0x450), i32(board + 0x444))
      -- Alien waves and weapon (alien-sylvester.md#board-fields).
      bclock = bclock .. string.format(',"wave":{"kind":%d,"countdown":%d,"t2c4":%d,"lockout":%d,"ax":%d,"ay":%d,"bx":%d,"by":%d,"waves":%d},"laser":%d,"food_level":%d,"max_food":%d',
        i32(board + 0x2bc), i32(board + 0x2c0), i32(board + 0x2c4), i32(board + 0x2c8),
        i32(board + 0x2cc), i32(board + 0x2d0), i32(board + 0x2d4), i32(board + 0x2d8),
        i32(board + 0x45c), i32(board + 0x3e4), i32(0x5e89dc), i32(0x5df58c))
      bclock = bclock .. string.format(',"shot_x":%d,"shot_y":%d', i32(board + 0x3d0), i32(board + 0x3d4))
      -- Message line (MessageWidget at +0x118; board.md#message-line):
      -- timer +0xd0, flash +0xd4, hint id +0xd8, text = std::string at +0x8c.
      local mw = u32(board + 0x118)
      if mw ~= 0 then
        local s = mw + 0x8c
        local len, cap = i32(s + 0x14), i32(s + 0x18)
        local p = cap >= 16 and u32(s + 4) or (s + 4)
        local text = len > 0 and (readString(p, len, false) or "") or ""
        bclock = bclock .. string.format(',"msg":{"t":%d,"flash":%d,"hint":%d,"text":"%s"}',
          i32(mw + 0xd0), readBytes(mw + 0xd4, 1) or 0, i32(mw + 0xd8), text:gsub('[\\"]', ''))
      end
      -- Light-band x positions, f32 (board.md#backdrop).
      bclock = bclock .. string.format(',"lights":[%.9g,%.9g,%.9g]',
        readFloat(board + 0x3d8), readFloat(board + 0x3dc), readFloat(board + 0x3e0))
      -- Hint bytes +0x4b0..+0x4e5 (one-time hint flags; dialog.md).
      bclock = bclock .. string.format(',"hints":"%s"', hex_block(board + 0x4b0, 0x36))
      -- Owned pets (profile +0x00..+0x17): the PetDialog's enabled buttons
      -- (pet-presto.md#the-petdialog-0x005355c0).
      local prof = u32(app + 0x8b8)
      if prof and prof ~= 0 then
        bclock = bclock .. string.format(',"owned":"%s"', hex_block(prof, 24))
      end
    end
    local board_json = "null"
    if board and board ~= 0 then
      board_json = string.format('{"money":%s,"clock":%s,"samples":"%s"}', tostring(i32(board + OFF.money)), bclock, samples)
    end
    emit(string.format('{"tick":%d,"u":%d,"board":%s,"pres":%s,"counts":{%s},"guppies":[%s],"raw":{%s},"rng_mti":{"app":%d,"global":%d}}',
      t, i32(app + OFF.updates), board_json, presentation(app), table.concat(counts, ","), table.concat(fish, ","),
      table.concat(raws, ","), ami2 or -1, gmi2 or -1))
    captured = captured + 1
  end
  sleep(5)
end
f:close()
return string.format("captured %d ticks (%d polls) to %s", captured, polls, out_path)
