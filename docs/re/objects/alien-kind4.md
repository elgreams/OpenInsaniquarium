# Alien kind 4: Gus, the "new breed" (`Sexy::Alien`, type `0x16`, kind 4)

Status: `draft (static)`. Nothing here has been captured yet. Every claim comes from reading the binary, and every section is `static` until a 2-3 recording replays (see "Recording" at the end). Addresses are inner-PE VAs. Offsets are raw `this`-relative, the same as in `alien-sylvester.md`.

Kind 4 is the tank 2-3 alien. The 2-3 dialog says "A new breed of alien is fast approaching! Lasers can't hurt this baddie, so find another way to defeat it!" (`framework/dialog.md`). Its sprite sheet is `IMAGE_GUS` (§7.1), so this spec calls it **Gus**. It is the same `Alien` class as Sylvester, with the same update (`0x004fd560`), ctor (`0x004ecfa0`), death (`0x004f9c70`), prey search (`0x004e8b60`) and steering/eating (`0x004ed460` → `0x004e8f70`). **This spec only describes what differs for kind 4.** Anything not mentioned here behaves as in `alien-sylvester.md`. Notation (`rand(n)`, `grand()`, `ix, iy`, tick) is the same as there.

In short: clicks don't shoot Gus. While he is the first alien, a click in the tank **drops a free pellet** instead. Gus chases and **eats food**, and each pellet costs him health according to its quality. When his health reaches 0 he **explodes** (bigger debris, no corpse) and leaves the usual diamond. After his 100-tick eat delay he also eats fish, and each fish costs him 15 HP.

Confidence: `static` for the whole file (per section below).

## 1. Wave timing

Differences from `alien-sylvester.md` §1:

- **Level table** (`0x00541200`, tank 2 branch, `board+0x3cc` switch): **2-3 sets kind `+0x2bc = 4`** and `+0x340 = 5000`. It doesn't write `+0x2c0`, so the countdown keeps the level-start value **3000** (`0x00541a20`). **2-5** sets `+0x340 = 10000` and `kind = (rand() & 1) | 4` (1 app draw at level setup: kind 4 or 5). This matches `sim/shop-bar.md`'s table.
- **Re-roll after a spawn** (`Board::Update`, the block after `AddAlien`): on tank 2 only level 5 re-rolls. If the kind is 9, `kind = (~r & 1) | 4`. Then `r % 10 == 0` swaps 4 ↔ 5; otherwise a further `r % 20 == 0` sets kind 9. That is 2 or 3 app draws per 2-5 spawn. **2-3 draws nothing** and keeps kind 4.
- **Warning line** (countdown 275): "ALIEN SIGNATURE TYPE-G DETECTED" (`sim/board.md#the-alien-warning-line`). Colours, timer, sound `0x106`, music and the 4 position draws are the same as kind 1.
- **Hint dialog** (countdown 276, `0x005485f6`): on 2-3, unless the game is completed, dialog 0x0e "WARNING!" with the text above is shown **once** (board byte `+0x4c1`, `framework/dialog.md`).
- **Music**: `PlayAlienMusic` (`0x0054c480`) has no kind check, so it is the same as kind 1 (`Alien.mo3`, the same fades).

`static`.

## 2. Spawn (`Board::AddAlien` @ `0x00545620`)

The same steps and the same **3 app draws** as kind 1 (ctor 2, shadow 1). The only difference is the arrival sample at the end. Kind 4 plays **`SOUND_GUFFAW`** (sample `0x123`, global `0x005e8e98`, image-table slot `0x5e0e9c`), played directly through the app's sound manager (vtable `+0xd8`), unthrottled, at `0x0054587b`. Kinds 1–3 play `SOUND_ROAR` instead.

`static`.

### Constructor (`0x004ecfa0`), kind 4

- **Divisor `+0x1b8` = 1.6** (`0x596730`), **health `+0x1f0` = max `+0x1f8` = 100.0** (`0x5c3bd0`).
- Everything else is the same as kind 1: 160×160, eat delay `+0x1a0 = 100`, `+0x200 = 0`, `+0x158 = 15`, `+0x1d4 = 40`, `+0x1d8 = 20`.
- RNG: the same 2 app draws in the same order (`rand()` direction → vx ±3.0, then `rand(10)` wander mode).
- Virtual Tank only (`app+0x880`): health = `grand() % 50 + 70` (the kind-4 arm at `0x004ed2c6`), and `+0x1a8 = 144`. Not reached in Adventure.

`static`.

## 3. `Alien::Update` (`0x004fd560`), kind 4

The top-level steps (warp-in, chase or wander, clamp `x ∈ [−10, 490]`, `y ∈ [85, 290]`, move, timers, resize, animation, death check) have **no kind-4 branch**. The move divides by 1.6, so a 1.8 cruise speed is 1.125 px/tick. The differences are all in the sub-routines below.

Death check (step 10): health ≤ 0 → `0x004f9c70(1)` (§4.3). Gus only loses health while eating (§3.2), which happens inside step 3 of the same update. So **he dies at the end of the tick in which he eats the last pellet** (or fish).

`static`.

### 3.0 Chase or wander (`0x004f03d0`)

For kind 1, the alien chases only when "has prey" (`0x004d4580`: guppies, Oscars, Starcatchers or some pets exist, or a boss is set). **Kind 4 also chases while the food list `board+0xac` is non-empty**, even with no fish in the tank. In both cases it calls `NearestPrey` (§3.1). A target means steer (§3.2) and "chasing". No target means wander (unchanged, §3.3 of the kind-1 spec).

Note that `0x004ed460` calls `NearestPrey` a second time. It uses no RNG and gives the same result, but it re-writes byte `+0x190`.

`static`.

### 3.1 Nearest prey (`0x004e8b60`), kind 4

1. `+0x190 = 0` (the "target is food" flag); best = 100,000,000; target = none. The centre is `(ix + 80, iy + 80)`.
2. **Food first** (kind 4 only): walk `board+0xac` in **list order**. A pellet is a candidate if its edible delay `+0x198 < 1` and its kind `+0x194 ≠ 2` (potions are skipped). Its distance is measured to the centre of its 40×40 widget: `d² = (cx − (f.ix + 20))² + (cy − (f.iy + 20))²` (ints). If `d² < best`: target = pellet, `+0x190 = 1`, **best = d² − 2500**.
   - The −2500 bias means a later pellet replaces the current one only if its `d²` is more than 2500 smaller. Fish (step 3) face the same margin.
3. **Fish**, only if `0x00503580` is true (always outside the Virtual Tank) **and the eat delay `+0x1a0` ≤ 0**. While the eat delay runs (the first 100 ticks after the ctor), Gus targets food only. Otherwise this is the same master-set walk, filter and type rules as kind 1 (guppies unless protected, types 5–10, pets `0x14/0x15` only with a boss). A fish wins if `d² < best`, and then best = its `d²` (no bias).
   - **Quirk:** `+0x190` is not cleared when a fish beats an earlier pellet. Steering then uses the food branch (§3.2) towards a fish.

`static`.

### 3.2 Steering and eating (`0x004ed460` → `0x004e8f70`), kind 4

**Steering.** If `+0x190` is set ("food" branch), the target point is **`(t.ix + 20, t.iy + 20)`** and:

```
if x + 80 > tx: if vx > -1.8: vx -= 0.1
elif x + 80 < tx: if vx <  1.8: vx += 0.1
if y + 80 > ty: if vy > -1.8: vy -= 0.1
elif y + 80 < ty: if vy <  3.0: vy += 0.3      # dives faster
```

(The comparisons are between doubles. Constants: 80.0 `0x596740`, ±1.8 `0x596780/0x597570`, 0.1 `0x5c3cd0`, 3.0 `0x5965a0`, 0.3 `0x5965c0`.) When `y + 80 == ty`, vy is unchanged. Without the flag (only fish were candidates), the kind-1 default branch applies (`+40`, ±0.1 to ±1.8).

**Eat test gate:** kind 1 runs the eat test only while the eat delay is < 1. **Kind 4 always runs it when there is a target** (`(+0x1a0 < 1 || kind == 4) && target`).

**Eat test (`0x004e8f70`), kind 4.** The half-extents for fish are computed as for kind 1 (45/65; 70/85 for an Ultravore).

1. **Food** (kind 4 only): walk `board+0xac` in list order. Let `ax = x + 80`, `ay = y + 80` (the alien's double position, not its widget). A pellet `f` is eaten if all of these hold:
   - `f.ix − 25 < ax < f.ix + 65` and `f.iy − 25 < ay < f.iy + 65` (strict; ints converted to double);
   - its edible delay `+0x198 == 0`;
   - the alien's `+0x200 < 6`, so after a gulp Gus can't eat again for 5 ticks.

   The kind `+0x194` is **not** checked here: a potion that drifts into the box would be eaten even though §3.1 never targets one (`static`; Nostradamus is not on 2-3). The first pellet that passes is eaten (at most one per tick):
   - **Damage** (health `+0x1f0`, doubles):
     - Virtual Tank (`app+0x880`): **−8**, whatever the pellet.
     - Food level `+0x188 == 3` (**star potion**): **−20**, plus an explosion: sample `0x11d` (`EXPLODE`, gap 3, volume 1.0), then app draws `n = rand(3) + 1`. n times:
       - `size = rand(3) + 3`, `y = rand(40) + 10 + iy`, `x = rand(40) + 10 + ix`, then `EffectShot(x, y, size)`;
       - `size = rand(3) + 3`, `y = rand(20) + 20 + iy`, `x = rand(20) + 20 + ix`, then `EffectShot`.
     - Else by food kind `+0x194`: 3 → **−30**, 5 → **−25**, 4 → **−20**. These kinds come only from `0x00543570` (via `0x00547530` cases 6, 5, 7), which looks like the Virtual Tank's bought-food drop; not in Adventure.
     - Otherwise (an ordinary pellet): **−(2 · level + 4)**, i.e. **level 0: −4, level 1: −6, level 2: −8**.
   - `+0x200 = 10` (gulp animation, §7.1, and the 5-tick no-eat window).
   - Slurp sound `0x005384c0(0)`: if sample `0x138` is off its 3-tick throttle, **1 app draw `r % 5`** (`food.md`, Brinkley).
   - Remove the pellet (slot 76, `0x004d9570`).
2. **Fish**, in the same call, after the food step whether or not a pellet was eaten. This runs only if `0x00503580` is true and (kind ≠ 4 or eat delay ≤ 0). It is the kind-1 test (master set, address order, one fish per tick). So once the eat delay has run out, **Gus can eat a pellet and a fish in the same tick**.
3. **"Ate" hook** (slot 77, `0x004d45a0`) when a fish is eaten: the chomp sound (`rand(3)` if off its cooldown) and the gore burst, as for kind 1. Kind 4 adds two writes in between: **`+0x200 = 10` and health −= 15** (`0x596578`). **Eating a fish costs Gus 15 HP** and plays the gulp animation.

Numbers for a fresh Gus (100 HP, no fish eaten): **25 level-0 pellets, 17 level-1 (16 × 6 = 96, so the 17th kills), 13 level-2, or 5 star potions.** Mixes add up the same way. Each fish he eats takes 15 off.

`static`.

## 4. Clicks and damage

### 4.1 Input routing (`Board` mouse-down @ `0x00543920`)

After the Bilaterus step (`+0xf4`, unchanged), the handler looks at the **first** alien in `+0xb8`. If its kind is 4 **and there is no boss** (`board+0x110 == 0`; the byte is set at `0x00543d4d`), the click takes the **food path** instead of the laser path:

1. The click must satisfy `31 ≤ x ≤ 586` and `61 ≤ y ≤ 379` (`x − 31 ≤ 0x22b`, `y − 61 ≤ 0x13e`). This is **20 px shorter at the bottom than the normal food path** (≤ 399). Outside this area nothing happens: no shot and no sound.
2. **No price is charged.** Neither `0x00540b30` nor the money check runs. The food lockout `+0x2c8` and the "pet 5 mode" test of the normal path aren't checked either.
3. Delay: `0x004d46f0(first alien, x, y)` is true when the alien's widget centre `(acx, acy)` satisfies `x − 35 < acx < x + 75` and `y − 35 < acy < y + 75`. A click near Gus's middle gives an **edible delay of 20**, otherwise 0. So a pellet dropped straight into his mouth can't be eaten (or targeted) for 20 ticks.
4. `Board::AddFood(x − 10, y − 10, motion 0, flag 0, delay, level −1)` (`0x00543280`). This is the normal drop: the count limit (`max_food + extra`; its refund `0x0053c280` only pays back when there are no aliens, so nothing here), the Food ctor's **1 app draw** `(r & 1) + 3`, and `SOUND_DROPFOOD`. An armed star potion (`board+0x440`) drops as level 3 and disarms, as on the normal path (`food.md`).
5. Byte `+0x4ec = 1`, `+0x3c0 = 0x00537b60()`.

There is **no `Alien::Hit`, no click shot, no laser sound and no shot-sound RNG** on this path. That is what "lasers can't hurt this baddie" means.

If kind 4 is **not first** in the list, or a boss is present, the normal laser path runs. `Alien::Hit` (`0x004fa6f0`) has no kind-4 arm: it would do `3 × laser` damage with kind-1 knockback. On 2-3 Gus is the only alien, so this doesn't occur there.

**Coin under Gus.** `Coin` mouse-down (`0x004e5450`) re-dispatches a click that lies inside an alien widget to the board's handler (`alien-sylvester.md#41`). With Gus, that **drops a free pellet** at the click (step 3 may give it delay 20), and the coin is collected as well.

`static`.

### 4.2 Pets and other damage sources

Kind-4 damage scaling in pet attacks (from the pet specs, confirmed at the addresses):

| Source | Normal damage | Kind 4 |
|---|---|---|
| Itchy bite (`0x004fb5b0`, pet 2) | 1.0 | **0.25** (`0x596818`; also kind 5) |
| Gash bite (`0x004fb5b0`, pet 0x11) | 3.0 | **0.5** (`0x5c3c20`; also kind 5) |
| Rufus (`0x004e7c30`, `0x004e7f8e`) | 2.0 | **0.5** (kind 6 also 0.5, kind 5 0.25) |
| Stanley's ball (`pet-stanley.md`) | 15 | 15 (no kind check) |

All of them kill through `0x004f9c70(1)`, which gives the same kind-4 death (§4.3).

`static`.

### 4.3 Death (`0x004f9c70(1)`), kind 4

1. Release an attached missile; **diamond** `AddCoin(ix + 25, iy + 25, kind 4)` (fall arg −1.0), if there is no boss and `0x004d70b0` holds (always true outside mode 5). This is identical to kind 1.
2. Remove the alien and its shadow; run the end-of-wave check `0x00539c90` (lockout `+0x2c8 = 36`, tank music, guppy byte `+0x55`).
3. **Debris, kind-4 branch** (app RNG, in this order):
   1. `n = (rand() & 1) + 2` (**2 or 3**, kind 1: 0 or 1). n times:
      - `y = rand(70) + 30 + iy`, `x = rand(70) + 30 + ix`, `Bubble`;
      - `y = rand(50) + 50 + iy`, `x = rand(50) + 50 + ix`, `Bubble`.
   2. `n = rand(3) + 4` (**4 to 6**, kind 1: 0 to 2). n times:
      - `size = rand(3) + 3`, `y = rand(50) + 20 + iy`, `x = rand(50) + 20 + ix`, `EffectShot`;
      - `size = rand(3) + 3`, `y = rand(30) + 40 + iy`, `x = rand(30) + 40 + ix`, `EffectShot`.
   3. `n = (rand() & 1) + 2` (**2 or 3**, kind 1: 0 or 1). The same two shots with `size = (rand() & 1) + 6`.
   4. Sounds `0x11d`, `0x11e` (gap 3).
4. **No `DeadAlien`.** The kind-4 branch returns after the boss cleanup `0x004f9950`, without calling `AddDeadAlien`. Gus simply bursts, which is consistent with `DeadAlien`'s draw having no kind-4 image (`0x004dddb0` maps 1/2/3/5/6/7/0x15 only). So there are none of the 10 corpse bursts either.

Effect shots can be capped (fewer than 20 in `+0xd8`) and carry the heap-dependent start-delay draw (`alien-sylvester.md#deviations`). With up to 2·6 + 2·3 = 18 sparks in one tick, the 20-shot cap can actually bite here.

`static`.

## 5. Fish and other objects while Gus is in

- Hunger, coins and the swim logic follow the kind-1 rules (`alien-sylvester.md` §5).
- **Guppies don't chase food** while the first alien is kind 4. `Fish::UpdateHunger` returns "not chasing" at `0x004f1816` (`food.md` step 5). **Breeders** do the same at `0x004f13e5` (`0x004f1340`, called from `Breeder::Update`). So Gus gets every pellet, unless a pet or another fish type takes it.
- Debug keys `z`/`x` (Board key handler `0x00549dc8`, app mode 3 only) are ignored while the first alien is kind 4.
- `Alien` slot 70 (`0x004d4540`) adds 3 to the counters at `[arg+0x14..0x1c]` for kind 4, and 1 to `[arg+0]`, `[arg+0xc]`, `[arg+0x10]` for every other kind. Its caller and the counters' meaning (stats? score?) are open.

`static`.

## 6. RNG summary (2-3)

| When | Stream | Draws, in order |
|---|---|---|
| Spawn | app | ctor `rand()`, `rand(10)`; shadow `rand(5)` (as kind 1) |
| Wandering (no food, no eligible fish), re-roll timer > 20 | app | `rand(10)`; if 0: `rand() & 3` |
| Player click (food path) | app | Food ctor `(r & 1) + 3` (if under the count limit). **No shot-sound draws** |
| Gus eats a pellet | app | slurp `r % 5` (only if `0x138` is off its throttle); star potion first: `rand(3) + 1` count, then per piece `rand(3)`, `rand(40)`, `rand(40)`, shot ctor, `rand(3)`, `rand(20)`, `rand(20)`, shot ctor. Order: damage and explosion, then slurp |
| Gus eats a fish (after the eat delay) | app | chomp `rand(3)` (if off cooldown); gore `rand(3)` + per piece `rand(20)`, `rand(20)`, shot ctor (if `app+0x882`) |
| Gus dies | app + global | §4.3: `rand()&1`, 2–3 × (4 draws + 2 bubbles), `rand(3)`, 4–6 × (6 draws + 2 shot ctors), `rand()&1`, 2–3 × (6 draws + 2 shot ctors). Bubbles draw from the global stream |
| 2-5 only: level setup | app | `rand() & 1` (kind 4 or 5) |
| 2-5 only: after each spawn | app | the re-roll in §1 |

Within the tick, everything Gus does happens in his own `Update` (food step, then fish step, then the death check), in z-order after the guppies. Click draws happen in the input phase.

`static`.

## 7. Drawing and sound

### 7.1 Alien (`0x004db210` → `Alien::DrawSprite` @ `0x004dae20`), kind 4

- Sheet **`IMAGE_GUS`** (global `0x005e8ae4`, written at `0x005092fd`; image id 0xbf; manifest `gus`, 10 cols × 3 rows, 160×160 cels).
- Mirror: by vx, as kind 1 (vx ≥ 0 → mirrored).
- Rows and columns:
  - Default: row 0, column = cel `+0x1c0`. While turning: row 1, as kind 1.
  - **Gulp** (`+0x200 = c > 0`, no boss): **row 2, column c**. The eat sets 10 and the same update decrements it, so the draws see c = 9, 8, …, 1 (column 0 is never shown). The mirror stays the vx facing even while turning.
- **No white hit flash** for kind 4 (unless a boss is present). The additive flash branch excludes it.
- Warp-in scaling: the same as kind 1.
- The health bar (with Blip, `board+0x178`, not tank 5) is the same as for other kinds.

`static`.

### 7.2 Sounds

| Event | Sample | Notes |
|---|---|---|
| Spawn | `0x123` `SOUND_GUFFAW` | direct, unthrottled (kinds 1–3: `ROAR`) |
| Click | `SOUND_DROPFOOD` | from `AddFood` (no laser sounds); not played when an armed star potion drops |
| Pellet eaten | `0x138` `SLURP` / `SLURP2` / `SLURP3` | `r % 5`: 0 → SLURP3, 1 → SLURP2, else SLURP; throttle 3 |
| Star potion eaten | `0x11d` `EXPLODE` | gap 3, before the slurp |
| Fish eaten | `CHOMP` / `CHOMP2` | as kind 1 |
| Death | `0x11d`, `0x11e` | gap 3 each; no dead-alien rumble (no corpse) |

`static`.

## Deviations

`static`. None of our own yet. The heap-dependent effect-shot start delay (`alien-sylvester.md#deviations`) applies to every spark here: the star-potion burst, the death debris and the fish gore.

## Open questions

- The meaning and caller of `Alien` slot 70 (`0x004d4540`) and its +3/+1 counters.
- Whether the `+0x190` quirk (food-style steering towards a fish that beat a pellet) shows in practice. It needs a fish within `√(d²_food − 2500)` while food is present after the eat delay.
- The food kinds 3/4/5 (`0x00543570`): confirm that they are Virtual Tank only.
- Whether `0x004d9570` removes the pellet from `+0xac` at once. If it does, a second pellet can't be eaten in the same tick anyway, since the food loop exits on the first hit.
- What the Coin re-dispatch path does with the 20-tick delay rule (the click is at the coin, which is inside Gus's widget, so it is probably often within the ±55 box).

## Recording

A 2-3 recording (the game's `-record` demo plus a CE capture, `dynamic-analysis.md` routine) must exercise:

1. The warning, the WARNING! dialog, the spawn (GUFFAW, warp, 3 draws), and the first 100 ticks, when Gus ignores fish.
2. **Clicks while he is in**: far from him (delay 0), on his middle (delay 20), below y 379 (nothing), and on a coin over him. Check that money doesn't change and that the count limit still applies.
3. **Feeding**: pellets of food level 0, then buy food quality and feed level 1 (and level 2 if affordable), plus at least one **star potion** (armed from the shop, then clicked). Health should drop by 4 / 6 / 8 / 20 per pellet, with the 5-tick gulp gap.
4. Guppies **not chasing** food while he is in, and Gus **eating a guppy** after the eat delay (−15 HP, gulp).
5. **The kill**: the explosion debris (2–3 / 4–6 / 2–3), no corpse, the diamond and its collection, the music back to the tank song and the 36-tick lockout. Then the countdown restarts from 3000 and a second wave runs.
6. Optional, in a separate zoo: Itchy, Gash or Rufus biting him (0.25 / 0.5 / 0.5).

`static` (a plan).
