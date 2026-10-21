# Fish (guppy): movement, hunger, coin drop

Status: `draft`. Idle swim (modes 0–9, reroll, hunger countdown, no food/coins) **replays bit-exactly for 400 ticks** from a raw capture (`crates/harness/tests/idle_guppies.rs`, `tank1-1_raw_01`). Movement integration, hunger countdown and the reroll timer are **confirmed live** (CE capture `traces/original/tank1-1_live_04.trace.jsonl`, 150 ticks: `x += vx/div` is bit-exact on 149/149 ticks, hunger −1/tick, `+0x1b4` cycles 0..20, div ∈ {2.0, 1.6, 1.8}, coin interval 217 ∈ [150, 349]). Food/eating and coin drop are static only.
Class: `Sexy::Fish` (type code `0x00`), vtable `0x00597d54`. `Fish::Update` = vtable slot 22 @ `0x004f6970` (also used by Oscar, Ultra, Gekko, SylvesterFish, BallFish, BiFish).

**Offset note:** Ghidra's recovered `Fish_data` struct is based at `this+0x154`. The offsets below are **raw `this`-relative** (decompiler `field_0xNN` + `0x154`).

## Fields

| Offset | Type | Meaning (our name) | Notes |
|---|---|---|---|
| `+0x2c` / `+0x30` | int | widget x / y (pixels) | Rounded position used for drawing and spawn points |
| `+0x88` | ptr | app | `[+0x88]+0x730` = Board, `+0x7b0` = app RNG |
| `+0x8c` | int | type code | 0 = guppy |
| `+0x9c` | int | **hunger** | Counts down 1/tick. ≤ 0 = hungry. Clamped at ≥ −1000 (`0x004d6ef0`) |
| `+0xa0`, `+0xa8` | int, byte | eating-animation counter / direction | `0x004d6c50` |
| `+0xa4` | int | hunger "warning" threshold | Hunger == `this+4` triggers a sound/flash (`0x004d6920`) |
| `+0xac` | int | state / dying flag | `< 0` = normal alive state (see `0x004d69d0`) |
| `+0x158` | double | **x** (sub-pixel position) | |
| `+0x160` | double | **y** | |
| `+0x168` | double | **vx** (pixels per move-step; see divisor) | |
| `+0x170` | double | **vy** | |
| `+0x180` | double | **speed divisor** | Position += velocity / divisor each tick. `AddRandomGuppy` sets it to 30.0 |
| `+0x178` | int | facing (+1 / −1) | Used by the "patrol" mode |
| `+0x190` | int | min y (top) | Bounds, set at spawn |
| `+0x194` | int | min x (left) | |
| `+0x198` | int | max y (bottom) | `< 321` enables slow sinking (see step 8) |
| `+0x19c` | int | max x (right) | |
| `+0x1a0` | int | **growth stage** | 0 = small, 1 = medium, 2 = large, 3/4 = king/star |
| `+0x1ac` | int | **swim mode** | 0–9 or 123 (`0x7b`, the "startle"/eat-reaction mode) |
| `+0x1b0` | int | mode timer | Ticks in the current mode; reset at 40 (`0x27 <`) inside the modes |
| `+0x1b4` | int | mode-reroll timer | Increments every tick; reroll check when > 20 |
| `+0x1bc` | int | animation frame (0–5) | Turns, etc. |
| `+0x208` | int | vy-damp timer | While > 0: `vy *= 0.9`, decrement |
| `+0x20c` | int | x-push timer | While > 0: `push *= 0.9`, `x += push`, decrement (`+0x218` double push) |
| `+0x210` | int | misc countdown | Decrements to 0 |
| `+0x220` | int | **coin timer** | +1 per tick when allowed to drop coins |
| `+0x224` | int | coin interval | Overridden to 35 when global flag `0x005e89cc` is set |
| `+0x228` | byte | "is a guppy-like fish" | Gates the hungry messages |
| `+0x229` | byte | flag used by the target-follow branch | |
| `+0x22a` | byte | gold-coin capable | Picks the coin-type table in CoinDrop |

**Numeric types:** position, velocity and divisor are `double` (x87). Bounds and timers are `int`. Comparisons between them convert the int to double.

## Construction (`Fish::Init` @ `0x004de370`, called by every Fish ctor with x, y)

In order, with **app-RNG draws numbered**:

1. `x = (double)x`, `y = (double)y`. Widget pos = `ftol(x), ftol(y)`. Size 80×80. Type 0. `vx = 0.0`, `vy = −0.5`. `+0x188` (facing scale) = 1.0.
2. **R1:** `if Next() & 1 == 0 { vx = −0.1; +0x188 = −1.0 }`
3. Facing (`+0x178`) = 1. Bounds: miny `+0x190` = **370**, maxy `+0x198` = **95**, minx `+0x194` = **10**, maxx `+0x19c` = **540**. (Despite the names, `+0x190`/`+0x198` are the floor and ceiling. Confirm against the clamp code.)
4. **R2:** divisor `+0x180` = `[2.0, 1.6, 1.8][Next() % 3]`
5. **R3:** **hunger `+0x9c` = `Next() % 200 + 400`** (400–599 ticks ≈ 11–17 s until hungry)
6. `+0x1a8` = (Virtual Tank: global `Rand() % 9 + 12`) else **R4:** `Next() % 3 + 4`
7. **R5:** initial swim mode `+0x1ac` = `Next() % 10`. Mode timer `+0x1b0` = 40, reroll timer `+0x1b4` = 0, coin timer `+0x220` = 0.
8. **R6:** **coin interval `+0x224` = `Next() % 200 + 150`** ticks (Virtual Tank: global `Rand() % 200 + 4320`)
9. `+0x228` = level-has-hints flag. `+0x55` = 1 unless aliens are present.

So **constructing a guppy draws 6 app-RNG values (R1–R6)**. `Board::AddRandomGuppy` then draws 4 more: 2 before the ctor (x, y) and 2 after (`+0x1c` = vy-ish `%5+18`, which overwrites, and `+0xb4` = `%10+45`). *TODO: reconcile `AddRandomGuppy`'s `fish+0x1c` / `+0xb4` with the decompiler-relative offsets; they may be `+0x170` / `+0x208`.*

`FUN_00564160` = **`_ftol2`**, i.e. `CVTTSD2SI`, **truncation toward zero**. All double→int position conversions truncate.

## Constants (from `.rdata`)

`0.05, 0.1, 0.2, 0.25, 0.3, 0.5, 0.7, 0.75, 0.8, 0.9, 0.95, 1.0, 1.3, 2.0, 3.0, 4.0, −0.1, −0.5, −1.0, −3.0, −4.0, 5.0, 15.0, 30.0, 40.0, 115.0, 175.0, 240.0, 250.0`. Each is cited below where it's used.

## `Fish::Update` order (per tick)

0. If the Board is missing or the board's pause flag (`board+0x94`) is set: return.
1. **`Fish::PreUpdate`** (`0x004f22c0`). TODO.
2. **Behaviour branch:**
   - **Follow-cursor mode** (global `0x005e89d0` set *and* `+0x229` set *and* stage ≤ 1): steer x/y toward the cursor (`0x005e89d4` / `0x005e89d8`). The deadzone is ±5 px, the snap distance ±50 px, steps are ±5.0 (far) or ±3.0 (near) per tick, and vx/vy are forced to ±0.5 / 0.
   - **Otherwise call `Fish::UpdateHunger`** (slot 83 @ `0x004f15c0`, below). If it returns true (the fish is chasing food), stop here for the behaviour branch. If false, run the **swim mode** state machine on `+0x1ac` (step 3).
3. **Swim modes** (`+0x1b0` is the per-mode timer; most modes act every 40 ticks):
   - **mode 123 (startled):** reset the coin timer and some counters. After 30 ticks, damp vx and vy by 0.95 if |v| > 2.0. After 50 ticks: **RNG A** `mode = Next() % 9 + 1`, then if x is out of bounds (`x > maxx − 20` or `x < minx + 20`) or **global RNG** `Rand() % 3 == 0`: `vx *= −1`. *(This is the only global-RNG use in guppy code.)*
   - **mode 0 (drift down):** if `+0xb4 == 0`, vy = 0.5. Every 40 ticks: move vx 0.5 toward 0 when |vx| > 0.5 (|vx| ≤ 0.5 is left alone), then frame = trunc(|vx|). *(Corrected: an earlier reading had 3.0; the step constant at `0x004f725d` is the 0.5 already on the FPU stack.)*
   - **modes 1 / 2 (cruise):** if `+0xb4 == 0`, vy = −0.5. Every 40 ticks: move vx by 1.0 (mode 1) or toward −1.0 in steps of 1.0 (mode 2).
   - **modes 3 / 4 (rise):** every 40 ticks: nudge vx toward ±1, nudge vy toward 3.0 by 1.0, and increment/decrement the frame toward 4.
   - **modes 0–2, at the end of their branch:** `y −= c / divisor` (c = 0.25 for mode 0, 0.5 for modes 1–2).
   - **modes 5–9 (patrol):** if `+0xb4 == 0`, vy = −0.5, or −0.1 when `y < 115`. Every 40 ticks: accelerate along the patrol direction `+0x178` by 2.0 while vx still points against it, else by 1.0; frame = trunc(|vx|). Then flip: dir +1 and x > 250 → dir −1, vx −= 2.0; dir −1 and x < 175 → dir +1, vx += 2.0.
     - Exact compares: dir +1 adds 2.0 iff vx < 0; dir −1 subtracts 2.0 iff vx > 0 (vx == 0 takes the 1.0 step either way).
   - **Modes 3/4 only** (end of their branch, every tick): if y > 240, set mode 0.
4. **Mode reroll:** `+0x1b0 += 1`, `+0x1b4 += 1`. When `+0x1b4 > 20`: reset it to 0, then **RNG B** `if Next() % 10 == 0 { RNG C: mode = Next() % 9 + 1 }`.
   - *Matches the live trace:* each guppy draws once every 21 ticks (`board.md` → "+2 every 21 ticks" with 2 guppies).
5. **Coin drop** (`Fish::UpdateCoin`, slot 82 @ `0x004f1400`), unless `0x004da780` says the level is over. Details below.
6. **Damping timers:** `+0x208`, `+0x20c`, `+0x210` (see Fields).
7. **Fixed drift by vx bucket.** For exact vx values only: vx == 0 → `y += 1/div`; vx == 1 → `y += 0.75/div`; vx == 2 → `y += 0.5/div`; vx == 3 → `y += 0.25/div`. *(Exact float equality on purpose; reproduce it exactly.)*
8. If max-y < 321: `y −= 0.25`.
9. **Clamp** x to [minx, maxx] and y to ≤ maxy.
10. If `+0xb4 < 1` or vy ≤ 0: clamp y ≥ miny. Otherwise (**bubble emission**, while `+0xb4` ≥ 31):
    - `n = 2` (or 1 if `+0xb4 > 40`). **RNG D** `if Next() % n == 0`, emit bubbles via `Board::AddBubble` (`0x00538ac0`). For a small fish (stage 0): **RNG** `&1`, then 2 draws for the offset (`15 − r%30`). For others: 4–6 draws (offsets `55 − r%60`, `20 − r%40`; stage 6 uses `125 − r%120`, `105 − r%80`).
11. **Edge steering:** near the right edge (x > maxx − 5) with vx > 0.1: `vx −= 0.1`. Near the left (x < 15) with vx < −0.1: `vx += 0.1`.
12. **Slot 89** (`0x004de840`, anim/turn update): swim-frame and turn-frame counters only. The one RNG use is the rainbow-tint path (`+0x1d1` set, global `Rand() % 255 + 1` once, cached in `+0x1d4`); never set for tank 1-1 guppies.
13. **Integrate:** `div = +0x180`, unless byte `+0xf6` is set, in which case div = 0.8 while `+0xf8` > 0, else 0.3. Then `x += vx / div` and `y += vy / div`.
14. Set the widget position (slot 42) to `trunc(x), trunc(y)` (`_ftol2`, truncation toward zero).

### App-RNG call order per guppy per tick (summary)

| When | Draws | Expression |
|---|---|---|
| Mode 123 exit (startle over) | 1 (+ global `Rand()` if in bounds) | `%9 + 1` |
| Every 21st tick (`+0x1b4 > 20`) | 1, plus 1 more w.p. 1/10 | `%10`, then `%9 + 1` |
| Bubble emission (`+0xb4` ≥ 31) | 1 + (1–6) | `%n`, offsets |
| Food chase / hunger | TODO | (`UpdateHunger` sub-calls) |

## Hunger (`Fish::UpdateHunger`, slot 83 @ `0x004f15c0`)

1. Animate the eat counter (`0x004d6c50`).
2. If there are **no aliens** (board lists `+0xb8` and `+0xf4` empty): **hunger −= 1** (`0x004d6ef0`, only when `+0x11c == −1`). When hunger reaches the warning value, play the warning. Clamp hunger at −1000.
3. If the fish is hungry (`0x004d69d0`: state < 0 and hunger < 1, or < −2159 / −4319 in Virtual Tank depending on species) or it's a special fish:
   - hunger == −1 → message "Fish are hungry! Click to drop food!"
   - hunger == −200 → "Fish are REALLY hungry!" and a sound
   - hunger == −400 → "about to die of hunger"
   - **hunger < −499 → die** (slot 89, `this, true`)
4. While hunger < 500 (not full), chase food via slot 85 (`0x004df0b0`). This returns true while chasing, which suppresses the swim modes. **TODO:** food search order and eat radius (`0x004df0b0`, `0x004dee90`).

So a guppy lasts **500 ticks hungry ≈ 14 s** after hunger hits 0 before dying (`< −499`). Starting hunger is set at spawn (TODO: find the value; `AddRandomGuppy` doesn't set it, so it comes from the ctor `0x004de370`).

## Coins (`Fish::UpdateCoin`, slot 82 @ `0x004f1400`)

- Runs only if stage > 0 or state ≥ 0.
- `coin_timer (+0x220) += 1`. The interval is `+0x224` (35 if `0x005e89cc` is set). `0x004d7100`: drop when `timer ≥ interval` (normal modes).
- On a drop: reset the timer. If `0x004d6bd0` allows it, pick the coin type by **growth stage** (gold-capable table when `+0x22a` is set): stage 1→4, 2→6, 4→7, else 2. Spawn via `Board::AddCoin` (`0x00544430`) with vy = −1.0.
- Small guppies (stage 0) don't drop coins.

## Open questions

- Initial hunger, growth thresholds (pellets per stage), starting coin interval: ctor `0x004de370` and the eat handler `0x004dee90`.
- Food targeting (`0x004df0b0`): nearest pellet? first in list? eat radius?
- `Fish::PreUpdate` `0x004f22c0` and slot 90 `0x004d5650` (turn animation).
- ~~Rounding~~: `FUN_00564160` is `_ftol2` (truncation), confirmed.

Partial decompiles from the interrupted research run are in `re/scratch/` (local, gitignored).
