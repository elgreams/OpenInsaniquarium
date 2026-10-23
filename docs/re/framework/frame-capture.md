# Frame capture (paired state and screenshot)

Status: `confirmed` (tooling). How to get frames from the original that the harness can compare with our renderer: a state snapshot and a screenshot of the same tick.

## How

`tools/frame_capture.lua` (CE Lua, read-only) waits until the logic counter `app+0x47c` has advanced by `FRAME_EVERY` ticks, then:

1. `pause()`: CE suspends the process's threads, so the update counter stops (checked: `0x47c` doesn't move across 600 ms paused).
2. Writes one snapshot with `tools/ce_capture.lua` (`CAPTURE_TICKS = 1`) to `<out>/NNNNN.state.jsonl`. The capture emits the current tick first, so this works while paused.
3. Runs `tools/orig_shot.sh <out>/NNNNN.png`, which activates and raises the 640×480 game window and grabs its client area with `ffmpeg -f x11grab`. A suspended process still has its last presented frame on screen.
4. `unpause()`.

The game must be windowed (`ScreenMode` 0) and not covered by other windows: the screenshot is of the screen region. Files are written by CE, so they're root-owned. Copy them into `traces/frames/<name>/` before use.

## What a frame is

The last frame the game presented before the pause, for the state at tick `T`. SexyApp draws after each batch of updates (`framework/timing.md`), so the picture shows tick `T` unless the draw for `T` hadn't happened yet, in which case it shows `T − 1`. The harness compares against our rendering of `T` and of `T − 1` and keeps the better match.

## Caveats

- **Cursor:** the mouse pointer is in the screenshot if it's over the window (the game draws its own cursor). Park it outside the window, or mask it.
- **Hover:** button hover art depends on the pointer too. Record where it was (the capture doesn't yet).
- **Global-RNG cosmetics:** ambient bubbles and the hatch screen's bubble emitter aren't modelled. Expect differences there; the diff reports them by region.

## The test

`crates/insaniquarium-app/tests/frames.rs` rebuilds each frame's state from its snapshot and renders it on the CPU (`render.rs`):
- the hatch screen from `pres.hatch`;
- the tank via `harness::replay::board_from_capture`, which includes the light bands, the level tick, the message line and dead fish;
- the board dialog from `pres.dialogs`.

It compares the result with the screenshot (`framediff.rs`: a pixel mismatches when a channel is off by more than 40; a 32×32 region fails when more than 5 % of its pixels mismatch). For each frame it writes ours | original | diff to `target/frames/<name>/`. Every frame must match.

Captured sets: `hatch_niko_01` (3 frames) and `tank12_start_01` (10 frames of 1-2: start, alien message, a guppy starving into a `DeadFish`, GAME OVER). Both match pixel for pixel in the compared regions.

What the frame diff found and fixed:
- the hatch-screen Menu button;
- the −2 px button-label offset;
- the backdrop's wave strip, light bands and foreground cut-out;
- the message line;
- the tank label;
- dead fish;
- the dialog minimum height and text positions.
