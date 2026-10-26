# Clyde the jellyfish (`Sexy::OtherTypePet`, pet id 5)

Clyde drifts through the whole tank and picks up any coin his body overlaps. Horizontally he wanders like Stinky (stop / left / right). Vertically he sinks slowly and gives a two-tick upward pulse every 40 ticks, timed to his swim animation. When there is any coin on the board he steers toward the nearest one in both x and y, one nudge every 5 ticks. Unlike Stinky he **keeps collecting during alien waves** and never hides. He is the pet hatched after tank 2-1 (`screens/hatch-screen.md#pet-award-table-adventure-first-play-through`).

Same class, fields and update skeleton as Stinky (`pet-stinky.md`). Only the id-5 differences are listed here. All offsets are raw `this`-relative (Ghidra's `OtherTypePet_data` is based at `this+0x154`).

- Ctor `OtherTypePet::OtherTypePet` @ `0x004eb5c0`
- Update (slot 22) @ `0x004f2e80`, id-5 branch at the top of the movement switch
- Coin chase gate `0x004efe80` → chase `0x004ebc80` → target `0x004e7270`, collect `0x004e7c30` (pickup `0x004ddc40`, as Stinky)
- Extra coin filter for id 5: `0x004e4f90`
- Animation `0x004eb7c0` (id-5 path)
- Draw (slot 26) `0x004f37c0` → `0x004f3590` → `0x004e2db0`

Confidence: `static` throughout. No capture with Clyde exists yet.

## Fields used by id 5

As Stinky's table (`pet-stinky.md#fields`), with these differences:

| Offset | Meaning for Clyde | Initial |
|---|---|---|
| `+0x160` | y: **not** forced (Stinky 370, Rufus 365, id 14 355). It keeps AddPet's random y | `Next() % 520 + 20` |
| `+0x188` | speed divisor | **1.5** |
| `+0x170` | vy: used (Stinky's stays 0) | 0 |
| `+0x1a0` | animation counter, 0..39; also times the upward pulse | 0 |
| `+0x1b4` | shell counter: never changes for id 5 (the shell routine `0x004e2a20` only handles ids 0 and 1), so it stays 0 | 0 |
| `+0x1bc` | reset to 0 on each pickup, never counted up (only Stinky's chase increments it) | 0 |

`static`.

## Construction

`Board::AddPet(5, -1, -1, 0, 0)` (`0x00544a90`) → ctor `0x004eb5c0` → shadow `0x00544800`. App MTRand (`MTRand__NextNoAssert`) draws, in order:

1. AddPet: `x = Next() % 265 + 105`.
2. AddPet: `y = Next() % 520 + 20`. Kept for Clyde. The first update clamps it to [95, 370] (see step 7 below), so a spawn below 370 snaps to the floor.
3. Ctor: `mode = Next() % 10`.
4. Ctor: `+0x1b8 = Next() % 250 + 250` (unused by Clyde).
5. Shadow ctor: `y = 410 + Next() % 5`. The shadow is **type 0** for id 5 (`0x00544800`: id 5 has its own case; other OtherTypePets except Niko get type 1). So it slides with his height like a guppy's: `415 − (max(0, 370 − iy) · 30) / 370` (`sim/board.md#shadows`).

That is **5 app draws**, like Stinky. Divisor `+0x188 = 1.5` (`0x005c3d58`). The ctor has no other id-5 branch; `0x004d89f0` (fixed spot) is Niko-only.

`static`.

## Update (`0x004f2e80`), id 5

Same skeleton as Stinky's update (`pet-stinky.md#update-0x004f2e80-pet-0-normal-game`). Id 5 differs in steps 2–4, 6, 7 and 9. Board `tank` = `board+0x3c8`; `mode` (app) = `app+0x87c`, 5 = Virtual Tank.

1. Return if there is no board or it is paused. `PreUpdate` (`0x004f22c0`).
2. The drop-in sink (`vy += 0.1` while y < 380) is for ids 0, 1, 7, 14 only. **Not Clyde.**
3. Virtual Tank only (`app+0x87c == 5`): the sleepy-flag helper `0x004d6b70` on `+0x1b0`. It clears the flag while he was fed less than 6480 ticks ago; otherwise, while the flag is clear, it sets it when **`Sexy::Rand() % 60 == 0`** (the global `Sexy::Rand`, not the app MTRand). Never runs in Adventure.
4. **Coin chase or wander.**
   - The chase gate `0x004efe80` (id 5): if the coin list `+0xa8` is non-empty **and** the slow flag `+0x1b0` is clear, run the chase `0x004ebc80` (see [Coin chasing](#coin-chasing)) and report "chasing", whatever the chase found. **There is no alien check for id 5** (Stinky's gate refuses while an alien or Bilaterus is present). Otherwise report "not chasing".
   - On **tank 5** the gate is never called: Clyde only wanders there and never collects.
   - When not chasing (or on tank 5): set target vx `+0x180` from the mode: 0 → 0, 1 → −0.5, 2 → +0.5, ≥ 3 → unchanged. (A positive shell counter would also force 0, but it is always 0 for Clyde.)
   - **Then, every tick (chasing or not):** if target < vx, `vx −= 0.1`; then if vx < target, `vx += 0.1`. **There is no snap to the target** (Stinky's step snaps). Both tests run in sequence, so once vx is within 0.1 of the target the two steps cancel (up to float rounding) and vx stays off-target instead of settling on it. While chasing, the target is the stale value from the last non-chasing tick, so this drift keeps pulling the chase's vx back toward it.
5. **Vertical drift**, using the animation counter `+0x1a0` from the previous tick (animation runs at step 9):
   - counter is 2 or 3: `vy −= 0.75` if `y ≥ 240`, else `vy −= 0.3`. So a two-tick upward kick every 40 ticks, stronger in the lower part of the tank.
   - any other value: `vy += 0.03` (slow sinking).

   vy has no cap here; the chase caps its own ±1 nudges at |vy| < 2, and the floor zeroes it (step 7).
6. Counters and reroll, as Stinky: `+0x194 += 1`, `+0x198 += 1`. Reroll if `+0x198 > 20`, or (`x ≤ 10` and target ≤ 0), or (`x ≥ 540` and target ≥ 0): `+0x198 = 0`, **app `Next() % 10`**; if 0, **app `mode = Next() % 3`** (the `% 9` variant is for ids 7 and 14). Then `+0x1ac −= 1` if non-zero; `+0x1a8 += 1` while < 20. The shell routine `0x004e2a20` does nothing for id 5.
7. **Clamp:** x ≤ **550** (ids 0 and 5), x ≥ 10. y > 370 → y = 370 and **vy = 0** (the default floor; ids 1, 7, 14 have their own). y < 95 → y = 95 (vy is **not** zeroed at the top).
8. Edge modes, as Stinky: x > 535 and vx > 0.1 → mode 1; x < 15 and vx < −0.1 → mode 2.
9. **Animation** (`0x004eb7c0`, id 5): `a = (a + 1) % 40` on `+0x1a0`; cel `+0x1a4` = `a / 2` for a < 7 (cels 0..3), 4 for a < 16, `a / 4` otherwise (cels 4..9). No turn counter, no facing update (`+0x178` stays 1.0). If a missile is attached (`+0x98`), its counter steps (`0x004d7020`), as every pet.
10. **Move:** `x += vx / 1.5`, `y += vy / 1.5` (÷ 7.5 with the slow flag).
11. Widget position = `trunc(x), trunc(y)`.

RNG per tick: only step 6 (`% 10`, then `% 3` w.p. 1/10, every 21 ticks or on each edge-bump tick) and the pickup sound (below). All app MTRand.

`static`.

## Coin chasing

`0x004ebc80`, id 5. Runs whenever the gate allows it (step 4).

1. **Target** (`0x004e7270`): exactly Stinky's nearest-coin search (skip collected coins, kinds `0x11`/`0x12`, and kinds 3/10 while a starcatcher exists; distance between `(x + 40, y + 40)` and `(coin.wx + 36, coin.wy + 36)`, integer-truncated components, first strict minimum wins), **plus** the id-5 filter `0x004e4f90`: when the app flag `app+0x880` is set **and** a starcatcher exists (`board+0xd0` non-empty), silver and gold coins (kinds 1, 2) are skipped. `app+0x880` is 0 in normal Adventure (`sim/shop-bar.md`), so this never applies there. Which mode sets it is an open question.
2. **Steering**, only when the counter `+0x194 > 4` **and** a target exists. Then `+0x194 = 0` (it is not reset without a target), and with `cx = x + 40`, `cy = y + 40`:
   - `cx > coin.wx + 36` and `vx > −2`: `vx −= 1`; else `cx < coin.wx + 36` and `vx < 2`: `vx += 1`.
   - `cy > coin.wy + 36` and `vy > −2`: `vy −= 1`; else `cy < coin.wy + 36` and `vy < 2`: `vy += 1`.

   Compared to Stinky's 6-band table: one band, steps of 1, both axes. Between nudges the update's drift (vx toward the stale target by 0.1/tick, vy +0.03 or the pulse) keeps acting.
3. If a target exists, **collect** (`0x004e7c30`, the shared id-0/5 path): over coins `+0xa8` in list order with the same skips (including the id-5 filter), the first coin with `coin.wx + 16 < cx < coin.wx + 56`, `coin.wy + 16 < cy < coin.wy + 56` and kind < 15 is picked up exactly as Stinky does (`pet-stinky.md#coin-chasing` step 3): banked, removed, `+0x1bc = 0`, and the collect sound (silver/gold/star via `0x005383d0`: **1 app draw `Next() & 3` if not throttled**; other kinds play fixed samples). At most one coin per tick. Note the box is tested against the **target-independent** list, so he can pick up a coin other than the one he steers toward.

Because the gate reports "chasing" whenever the coin list is non-empty, the wander target is frozen while any coin (even a collected one flying to the counter, or an unreachable kind) exists.

`static`.

## Effects on other objects

- **Coins:** removes and banks the coin it overlaps (pickup `0x004ddc40` → money `0x004d54e0`, coin removal `Coin` slot 76). Same code as Stinky. Pearls (kind 5) and other kinds < 15 are collected too.
- **Aliens:** none. Aliens target fish only (`alien-sylvester.md`), and Clyde ignores them. He keeps collecting during a wave.
- **Food, fish:** none.

`static`.

## Clicks

Mouse-down (slot 54 @ `0x004f3a70`, the same for every OtherTypePet) first calls `0x004f3a00`. For a non-negative click count (a left click in SexyApp), that returns false and the click is forwarded to the board's mouse-down (board vtable `+0xd8`) at board coordinates, guarded by the re-entrancy byte `0x005e89f8`. So clicking Clyde shoots/feeds through him. For a negative count (right click) it is handled locally: drop-in pets call `0x004f2650(+0x1ac)` and maybe `0x0054c6f0(this)`; others call `0x0053bfb0(widget x + px, widget y + py)`. Not followed further.

`static`.

## Drawing

Draw (slot 26 @ `0x004f37c0`): first `0x004d6ae0` (calls `0x00538060` while the global byte `0x005e89cd` is set; not followed). For id 5 it always calls `0x004f3590(g, mirror = 0)`: **Clyde is never mirrored**. Then `0x004e2db0`, positions relative to the widget's top-left:

1. If the slow flag `+0x1b0` is set: `IMAGE_ZZZ` (`0x005e8b44`) at (50, −27) (the (30, −27) mirrored variant is never used).
2. `IMAGE_CLYDE` (`0x005e8bd0`, `resources.xml`: 10 × 1 cels of 80 × 80) at (0, 0): source `(cel · 80, 0, 80, 80)`, cel = `+0x1a4`. One row only.

Then the shared tail of `0x004f3590`: the drop-in PRESTO flash while the drop-in flag is set and `+0x1a8 < 20` (as Stinky), the missile overlay if `+0x98` is set, and for drop-in pets `0x004f2450(+0x1ac)`. Colorize is reset at the end. Shadow: type 0 (see Construction).

`static`.

## Summary / open questions

- **5 app draws** on AddPet (x, y, mode, `+0x1b8`, shadow). Per tick: the wander reroll and the coin-sound draw, as Stinky.
- Collects coins with no alien check; frozen wander target while any coin exists; vx drift without snap (within 0.1 the two steps cancel; check the float residue against a capture).
- Open: what `app+0x880` is (the silver/gold skip with starcatchers); the right-click path `0x0053bfb0`; `0x005e89cd` / `0x00538060` in draw.
- First capture should be tank 2-2 (the first level he can be brought to).
