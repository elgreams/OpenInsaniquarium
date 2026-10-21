# Demo recording & playback (SexyApp)

Status: `draft` (static). Code paths are present and wired to the command line. Needs a live test (`-record`, then `-play`) to confirm the output and that it replays cleanly.

## Summary

SexyApp's input recorder is **compiled in and reachable** in this build:
- `-record` → `mRecordingDemoBuffer` (`app+0x508 = 1`): input is written to a bit-packed `Buffer`
- `-play` / `-playnum N` → `mPlayingDemoBuffer` (`app+0x509 = 1`): updates are driven from the file
- `-demofile <path>` sets the file name; `-recnum N` rotates numbered `N.dmo` files (default 5)
- `-version` prints build info (handled by `SexyApp`, slot 45 @ `0x00493cb0`)

Parsing: `SexyAppBase::ParseCmdLine` (slot 44 @ `0x004877d0`, a quote-aware tokenizer) → `SexyApp::HandleCmdLineParam` (slot 45) → `SexyAppBase::HandleCmdLineParam` (slot 46 @ `0x00487c80`).

**Caveat:** the game is launched by the DRM wrapper. TODO: check that the wrapper forwards the command line to the inner app (Steam launch options are the easy way to pass it).

## File format (`.dmo`, little-endian)

Loader: `SexyAppBase::ReadDemoBuffer` @ `0x00479fe0`.

| Field | Type | Notes |
|---|---|---|
| magic | u32 | `0x42BEEF78` |
| version | u32 | `< 2`: legacy flat layout; `>= 2`: has a sync-point section |
| `gMTRand` seed | u32 | Read into `app+0x8`, then `gMTRand.SRand(seed)` (`0x0040f9f0`) |
| product name len | u16 | Clamped to 255 |
| product name | bytes | Must match the app's product name or load fails ("This demo file appears to be for '…'") |
| *(v<2)* demo length | u32 | → `app+0x56c` |
| *(v<2)* bitstream | rest | Event stream |
| *(v≥2)* bitstream size | u32 | |
| *(v≥2)* bitstream | bytes | Event stream |
| *(v≥2)* sync points | rest | Count + entries; used for `DemoJumpTime` seeking |

### Event bitstream

Read with `Buffer::ReadNumBits(n, signed)` (`0x004134c0`); written with `Buffer::WriteNumBits(value, n)` (`0x004130c0`). Each record (`ProcessDemo` @ `0x004836b0`, decode loop; `0x00483630` header):

```
delta_ticks : 4 bits   (u; tick gap since last event — ≥15 is chained via repeated 15+marker)
short_form  : 1 bit
cmd         : 1 bit if short_form else 5 bits
payload     : depends on cmd
```

| cmd | Meaning | Payload |
|---|---|---|
| 0 | Mouse move | long form: x:12, y:12 absolute; short form: dx:6s, dy:6s relative |
| 1 | Activate/focus change | 1 bit (long) / button-down/up + 3 bits (short) |
| 2 | Minimized flag | 1 bit |
| 3 / 4 | Key down / up | keycode: 8 bits |
| 5 | Mouse button | 1 bit count, then (count+1)·8 bits |
| 6 | Close request | — |
| 7 / 8 | Mouse enter / leave | — |
| 9 | Demo end marker | — |
| 0x15 | Char typed | 8 bits signed |
| 0x17 | Fullscreen/vsync toggle | 1 + 8 bits |
| 0x1f | Long-delta continuation | — |

Registry writes are recorded too (cmd `0x0c`, `0x0047de50`). On playback, registry reads come **from the stream instead of the real registry**, which keeps profile-dependent behavior deterministic.

### Playback timing

`SexyAppBase::Process` (`0x00486390`) runs a dedicated loop under `mPlayingDemoBuffer`: it ticks `UpdateApp` back-to-back until `update_count` reaches the next event's tick, applies the event, and yields to the window once a second. **The demo is tick-indexed, not time-indexed**, which is what we want.

## Why this matters for us

1. **Free input capture from the original.** Record a session with `-record`, and we get the exact tick-stamped input sequence. No CE input hooking needed.
2. **Deterministic replay on the original.** The same `.dmo` replays identically on the original (the RNG is seeded from the header, and registry I/O comes from the stream). We can replay a demo while CE samples state each tick, so captures are repeatable.
3. **Cross-check.** Our engine can read `.dmo` files directly: parse the event stream → feed our sim → compare against the CE trace of the original replaying the same file.

**One gap:** the app gameplay RNG (`app+0x7b0`) is seeded from `gMTRand` during `WinFishApp` construction, which happens **before** `ReadDemoBuffer` reseeds `gMTRand` in `Init`. So on playback the app RNG may *not* be reproducible from the demo seed alone. Needs a live test. If it isn't, the harness snapshots `[app+0x7b0]` state at tick 0 via CE (see `rng.md`).
