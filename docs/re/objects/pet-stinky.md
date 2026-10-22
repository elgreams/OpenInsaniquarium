# Stinky the snail (`Sexy::OtherTypePet`, pet id 0)

Status: `draft`, **static only** (nothing verified live yet).
Class: `Sexy::OtherTypePet` (type code `0x14`), vtable `0x00596d74`, object size `0x1d0`. The same class also implements pets 1, 5, 7 and 14. Only pet 0 is described in detail here; branches for other ids are noted only where they share code.

- `OtherTypePet::OtherTypePet` @ `0x004eb5c0`
- `Update` (slot 22) @ `0x004f2e80`
- Draw (slot 26) @ `0x004f37c0`
- Mouse-down (slot 54) @ `0x004f3a70`

**Offset note:** Ghidra's `OtherTypePet_data` struct is based at `this+0x154`. All offsets below are **raw `this`-relative**.

## Fields

| Offset | Type | Meaning (our name) | Initial value |
|---|---|---|---|
| `+0x30`, `+0x34` | int | widget x / y = `trunc(x)`, `trunc(y)` | from x, y |
| `+0x38`, `+0x3c` | int | widget size | 80 × 80 |
| `+0x8c` | int | type code | `0x14` |
| `+0x158` | double | **x** | ctor arg 1 |
| `+0x160` | double | **y** | ctor arg 2, **forced to 370.0 for pet 0** (unless the "drop-in" flag is set) |
| `+0x168` | double | **vx** | 0 |
| `+0x170` | double | **vy** | 0 |
| `+0x178` | double | facing (last non-zero vx sign source) | 1.0 |
| `+0x180` | double | **target vx** | 0 |
| `+0x188` | double | speed divisor | **1.2 for pet 0** (2.0 default, 1.5 for pet 5) |
| `+0x190` | int | **wander mode** (0 stop, 1 left, 2 right) | `Next() % 10` (see note) |
| `+0x194` | int | free-running counter (+1/tick) | 40 |
| `+0x198` | int | mode-reroll timer | 0 |
| `+0x19c` | int | turn-animation counter (±20 → 0) | 0 |
| `+0x1a0` | int | animation counter | 0 |
| `+0x1a4` | int | **cel** | 0 |
| `+0x1a8` | int | counter, +1/tick up to 20 | 0 |
| `+0x1ac` | int | countdown (−1/tick to 0) | 360 if drop-in flag and id ≠ 0x13, else 0; 0 in Virtual Tank |
| `+0x1b0` | byte | "slow" flag (movement divisor ×5) | 0 |
| `+0x1b4` | int | **shell counter** (0..9; >0 = hiding) | 0 |
| `+0x1b8` | int | ? | `Next() % 250 + 250` |
| `+0x1bc` | int | ? | 0 |
| `+0x1c0` | byte | ? | 0 |
| `+0x1c4` | int | **pet id** | 0 |
| `+0x1c8` | byte | "drop-in" flag (AddPet flagA) | arg |

TODO: `+0x1b8`, `+0x1bc`, `+0x1c0` for pet 0.

## Construction

### `Board::AddPet(id, x, y, flagA, flagB)` @ `0x00544a90`

1. If id == `0x13`, force flagA = 1.
2. If `x == -1`: **app RNG draw 1**: `x = Next() % 265 + 105`; **draw 2**: `y = Next() % 520 + 20`. (The ranges look swapped relative to the tank, x 105..369 and y 20..539. For Stinky the y is overwritten anyway; see below.) `static`
3. Ids 0, 1, 5, 7, 14 → `new OtherTypePet(x, y, id, board[+1000], flagA)`; others → `FishTypePet`.
4. Virtual Tank-only (`app+0x87c == 5` and !flagB): sets `+0x24 = (flagA ? 0x13 : id) + 1000`.
5. `AddGameObject` (joins list `+0xb0`, `++board[+0x144 + 4*id]`), `wm->AddWidget`, shadow creation `0x00544800`, `RebuildZOrder`.

### Constructor `0x004eb5c0`

In order:
1. `GameObject` base ctor, type `0x14`, id/flag stored.
2. `x = arg1`, `y = arg2`. Pet 0 without the drop-in flag: **`y = 370.0`** (pet 14: 355, pet 7: 365).
3. `0x004d89f0(board[+1000])`: only for pet 1 (fixed spawn positions); no-op for pet 0.
4. Widget pos = `trunc(x), trunc(y)`; vx, vy, target vx = 0; size 80×80; facing = 1.0; divisor = 1.2.
5. **App RNG draw 3**: `mode = Next() % 10`.
6. **App RNG draw 4**: `+0x1b8 = Next() % 250 + 250`.

### Shadow (`0x00544800`)

**App RNG draw 5**: shadow y = `410 + Next() % 5` (see board.md / fish-guppy.md).

So `AddPet(0, -1, -1, 0, 0)` costs **5 app-RNG draws**: x, (discarded) y, mode, `+0x1b8`, shadow.

### Update order

`+0xb0` OtherTypePet comes after aliens (pass 1) and before FishTypePet, missiles, shots and coins in `RebuildZOrder`. So in tank 1-2 Stinky updates **after all guppies and food and before the `+0xa8` coins** — he sees coin positions from the previous tick.

## `Update` (`0x004f2e80`), pet 0, normal game

Field names: x `+0x158`, y `+0x160`, vx `+0x168`, vy `+0x170`, facing `+0x178`, target vx `+0x180`, divisor `+0x188` (1.2), mode `+0x190`, counter `+0x194`, reroll timer `+0x198`, turn `+0x19c`, anim `+0x1a0`, cel `+0x1a4`, `+0x1a8`, `+0x1ac`, slow `+0x1b0`, shell `+0x1b4`.

1. Return if the board is paused. Run `PreUpdate`.
2. (Drop-in flag and y < 380: vy += 0.1. Not the case at level start.)
3. **Coin chase** (`0x004efe80`). For pet 0 it runs only when the coin list `+0xa8` is non-empty, **no aliens** are present, and the slow flag `+0x1b0` is clear. See [Coin chasing](#coin-chasing). In that case it **always reports "chasing"**, even if no coin is eligible (e.g. all are collected and flying to the counter), so step 4 is skipped and vx keeps its value. Confirmed by `tank1-2_partial`.
4. **Wander:** set target vx from the mode: 0 → 0, 1 → −0.5, 2 → +0.5, ≥ 3 → unchanged. Then step vx toward the target by 0.1 per tick, snapping to the target if it would overshoot.
5. Reroll and timers:
   - `+0x194 += 1`; `+0x198 += 1`.
   - If `+0x198 > 20`, or (`x ≤ 10` and target vx ≤ 0), or (`x ≥ 540` and target vx > 0): `+0x198 = 0`, then **app RNG `Next() % 10`**. If that is 0, **app RNG `mode = Next() % 3`**.
   - `+0x1ac −= 1` if non-zero; `+0x1a8 += 1` while < 20.
6. **Shell** (`0x004e2a20`, pet 0): while an alien is present, shell `+0x1b4` counts up to 9 and `+0x1bc = 0`; otherwise it counts down to 0. No RNG.
7. **Clamp:** x ∈ [10, 550] (pet 0's right bound is 550); y ≤ 370 (sets vy = 0); y ≥ 95.
8. Edge modes: if x > 535 and vx > 0.1: mode = 1 (head left). If x < 15 and vx < −0.1: mode = 2 (head right).
9. **Animation** (`0x004eb7c0`, pet 0 path). See [Animation](#animation).
10. **Move:** `x += vx / div`, `y += vy / div`. With the slow flag the divisor is `div × 5`.
11. Widget position = `trunc(x), trunc(y)`.

## Coin chasing

`0x004ebc80`, gated by step 3:

1. **Target** (`0x004e7270`): over coins `+0xa8` in list order, skip:
   - collected coins (`+0x198`)
   - kinds `0x11` and `0x12`
   - kinds 3 and 10 (stars), unless no starcatcher exists (`+0xd0` empty)

   For each remaining coin: `dx = trunc((coin.wx + 36) − (x + 40))`, `dy = trunc((coin.wy + 36) − (y + 40))`. Keep the first coin with the strictly smallest `dx² + dy²`.
2. Steering only happens when the counter `+0x194 > 4`. Then the counter is reset to 0 and, with `cx = x + 40` and `w = coin.wx` (int), the first matching rule applies:

   | Condition | Change to vx |
   |---|---|
   | `cx > w + 48` and `vx > −2.3` | −1.0 |
   | `cx < w + 24` and `vx < 2.3` | +1.0 |
   | `cx > w + 40` and `vx > −1.3` | −0.5 |
   | `cx < w + 32` and `vx < 1.3` | +0.5 |
   | `cx > w + 36` and `vx > −0.3` | set to 0 |
   | `cx < w + 36` and `vx < 0.3` | set to 0 |

   (The comparisons nest like a binary search; this table reproduces them. When `cx` is within `[w+32, w+40]`, vx goes to 0 once it's small.)
3. If there is a target, **collect** (`0x004e7c30`): for coins `+0xa8` in list order with the same skip rules, the first coin with `w + 16 < cx < w + 56` and `wy + 16 < cy < wy + 56` (`cy = y + 40`) and kind < 15 is picked up. Pickup (`0x004ddc40`):
   - **bank it** (`0x004d54e0`: money += value),
   - remove the coin,
   - set `+0x1bc = 0`,
   - play the collect sound: silver/gold/star use `0x005383d0`, which makes **1 app draw `Next() & 3` if not throttled**, exactly as a player click does (`coin.md`).

   Only one coin is collected per tick.
4. (The return value is ignored for pet 0; see Update step 3.)

## Animation

`0x004eb7c0`, pet 0. No RNG.

1. Turn: facing > 0 and vx < 0 → turn = 20; facing < 0 and vx > 0 → turn = −20. Then the turn counter steps 1 toward 0.
2. Turn 0:
   - |vx| ≥ 0.3: `anim = (anim + 1) % 20`, cel = anim / 2.
   - Otherwise: `anim = (anim + 1) % 40`, cel = anim / 4.
3. Turn > 0: cel = 9 − turn / 2. Turn < 0: cel = turn / 2 + 9.
4. If shell > 0: cel = shell (hiding).
5. If vx ≠ facing, vx ≠ 0, facing ≠ 0 and shell == 0: facing = vx.

## Interactions

- **Aliens:** while any alien is present, Stinky stops chasing coins and hides in his shell (cel = shell counter 1..9). Aliens target fish only (`alien-sylvester.md#31`), so Stinky can't be eaten.
- **Clicks** (slot 54 @ `0x004f3a70`): TODO. Probably a click-through to the board.
- **Food:** none.

## Summary / open questions

- Stinky costs **5 app draws** when added (2 position, 2 ctor, 1 shadow). After that he draws every 21 ticks while wandering (`% 10`, then `% 3` w.p. 1/10), on every edge bump, and when collecting coins (the points sound).
- TODO: confirm with a tank 1-2 capture.
