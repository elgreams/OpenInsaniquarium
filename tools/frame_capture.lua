-- Paired (state, screenshot) frames from the original, for the frame-diff
-- harness (docs/re/framework/frame-capture.md). Read-only: it pauses the
-- process (CE pause()), dumps one state snapshot in the trace format, asks
-- the shell to screenshot the window, then resumes.
--
-- Usage (CE Lua / MCP bridge):
--   FRAME_OUT="/tmp/frames"; FRAME_EVERY=10; FRAME_COUNT=50
--   dofile("<repo>/tools/frame_capture.lua")
-- Needs tools/ce_capture.lua's helpers: it runs it with CAPTURE_TICKS=1
-- per frame to write <out>/NNNN.state.jsonl, and runs
-- tools/orig_shot.sh <out>/NNNN.png through os.execute while paused.

local out = FRAME_OUT or "/tmp/frames"
local every = FRAME_EVERY or 10
local count = FRAME_COUNT or 20
local repo = FRAME_REPO or "/home/three/fsf/aquarium"
os.execute("mkdir -p " .. out)
local app = readInteger(0x5eb680)
local last = readInteger(app + 0x47c)
local done = 0
while done < count do
  local t = readInteger(app + 0x47c)
  if t - last >= every then
    -- Keep the pause short: the window manager dims and overlays a window
    -- that stops answering for ~5 s (muffin check-alive-timeout). So the
    -- screenshot comes first, then the snapshot, then resume.
    pause()
    local tag = string.format("%s/%05d", out, done)
    os.execute(repo .. "/tools/orig_shot.sh " .. tag .. ".png")
    CAPTURE_OUT = tag .. ".state.jsonl"; CAPTURE_TICKS = 1; CAPTURE_GPOS = nil
    -- ce_capture emits the current tick first (its loop starts at
    -- start_tick − 1), so one tick is captured even while paused.
    dofile(repo .. "/tools/ce_capture.lua")
    unpause()
    last = t
    done = done + 1
  end
  sleep(2)
end
return string.format("captured %d frames to %s", done, out)
