# harness: differential testing against the original

The proof of fidelity: same starting state and same inputs give the same state, tick by tick, in the original and in `insaniquarium-sim`.

```
original game  ──(-record)──▶  .dmo (inputs + I/O, tick-indexed)
      │                              │
      │ CE per-tick capture          │ harness::demo
      ▼                              ▼
  trace.jsonl  ◀────── diff ───────  our sim replaying the same inputs
```

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

### Snapshot (lines 2…)

```json
{"tick":1234,"rng_draws":{"app":3,"global":0},
 "board":{"money":200,"food_count":1,"alien_timer":2970},
 "objects":[{"list":"fish","idx":0,"type":0,"x":312.5,"y":181.0,"vx":1.25,"vy":-0.5,"hunger":412,"size":0},
            {"list":"coin","idx":0,"type":1,"x":300.0,"y":250.0}]}
```

- `tick`: logic tick *after* the update completed.
- `rng_draws`: how many values each generator produced during this tick. This is the cheapest strong check: a divergence in RNG call count shows up before any visible state differs.
- `objects`: grouped by Board list in **update order** (see `docs/re/sim/board.md`), insertion order within each list.
- Floats are written with full round-trip precision. Values captured from `double` fields keep all 64 bits, and `float` (f32) fields are widened exactly.

Versioning: any incompatible change bumps `version`. Readers reject versions they don't know.

## Diff

`diff(original, sim)` walks both traces in lockstep and reports the **first** divergence as `(tick, path, original_value, sim_value)`, e.g. `tick 1517 objects[fish#2].x: 311.75 != 311.5`. Floats compare exactly by default (we're aiming for bit-exact), and a per-field tolerance can be configured while a system is still converging.

## Files

Traces and demos captured from the original live in `traces/` (gitignored). Tests that need them **skip when absent**.
