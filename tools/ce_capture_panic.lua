-- RE tooling only: ce_capture.lua wrapper that pushes alien HP mid-capture
-- to trigger kind 7's panic dive.
--
-- After the alien spawns and passes warp-in (50 ticks), writes HP to 105.0.
-- The next laser hit (6 damage) brings it to 99 < 100, triggering panic dive.
--
-- Globals expected (same as ce_capture.lua plus ALIEN_HP):
--   CAPTURE_OUT, CAPTURE_TICKS, CAPTURE_GPOS
--   ALIEN_HP  (double, default 105.0)

local hp_target = ALIEN_HP or 105.0
local pushed = false
local alien_appeared_tick = nil

-- Monkey-patch: after ce_capture.lua writes each line, check for the alien.
-- ce_capture.lua's main loop calls snap() then writes; we hook by wrapping
-- the file write.  Instead, we run the capture ourselves by setting a
-- post-tick callback that ce_capture.lua doesn't have.
--
-- Simpler: just dofile ce_capture.lua but override the write to inject the
-- push.  ce_capture.lua opens CAPTURE_OUT and writes lines in a loop.
-- We can't easily hook into that loop.
--
-- Simplest approach: replicate the core capture loop with the push added.
-- But ce_capture.lua is complex.  Instead, modify the alien HP BEFORE the
-- capture starts: wait for the alien to appear, push HP, then capture.

local app_ptr = 0x5EB680
local function rd32(a) return readInteger(a) or 0 end
local function rdptr(a) return rd32(a) end
local function rdf64(a)
  local b = readBytes(a, 8, true)
  if not b then return 0.0 end
  return byteTableToDouble(b)
end
local function wrf64(a, v)
  local b = doubleToByteTable(v)
  writeBytes(a, b)
end

-- Wait for the board to exist and have an alien.
local app = rdptr(app_ptr)
local board = 0
local alien = 0

-- Poll for alien (up to 60 seconds = 12000 polls at 5ms)
for i = 1, 12000 do
  app = rdptr(app_ptr)
  if app ~= 0 then
    board = rdptr(app + 0x730)
    if board ~= 0 then
      -- Check alien list: +0xb8 is the vector, +4 is begin, +8 is end
      local vec_begin = rdptr(board + 0xB8 + 4)
      local vec_end = rdptr(board + 0xB8 + 8)
      if vec_end > vec_begin then
        alien = rdptr(vec_begin)
        if alien ~= 0 then
          -- Alien found; wait 50 ticks (~1.4s) for warp-in to complete
          sleep(1400)
          -- Re-read in case the alien died during warp-in
          vec_begin = rdptr(board + 0xB8 + 4)
          vec_end = rdptr(board + 0xB8 + 8)
          if vec_end > vec_begin then
            alien = rdptr(vec_begin)
            if alien ~= 0 then
              local cur_hp = rdf64(alien + 0x1F0)
              local has_sub = rd32(alien + 0x19C)
              if has_sub == 0 and cur_hp > hp_target then
                wrf64(alien + 0x1F0, hp_target)
                -- Log it
                local f = io.open("/tmp/hp_push.log", "w")
                if f then
                  f:write(string.format("pushed alien 0x%08x hp %.1f -> %.1f at poll %d\n",
                    alien, cur_hp, hp_target, i))
                  f:close()
                end
              end
            end
          end
          break
        end
      end
    end
  end
  sleep(5)
end

-- Now run the normal capture
CAPTURE_OUT = CAPTURE_OUT or "/tmp/capture.jsonl"
CAPTURE_TICKS = CAPTURE_TICKS or 6000
CAPTURE_GPOS = CAPTURE_GPOS or "/tmp/gpos"
dofile("/home/three/fsf/aquarium/tools/ce_capture.lua")
