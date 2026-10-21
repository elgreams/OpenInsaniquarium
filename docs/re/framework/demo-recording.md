# Demo recording & playback (SexyApp)

Status: **recording and file format `confirmed`** (live recording, 2026-10-21, decoded end-to-end by `harness::demo`). Playback: `draft`. The live `-play` attempt exited within seconds and the cause is still unknown (see Open questions).

## Summary

SexyApp's input recorder is compiled in and works in this build:
- `-record`: records input to `popcap.dmo` in the app data directory (under Steam: `C:\ProgramData\Steam\Insaniquarium\`). The file is written on exit.
- `-play` / `-playnum N`: plays a demo back. **The read path differs from the write path**: playback looks for `C:\ProgramData\PopCap Games\Insaniquarium\popcap.dmo`.
- `-demofile <path>` sets the file; `-recnum N` rotates numbered `N.dmo` files.
- `-version` prints build info.

Parsing: `SexyAppBase::ParseCmdLine` (slot 44 @ `0x004877d0`) → `SexyApp::HandleCmdLineParam` (slot 45 @ `0x00493cb0`) → `SexyAppBase::HandleCmdLineParam` (slot 46 @ `0x00487c80`).

**The DRM wrapper drops extra command-line arguments.** It launches the inner game with only `-changedir="…"` (wrapper `FUN_00407ae7` builds the child command line from the exe path plus that one argument). To pass `-record`/`-play`, run the inner PE directly under the same Proton prefix. This is RE tooling only; see `docs/re/dynamic-analysis.md`.

## File format (`.dmo`, little-endian)

Writer: `SexyAppBase::WriteDemoBuffer` @ `0x0047a520`. Reader: `SexyAppBase::ReadDemoBuffer` @ `0x00479fe0`.

| Field | Type | Notes |
|---|---|---|
| magic | u32 | `0x42BEEF78` |
| version | u32 | `2` in this build. `< 2`: legacy layout with no sync section |
| `gMTRand` seed | u32 | Stored at `app+0x8`. On load: `gMTRand.SRand(seed)` |
| version-string length | u16 | Clamped to 255 on read |
| version string | bytes | Product version (`"1.1"`, from `app+0x39c`). Must match or loading fails ("This demo file appears to be for '…'") |
| *(v≥2)* sync section size | u32 | Byte length of the next section |
| *(v≥2)* sync section | bytes | `Buffer`: `u32 count`, then per entry `string` name (`u16 len` + bytes) + `u32` value. Marker list used for `DemoJumpTime` seeking (entries come from the list at `app+0x5a0`). Empty in our recording (`00 00 00 00`) |
| total ticks | u32 | `app+0x484` (update count) at the end of recording. Our recording: 11420 ticks ≈ 320 s |
| bitstream | bytes | Event stream, **no length prefix**: runs to end of file |

Our first recording (`traces/demos/tank1-1_scripted_01.dmo`, gitignored): 4387 bytes, seed `0x08F9A83A`, 11420 ticks.

### Event bitstream

`Buffer::WriteNumBits(value, n)` (`0x004130c0`) packs **LSB-first**: bit *i* of the value goes to the next free bit, and bits fill each byte from bit 0 upward. `ReadNumBits(n, signed)` (`0x004134c0`) mirrors it, and `signed` sign-extends from bit *n-1*.

Each event starts with a **tick header** (`SexyAppBase::WriteDemoTimingHeader` @ `0x004835a0`):

```
while (now - last) > 15:
    bits(15, 4); last += 15
    bits(0, 1); bits(0x1F, 5)          # "long" cmd 0x1F = padding/no-op
bits(now - last, 4); last = now       # 0..15 ticks since the previous event
```

So the first header of a real event is `delta:4` (0–15). Larger gaps are bridged with no-op records (cmd `0x1F`).

Then the **command** (`ProcessDemo` @ `0x004836b0`, reader header @ `0x00483630`):

```
short : 1 bit
cmd   : 1 bit if short else 5 bits
```

Multi-byte payloads are written with `WriteByte` (`0x00413020`), which places 8 bits LSB-first at the current bit position (unaligned). `u32` = 4 such bytes, little-endian. `blob` = `u32 len` + `len` bytes. `WriteBoolean` (`0x004131c0`) writes a **whole byte**, not a bit.

| Form | cmd | Meaning | Payload |
|---|---|---|---|
| short | 0 | Mouse move (relative) | dx: 6 bits signed, dy: 6 bits signed. Used when both deltas are in −32..31 |
| short | 1 | Mouse button | down: 1 bit, button: 3 bits signed (1=left, 2=right, 3=middle, −1/−2 = left/right double-click). Position is the last mouse position |
| long | 0x00 | Mouse move (absolute) | x: 12 bits, y: 12 bits |
| long | 0x01 | Activate/focus | 1 bit |
| long | 0x02 | Minimized | bool (8 bits) |
| long | 0x03 | Key down | keycode: 8 bits |
| long | 0x04 | Key up | keycode: 8 bits |
| long | 0x05 | Char typed | is-wide: 1 bit, then 8 or 16 bits |
| long | 0x06 | Close request | — |
| long | 0x07 / 0x08 | Mouse enter / leave | — |
| long | 0x09 | Demo end marker (written by `UpdateAppStep` when recording stops) | — |
| long | 0x0B | Registry read (`0x0047e8b0`) | ok: 1 bit; if ok: `u32` type, `blob` value |
| long | 0x0C | Registry write (`0x0047de50`) | ok: 1 bit |
| long | 0x0E | Registry key query (`0x0047fb10`) | ok: 1 bit |
| long | 0x0F | Data file read (`0x0047f5d0`) | ok: 1 bit; if ok: `blob` contents (e.g. the 589-byte properties XML) |
| long | 0x10 | Data file write (`0x0047f310`) | ok: 1 bit |
| long | 0x11 | HTTP transfer result (`0x0040e460`) | `u32` id, result: byte, `blob` body |
| long | 0x12 | Buffer replay (`0x0047a750`) | `blob` |
| long | 0x15 | Mouse wheel | 8 bits signed (wheel delta / 120) |
| long | 0x17 | Display mode (`0x00489a80`) | windowed: bool (8 bits), refresh: byte |
| long | 0x1F | No-op (tick padding) | — |

Verified against the recording: the decoded stream covers 34862 of 34864 bits, the final event lands on tick 11420 (= header total), and it contains 190 absolute moves plus 380 button events (190 clicks), matching the script. So **all game I/O (registry, profile/save files, properties) is in the stream**, and playback is hermetic.

The short-form mouse button `button` encoding comes from the window message (`0x201`…`0x208`): LBUTTONDOWN→(1, down), LBUTTONUP→(1, up), LBUTTONDBLCLK→(−1, down), RBUTTONDOWN→(2, down), RBUTTONUP→(2, up), RBUTTONDBLCLK→(−2, down), MBUTTONDOWN→(3, down), MBUTTONUP→(3, up).

### Playback timing

`SexyAppBase::Process` (`0x00486390`) has a dedicated path under `mPlayingDemoBuffer` (`app+0x509`). It runs `UpdateApp` until `update_count` (`app+0x484`) reaches the next event's tick, applies the event, and yields to the window once a second. **Demos are indexed by tick, not by time**, which is exactly what our harness needs.

## Why this matters

1. **Free input capture from the original.** We record with `-record` and get the exact tick-stamped input sequence with no CE input hooking.
2. **Our engine can read `.dmo`** (`harness::demo`), so the same input stream drives both the original and our sim.
3. Registry reads, save/profile file reads and HTTP results are all recorded in the stream, so replay doesn't depend on the machine's state.

**Remaining gap:** the app gameplay RNG (`app+0x7b0`) is seeded from `gMTRand` during `WinFishApp` construction, which happens *before* `ReadDemoBuffer` reseeds `gMTRand`. On playback the app RNG may therefore not be reproducible from the demo seed alone. If not, the harness also snapshots `[app+0x7b0]` at tick 0 (see `rng.md`).

## Open questions

- **Live `-play` exits after ~2 s with no crash log** (the first attempt reported "Demo file not found" until the file was placed at the PopCap path). Suspects: the version-string check, or something in the replayed I/O (the stream contains the original save-file contents, so the on-disk saves shouldn't matter). The saves were restored to their pre-record state before playback. Next step: break on `ReadDemoBuffer` failure paths, or log `app+0x509` via CE.
- Exact payload of the long-form Activate (cmd 1) in playback vs. recording.
