# harness: differential testing against the original

The proof of fidelity: same starting state and same inputs give the same state, tick by tick, in the original and in `insaniquarium-sim`.

```
original game ──CE per-tick capture (tools/ce_capture.lua)──▶ trace.jsonl
                                                                   │
            harness::replay: rebuild Board from the first          │
            snapshot's raw object bytes + RNG states, infer        │
            clicks from object appearances, step the sim           ▼
                         sim trace ◀──────── diff ────────▶ decoded capture
```

The original's `-play` is broken in this build (`docs/re/framework/demo-recording.md`), so demos can't drive the original. Instead the harness **captures** the original while it's played (by hand or by a scripted clicker) and **infers** the inputs from the captured state. The `.dmo` parser (`harness::demo`) is kept for reading recorded sessions.

## Inputs: SexyApp demo files (`.dmo`)

Recorded by the original with `-record` (see `tools/run_inner_proton.sh` and `docs/re/framework/demo-recording.md`). `harness::demo::Demo::parse` reads them. A demo carries every input event *and* every I/O result (registry, save files, properties) stamped with its logic tick, so a replay doesn't depend on the machine's state.

## Traces: format v1

A trace is a **JSON Lines** file (`*.trace.jsonl`). The first line is a header and each following line is one tick's snapshot. JSON Lines keeps traces diffable with standard tools and lets captures stream to disk.

### Header (line 1)

```json
{"format":"insaniquarium-trace","version":1,
 "source":"original"|"sim",
 "build":"f54c2c6e…" ,
 "demo":"tank1-1_scripted_01.dmo",
 "start_tick":1234,
 "rng":{"app":{"seed":2146374468}|{"mt":[…624…],"mti":17},
        "global":{"mt":[…],"mti":6}},
 "fields":["board.money","fish[].x", …]}
```

- `build`: SHA-256 of the original exe the trace came from (or the sim's git revision).
- `start_tick`: the `update_count` (`app+0x484`) of the first snapshot.
- `rng`: full generator state at `start_tick`. If `mti == 624` and the state equals `SRand(mt[0])`, `{"seed": mt[0]}` is enough (see `docs/re/framework/rng.md`).
- `fields`: which fields this trace captured. The diff only compares fields both sides captured.

### Snapshot (lines 2…), as written by `ce_capture.lua`

```json
{"tick":1234,
 "board":{"money":200,"clock":100654,"tank":1,"level":1,"paused":false,"samples":"<hex 64×i32>"},
 "counts":{"guppy":2,"food":1,"shadow":2},
 "guppies":[{"x":312.5,"y":181.0,"vx":1.25,…,"raw":"<hex of 0x230 object bytes>"}],
 "raw":{"food":["<hex 0x200>"],"coin":["<hex 0x200>"]},
 "rng_mti":{"app":17,"global":6}}
```

- `tick`: the app update counter (`app+0x484`) after the update completed. The capture polls faster than the 28 ms tick and records each new tick once.
- `raw`: full object bytes, so the harness decodes fields itself (`harness::replay::{fish,food,coin}_from_raw`) and new fields need no re-capture.
- `rng_mti`: the generators' `mti` index. It's a cheap strong check: an extra or missing draw shows up before any visible state differs. The sim side compares only `app` (see `docs/re/sim/board.md`: the global stream is consumed by unmodelled cosmetic bubbles).
- Floats are written with `%.17g` and parsed with serde_json's `float_roundtrip`, so doubles survive bit-exactly.

Versioning: any incompatible change bumps `version`. Readers reject versions they don't know.

## Diff

`diff(original, sim)` walks both traces in lockstep and reports the **first** divergence as `(tick, path, original_value, sim_value)`, e.g. `tick 1517 objects[fish#2].x: 311.75 != 311.5`. Floats compare exactly by default (we're aiming for bit-exact), and a per-field tolerance can be configured while a system is still converging.

## Capture runbook

1. Launch the inner game: `tools/run_inner_proton.sh` (if it exits silently, kill the stale prefix: `WINEPREFIX=<compatdata>/3320/pfx "<Proton>/files/bin/wineserver" -k`).
2. Start CE as root (`docs/re/dynamic-analysis.md`), dismiss the "Proton target" and "Patreon or Trial" dialogs, and attach to `game.exe`.
3. Get the board into the state you want, then from the CE bridge:
   `CAPTURE_OUT="/tmp/x.jsonl"; CAPTURE_TICKS=5000; CAPTURE_GPOS="/tmp/gpos"; dofile("tools/ce_capture.lua")`.
   The MCP call times out after 30 s but the capture keeps running.
4. Optional scripted play: with `CAPTURE_GPOS` set, the capture publishes `gx gy paused` for the first guppy, plus one `x y` line per uncollected coin. A shell clicker (xdotool) can then feed, collect, and click Resume (312, 258) when paused.
5. Copy to `traces/original/tank1-1_<what>_NN.trace.jsonl`. `crates/harness/tests/tank1_1.rs` picks up `_raw_`, `_feed_` and `_coin_` captures.

Replay details (`harness::replay`):
- The board is rebuilt from the **first** snapshot (paused lead-in skipped) and replayed up to the first pause.
- Clicks are inferred from new pellets (`anim` 1 means the click came before the tick, 0 means after it) and from coins turning collected. A click that lands after an update but before the capture read is "mid-frame": it's applied at the next tick and that snapshot is masked.

## Presentation (sound, music, dialogs, screens)

State replays don't see what the player hears or which screen is up, so presentation is compared separately (`harness::presentation`, spec `docs/re/framework/presentation-capture.md`).

- **Sounds, from any capture:** the board's per-sample cooldown table (`board.samples`) changes exactly when a throttled sample starts. `decode_capture` adds `sounds` (cooldown slots stamped that tick) to each snapshot. The sim side records the same slots (`Sound::throttle_slot`), so every replay test compares sounds tick by tick.
  - A mid-frame click's sounds move with the click to the next tick (`Inferred::clean`).
  - After a capture gap the snapshot holds the gap's sounds, so they're compared under `sounds_gap`, which only the capture side carries (skipped).
  - On effect-shot ticks the RNG-chosen shot-sound variants (ZZAM, RICOCHET) are dropped on both sides (`without_deviation_sounds` / `mask_rng_sounds`). The stale-field draw can come before them.
  - Unthrottled sounds (splash, arrival roar, hatch) aren't in the table.
- **Music, dialogs, screens:** `ce_capture.lua` writes a `pres` block per snapshot: the music interface's tracks (volume, fade, playing), the requested track/order, open dialog ids, the screen widgets, and the hatch counter. It keeps capturing without a board. `presentation::capture_records` and `sim_record` turn both sides into the same shape. Our music is the sim's `MusicState`, so it's deterministic and comparable.

## Files

Traces and demos captured from the original live in `traces/` (gitignored). Tests that need them **skip when absent**.
