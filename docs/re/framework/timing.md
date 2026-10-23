# Timing & main loop

Status: `confirmed`. CE on the live game (Proton, 2026-10-21): `app+0x454 == 28`, `app+0x47c` advanced 36 in a 1 s window (title screen).
Binary: inner game PE extracted from `Insaniquarium.exe` (see `docs/re/framework/drm-wrapper.md`). Addresses are inner-PE VAs (image base `0x400000`).

## Summary

| Fact | Value | Confidence |
|---|---|---|
| Logic tick length (`mFrameTime`) | **28 ms** | confirmed |
| Logic tick rate | **1000/28 ≈ 35.714 Hz** | confirmed (36 updates/s measured) |
| SexyAppBase default `mFrameTime` | 10 ms (overridden by WinFishApp) | static |
| Updates are tick-count based, not dt based | yes: `UpdateApp` takes no delta | static |
| Fractional-tick accumulator | `double`, increments in units of 1.0 tick | static |

**Insaniquarium runs at 28 ms per logic tick, not the SexyApp default of 10 ms.** `WinFishApp::WinFishApp` (ctor @ `0x0054d850`) stores `0x1c` (28) into `SexyAppBase::mFrameTime` (`this+0x454`) at `0x0054da98`, overriding the base ctor's `10` (`0x00478a29`). Every timer, speed, and animation `framedelay` in the game counts in these 28 ms ticks.

## Relevant `SexyAppBase` fields (offsets from `this`)

| Offset | Type | Meaning (our name) |
|---|---|---|
| `+0x454` | int | `frame_time_ms`: 10 in base, 28 in WinFishApp |
| `+0x460` | double | `update_accum`: fractional updates owed to the render loop |
| `+0x468` | double | Accumulated real ms since the last check, capped at 200.0 (`0x005c3c10`) |
| `+0x470` | DWORD | Last `timeGetTime()` sample |
| `+0x47c` | int | `update_count`: total logic ticks executed (incremented in `UpdateApp`) |
| `+0x490` | double | `speed`: playback multiplier, default 1.0; `<= 0` pauses updates |
| `+0x5b0` | byte | VSync / "wait for vblank" mode flag |
| `+0x5b4` | int | Display refresh rate (Hz) used in vsync mode |

`confirmed` (CE).

## Loop structure (`SexyAppBase::Process` @ `0x00486390`, vtable slot 7)

Per call (one Windows message-pump iteration):

1. Compute the update/render ratio:
   - non-vsync: `updates_per_draw = frame_time_ms / speed` (in ms terms), `step = 1.0f`
   - vsync (`+0x5b0`): `updates_per_draw = (1000.0f / refresh) / speed`, `step = 1000.0f / (frame_time_ms * refresh)`
2. Sample `timeGetTime()`; fold the elapsed real time into `+0x468`, capped at 200 ms (`FUN_00486320`). This bounds catch-up after stalls.
3. State machine on `+0x480` (1 = need update, 2 = updated/need draw, 3 = drawn):
   - `+0x460 += step` (minus 1.0 per executed update). While `+0x460 >= 1.0`, it calls `UpdateAppStep` (slot 10 @ `0x00480340`), which calls `UpdateApp` (slot 9).
   - After updates it calls `UpdateFrames`/draw (slot 11) with the fractional remainder (`step` as float). That's the render interpolation hook.
   - When ahead of schedule it `Sleep`s for the remaining ms.
4. Demo playback (`+0x509`) runs a separate fast path: while `update_count < demo_target_tick`, it runs updates back-to-back, yielding to the window only once a second.

`UpdateApp` (slot 9 @ `0x004802d0`, overridden by `WinFishApp` slot 9 @ `0x0054bb30`):
- `update_count += 1` (`+0x47c`)
- calls `WidgetManager::UpdateFrame` (the widget tree's `Update()`, where `Board::Update` lives)
- `WinFishApp` wraps this. It has two compile-time debug toggles (`0x005e9635` slow-mo: 1 update per 4 calls; `0x005e9636` turbo: 100 updates per call). Both bytes are **never written** in the binary, so they're always off in shipped builds. We may expose them as dev options.

Tick length: `traced` (all replays run at one logic tick per update). The catch-up and draw scheduling details are `static`, except the catch-up bursts, which are `confirmed` (they show as torn snapshots in captures).

## Implications for our engine

- The sim ticks at a fixed **28 ms**. Model the tick as an integer counter, never as `f32` seconds.
- Our frontend runs the sim with a classic fixed-step accumulator (Bevy `FixedUpdate` with `Time::<Fixed>::from_duration(28ms)` is suitable), capping catch-up at **200 ms** (≈7 ticks) to match the original's stall behavior.
- Rendering may interpolate, but the original draws the latest state with no interpolation for game objects (TODO confirm in `Board::Draw`). Default to no interpolation for fidelity.
- `speed` (`+0x490`) is a global time-scale knob. TODO: find whether the game ever sets it (possibly the debug key `+`/`-`).

Design notes (no claims about the original).

## Open questions

- Dynamic confirmation: does `update_count` advance at ~35.7/s in-game (CE watch on `app+0x47c`)?
- Are any per-screen update rates different (e.g. Board skipping ticks while paused or in menus)?
- Exact semantics of the `+0x509` demo-playback fast path → `demo-recording.md`.
