# Alien: Sylvester (`Sexy::Alien`, type `0x16`, kind 1)

Status: `draft`. **Confirmed** against `tank1-2_full_01` from the warning through the kill (420 ticks, bit-exact): countdown, warning position draws, spawn (ctor + shadow draws), warp-in, chase, every hit (damage, knockback, cooldown, click-flash and effect-shot draws), death, and the diamond drop. Not yet confirmed: the DeadAlien debris bursts (heap-dependent, below) and eating (no fish was eaten in the capture). Addresses are inner-PE VAs; offsets are raw `this`-relative.

Sylvester is the first alien. In Adventure he first appears in **tank 1-2**; tank 1-1 has no alien waves. `Sexy::Alien` is one class shared by all the "simple" aliens. Its kind code is at `alien+0x1ec`, and Sylvester is **kind 1**. Kind 2 (tank 1-3) uses the same sprite with other stats (§2 table). Most of this spec also holds for kind 2.

Notation:
- `rand(n)` = `app MTRand::NextNoAssert() % n` (`[app+0x7b0]`, `0x0040aeb0`, unsigned modulo).
- `rand()` = raw 31-bit draw from the same stream.
- `grand()` = global `Sexy::Rand` (`0x0040f9e0`).
- "Tick" = one 28 ms logic tick (`framework/timing.md`).
- `ix, iy` = the object's widget position (`+0x30/+0x34`, ints). `x, y` = its double position.

## 1. Wave timing (`Board::Update` @ `0x00547610`, section `0x005481c4`–`0x00548e05`)

### Board fields

| Offset | Type | Meaning |
|---|---|---|
| `+0x2bc` | int | next alien kind (0 = this level has no waves) |
| `+0x2c0` | int | wave countdown, ticks |
| `+0x2c4` | int | set to 35 on spawn, −1 per tick while non-zero (read by `0x0053be54`; purpose TODO) |
| `+0x2c8` | int | set to 36 when the last alien dies (`0x00539c90`); −1 per tick at the top of `Board::Update`. While > 0, clicks **within 50 px of the last shot** (`+0x3d0/+0x3d4`, `dx²+dy² ≤ 2500`) can't drop food; a click farther away resets it to 0 (`0x00543972`). Confirmed |
| `+0x2cc`,`+0x2d0` | int | spawn position A (x, y), rolled at the warning |
| `+0x2d4`,`+0x2d8` | int | spawn position B, used only by the two-alien kinds 9–12 |
| `+0x3e4` | int | laser level (damage, §4) |
| `+0x45c` | int | waves spawned so far on this level |
| `+0x488` | int | fish lost counter (+1 per fish eaten, `0x004df0b0`) |
| `+0x110` | ptr | boss alien (kind `0x15`). While set, the countdown is paused |

### Level setup

Level start (`0x00541a20`) sets `+0x2c0 = 3000`, `+0x2bc = 2` and `+0x3e4 = 2` (laser level). At its end (`0x00541c1a`) it calls the per-level table `0x00541200`, which overwrites them. For the normal game (`app+0x880 == 0`), tank 1 (`board+0x3c8 == 1`), it jumps on `board+0x3cc − 1` (table `0x00541888`):

| Level | kind `+0x2bc` | countdown `+0x2c0` | `+0x340` (meaning TODO) |
|---|---|---|---|
| 1-1 | **0** (no aliens) | 3000 (never ticks) | 150 |
| 1-2 | **1 (Sylvester)** | **1750** | 500 |
| 1-3 | 2 | 3000 | 2000 |
| 1-4 | 3 | 3000 | 3000 |
| 1-5 | 3 | 3000 | 5000 |

No RNG on tank 1. Some later levels draw one here, e.g. 2-5: `kind = 4 | (rand() & 1)`.

### Per-tick countdown

This runs after the ambient-bubble draw (`rand(1000)`, `sim/board.md`) and after `+0x2c4` is decremented:

1. Skip the wave block if the board is paused (`+0x94`), or if mode `app+0x87c == 5` and byte `+0x4fe == 0`.
2. **Skip the wave block while any alien exists**, i.e. while `+0xb8` (aliens), `+0xf4` (Bilaterus) or `+0xdc` (missiles) is non-empty. The countdown is frozen while Sylvester is alive.
3. If `kind != 0 && +0x110 == null && byte +0x4f4 == 0`: `countdown −= 1`. (With byte `+0x4fe` set and countdown 33: sound `0x143`.)
4. Countdown == 30, pet-21 counter `board+0x198` non-zero, and tank ≠ 5: "Sneeze of Power" `0x00543f40`. This is the only **`grand()`** user here: `grand() % 100 < 50` postpones the wave (`countdown = 635`, plus more `grand()` draws for bubbles). It can't happen on 1-2 without that pet.
5. Branch on the new countdown value (the mode-4 "2999" branch is omitted):
   - **276**: one-time hint messages. No RNG.
   - **275 = warning**: message, alien music, sound `0x106`, then **4 app draws in order**:
     `A.x = rand(450) + 20`, `A.y = rand(195) + 105`, `B.x = rand(450) + 20`, `B.y = rand(195) + 105`.
   - **1..274**: at 100, a level-specific hint (not tank 1). At 1, music call `0x0054b1a0(1,1,0)`. No RNG.
   - **≤ 0 = spawn**:
     1. `+0x2c4 = 35`; bytes `+0x4ec = +0x4ed = 0`.
     2. `AddAlien(kind, A.x, A.y, sound=1)` (`0x00545620`, §2).
     3. (Mode 4 only: extra aliens, with more position draws.)
     4. Re-roll the kind for the next wave. On tank 1 only **1-5** does: `kind = (rand() & 1) ? 3 : 9`. Levels 1-2 to 1-4 keep their kind and **draw nothing**.
     5. `+0x45c += 1`; `countdown = 3000`.

**Timeline on 1-2 (static):** the countdown starts at 1750. The warning comes on the 1475th counted tick and the spawn on the 1750th (≈ 49 s). The next wave is counted from 3000 again, but **only once the alien is gone**: a warning 2725 ticks after the kill and a spawn 3000 ticks after it. Paused ticks don't count.

## 2. Spawn (`Board::AddAlien` @ `0x00545620`, kind 1 path)

1. Global `0x005e8f18 = app+0x484 − 216` (only used by the Virtual-Tank helper `0x00503580`, which always returns true in the normal game).
2. `new Alien(x, y, kind)` (`0x004ecfa0`, `0x208` bytes): **2 app draws** (below).
3. Warp-in effect `0x004ed370` (any kind except `0x14`): `new Warp(ix + 30, iy − 40)` (type `0x21` → list `+0xf0`), added and brought to front. Neither the Warp ctor `0x004ecf00` nor `Warp::Update` `0x004f3e70` uses RNG.
4. `AddGameObject(alien)` → list `+0xb8`, plus the master set `+0x104`. `WidgetManager::AddWidget`.
5. Shadow via `0x00544800` → `0x00544370(2, alien)`, only if there are fewer than 40 shadows. The Shadow ctor (`0x004ec4f0`) costs **1 app draw**: `shadow.iy = rand(5) + 410`.
6. `0x00539ad0(0)`: clear byte `+0x55` on every guppy (`+0xa0`), breeder (`+0xc0`) and pet (`+0xb4`), and probably more lists. Fish read it as "no alien present".
7. `Board::RebuildZOrder`, then the arrival sample (kinds 1–3: `DAT_005e8c6c`).

**Spawn cost: 3 app draws** (ctor 2, shadow 1), with no global draws in the normal game.

### Alien constructor RNG order (kind 1)

1. `r = rand()`: even → `vx = −3.0`, last-vx `+0x1c8 = −1.0`. Odd → `vx = +3.0`, `+0x1c8 = +1.0`.
2. (Kind `0x14` only: `rand(5)` for its speed.)
3. (Only if `app+0x880` [Virtual Tank]: health = `grand() % 100 + 150`, or `% 50 + 70` for kind 4. These are the `0x4ed2c6`/`0x4ed2de` call sites. Not reached in Adventure.)
4. `+0x1d0 = rand(10)` = initial wander mode (§3.3). Values 4–9 mean "no target change".

### Fields (raw offsets) and initial values

| Offset | Type | Init | Meaning |
|---|---|---|---|
| `+0x30`,`+0x34` | int | trunc(x), trunc(y) | widget pos (`ix, iy`) |
| `+0x38`,`+0x3c` | int | 160, 160 | widget size (80×80 for kind `0x14`) |
| `+0x98` | ptr | 0 | GameObject: attached missile, if any (none for Sylvester) |
| `+0x158` | int | 15 | warp-in counter (§3.1). Draw scales the sprite in while it is 1..9 |
| `+0x160`,`+0x168` | double | x, y | position (top-left). **Note: x/y here are `+0x160/+0x168`, not Fish's `+0x158/+0x160`** |
| `+0x170`,`+0x178` | double | ±3.0, 0 | vx, vy |
| `+0x180`,`+0x188` | double | 0, 0 | wander target vx, target vy |
| `+0x1a0` | int | 100 | eat delay: no eating until it reaches 0 (§3.2) |
| `+0x1a4`,`+0x1a8` | int | 0, 75 | shoot timer/interval (kinds 5/6 only) |
| `+0x1ac` | int | 0 | special-action timer (kinds 5/6/7). Always 0 for Sylvester |
| `+0x1b0` | int | 0 | animation counter |
| `+0x1b4` | int | 0 | turn counter (±10 → 0) |
| `+0x1b8` | double | **2.0** | movement divisor: position advances by `v / +0x1b8` per tick |
| `+0x1c0` | int | 1 | sprite cel |
| `+0x1c8` | double | ±1.0 | last non-zero vx (turn detection) |
| `+0x1d0` | int | `rand(10)` | wander mode |
| `+0x1d4` | int | 40 | counter, +1 per wander tick (unused by kind 1) |
| `+0x1d8` | int | 20 | wander re-roll timer |
| `+0x1dc`..`+0x1e8` | int | 0, 0, 300, 75 | boss timers (kind `0x15` only) |
| `+0x1ec` | int | kind | alien kind |
| `+0x1f0` | double | **50.0** | health |
| `+0x1f8` | double | = health | max health |
| `+0x200` | int | 0 | hit cooldown / white flash (§4) |
| `+0x204` | byte | 0 | kind 7 state |

Per-kind ctor values:

| Kind | Divisor `+0x1b8` | Health `+0x1f0` | Notes |
|---|---|---|---|
| 1 (Sylvester) | 2.0 | 50 | sprite `DAT_005e8ebc` |
| 2 | 1.6 | 60 | same sprite as 1 |
| 3 | 1.2 | 130 | |
| 4 | 1.6 | 100 | |
| 5 | 1.2 | 150 | |
| 6 | 3.5 | 220 | y forced to 280 outside mode 5 |
| 7 | 0.5 | 260 | |
| `0x14` | 0.8 + rand(5)/10 | 1.0 | 80×80, eat delay 40, `+0x200 = 30` |

## 3. `Alien::Update` (vtable slot 22 @ `0x004fd560`), kind 1

1. Return if there is no board or it is paused. Run the common `GameObject` pre-update `0x004f22c0` (counter housekeeping, no RNG).
2. **Warp-in** (`+0x158`, starts at 15): `lunge = 0`. If the counter is non-zero:
   - if it is ≤ 10, `lunge = (vx ≥ 0) ? +3 : −3`;
   - `counter −= 1`;
   - if the counter is now > 8, **return** (no movement, no chase, no timers).

   Net effect: the alien is frozen for 6 ticks. On ticks 7–15 it moves with an extra ±3 px/tick in x. After that, normal behaviour.
3. **Chase or wander** (`0x004f03d0`):
   - *Has prey?* (`0x004d4580` → `0x005392a0`): true if lists `+0xa0` (guppies), `+0xa4` (Oscars) or `+0xd0` (Starcatchers) are non-empty, or one of four pet checks holds. (Always true when the boss `+0x110` is set.)
   - If there is prey: `target = NearestPrey()` (§3.1). If there is a target, steer (§3.2) and the tick is "chasing". With no target, fall through as "wandering".
   - If wandering (§3.3).
4. Clamp (all kinds except `0x14`): `x ∈ [−10, 490]`, `y ∈ [85, 290]`. Velocities are not touched. The clamp runs **before** the move, so the position can sit one step outside the range until the next tick.
5. **Move:** `x += vx / div + lunge`; `y += vy / div` (div = 2.0).
6. `+0x200 −= 1` if > 0 (hit cooldown). `+0x1a0 −= 1` if > 0 (eat delay).
7. (Kinds 5/6 shooting: skipped.)
8. Widget resize to `(trunc(x), trunc(y))` (vtable slot 42).
9. **Animation** `0x004daa00` (§3.4). It can move the alien.
10. If health ≤ 0: die (`0x004f9c70(1)`, §4.3).

### 3.1 Nearest prey (`0x004e8b60`)

Alien centre = `(ix + 160/2, iy + 160/2)`. Best = 100,000,000. The candidates are **every object in the board's master set `+0x104`**. It is a `std::set<GameObject*>`, so iteration is in **pointer-address order**, not list order.

An object must have `+0xac < 0` and `+0x120 < 1` (GameObject fields with defaults −1 and 0; TODO meaning, probably "held by a pet" and "dying"). Then by type `+0x8c`:
- `0` guppy: a candidate unless it is protected. It is protected when global `0x005e89d0` ≠ 0 (a protector pet is active) and the guppy's `+0x1a0 < 2`.
- `5..10` (Oscar, Ultra, Gekko, Starcatcher, Grubber, Breeder): always.
- `0x14`/`0x15` pets: only while a boss is present.
- Everything else: skipped.

`d² = (cx − (o.ix + o.w/2))² + (cy − (o.iy + o.h/2))²` (ints). The candidate wins if `d² < best`. Ties keep the earlier one in address order.

### 3.2 Steering and eating (`0x004ed460` → `0x004e8f70`)

Target point = `(t.ix + 40, t.iy + 40)` (ints, regardless of the target's size). Kind 1 uses the "default" branch:

```
if x + 80 > tx: if vx > -1.8: vx -= 0.1
elif x + 80 < tx: if vx <  1.8: vx += 0.1
if y + 80 > ty: if vy > -1.8: vy -= 0.1
elif y + 80 < ty: if vy <  1.8: vy += 0.1
```

(Doubles. With div 2.0, the top cruise speed is 0.9 px/tick per axis. The initial ±3.0 decays only while it points the wrong way.)

Then, **only if eat delay `+0x1a0` < 1** (for a fresh Sylvester that is about tick 107 after spawn, counting the 6 frozen ticks), run the eat test `0x004e8f70`:
- Walk the master set again with the same filter and type rules. The half-extents are 45 (x) and 65 (y), or 70/85 when the candidate is an Ultravore (type 6).
- `dx = (ix + 80) − (o.ix + o.w/2)`, `dy = (iy + 80) − (o.iy + o.h/2)` (ints, from the previous tick's widget positions). A candidate is eaten if `|dx| < hx && |dy| < hy` (strict).
- On the first hit: call the alien's "ate" hook (slot 77 `0x004d45a0`), then the victim's "eaten" hook (Fish slot 75 → `0x004df0b0`). The fish is removed from its list, the widget manager and the master set, its shadow is deleted, and `board+0x488 += 1`. No DeadFish is created. **Only one fish per tick.**
- `Alien` "ate" hook (`0x004d45a0`):
  1. Eat sound `0x00538560`. If sample `0x113` is off its 3-tick cooldown: **1 app draw** `rand(3)` picks `0x113` (r = 0) or `0x114`.
  2. (Kind 4 only: +0x200 = 10, health −= 15.)
  3. If byte `app+0x882` is set (written as 1 by the `WinFishApp` ctor `0x0054d850`; TODO whether an option clears it), spawn a gore burst:
     - **1 draw** `n = rand(3) + 2`;
     - then n times: `y = rand(20) + 15 + victim.iy`, `x = rand(20) + 15 + victim.ix` (Ultravore victims: +55 instead of +15), followed by `EffectShot(x, y, 1)` (§6 note on effect shots).

### 3.3 Wander (`0x004f03d0` returned false)

1. Wander mode `+0x1d0`:
   - 0 → target vx = −1.5
   - 1 → target vx = +1.5
   - 2 → target vy = −1.5
   - 3 → target vy = +1.5
   - 4..9 → no change.

   Targets persist until changed; both start at 0.
2. `vy` moves toward target vy: `if tvy < vy: vy −= 0.1`, then `if vy < tvy: vy += 0.1`. These are two sequential ifs, so floating-point noise can cause a ±0.1 dither. Reproduce this exactly.
3. `vx` moves toward target vx the same way.
4. `+0x1d4 += 1`.
5. If `+0x1ac == 0` (always, for kind 1): `+0x1d8 += 1`. When it exceeds 20: reset it to 0, then **draw `rand(10)`**. If that is 0, **draw again**: `mode = rand() & 3`.

   Since `+0x1d8` starts at 20, the first wander tick draws. After that, every 21st wander tick costs 1 draw, with a 10 % chance of a 2nd. The timer doesn't advance while chasing.

There is no wall bounce. The alien presses against the clamp until its mode changes.

### 3.4 Animation (`0x004daa00`, no RNG, but it moves the alien)

1. Turn detection against last-vx `+0x1c8`:
   - last > 0 and vx < 0 → turn = +10;
   - last < 0 and vx > 0 → turn = −10.

   Then the turn counter steps 1 toward 0.
2. While turning: cel = `9 − turn` (turn > 0) or `turn + 10` (turn < 0).
3. Not turning and `−1.6 ≤ vx ≤ 1.6`: `anim = (anim + 1)`, wrapping 19 → 0; `cel = anim / 2`.
4. Not turning and **|vx| > 1.6** ("stroke"):
   - `anim += 1`, then by anim:
     - < 7: `x −= vx/div·0.25`, `y −= vy/div·0.25`
     - 7..10: `x += vx/div·0.75`, `y += vy/div·0.75`
     - 11..39: cel 6, `x += vx/div·0.5`, `y += vy/div·0.5`
     - 40..50: the same 0.5 push, `cel = |anim − 50| / 2`
     - > 50: cel 0, anim 0.
   - For anim < 11, cel = anim / 2.

   This happens right after spawn (|vx| = 3) and after knockback.
5. If vx differs from last-vx and both are non-zero: last-vx = vx.
6. If `+0x98` (missile) is set, `0x004d7020`. N/A here.

## 4. Being shot

### 4.1 Input routing (`Board` mouse-down @ `0x00543920`)

While `+0xb8` (aliens) is non-empty, and the first alien isn't kind 4 (or a special pet widget `+0x84` exists), a left click never drops food. Instead:

1. Only if click y > 40: walk the aliens in **list order**, calling `Alien::Hit(x, y)` (`0x004fa6f0`) on each. Stop at the first that **dies**; a non-lethal hit does not stop the walk, so overlapping aliens are all hit.
2. Walk missiles (`+0xdc`) with `0x004d82a0`.
3. `AddClickShot(x − 40, y − 40)` (`0x00543640`): an 80×80 `Shot` (type `0x20`, list `+0xd8`). Its 2-arg ctor `0x004ec600` uses **no RNG** and sets its kind `+0x160 = 0`. It also records `board+0x3d0/+0x3d4 = (x, y)`.
4. Set byte `+0x4ed` and `+0x338 = 0x00537b60()`.
5. Shot sound `0x00538ae0`:
   - **draw 1**: `rand() & 1` → play `0x144`, or `0x144` + `0x145`;
   - **draw 2**: `rand(10) == 0` → also play `0x132`.

A click with y ≤ 40 does nothing. Clicking a **coin** that lies inside an alien's 160×160 widget rect re-dispatches the click to the board (`0x00539d40`, called from the coin's mouse-down), so it also shoots. TODO: whether the coin is collected as well.

### 4.2 `Alien::Hit(cx, cy)` (`0x004fa6f0`), kind 1

1. Hit box: `x < cx < x + 160` and `y < cy < y + 160` (strict, doubles). Otherwise no effect.
2. **If the hit cooldown `+0x200` > 0: no effect.** After a non-lethal hit the alien ignores clicks for 10 ticks.
3. `health −= 3 × laser`, with laser = `board+0x3e4`. It starts each level at 2 (6 damage) and each store upgrade adds 1, capped at 12 (`0x00549735`), for a maximum of 36 per hit. With 50 HP, Sylvester needs **9 hits at the base laser**.
4. `EffectShot(cx − 40, cy − 40, 2)` (`0x005436f0`) and sound `0x126`.
5. **Knockback** sets the velocity away from the click point by zone, relative to the alien's top-left. Let `s = div` (2.0), so the net move is always 3 or 2.5 px/tick per axis after dividing by `s`. Columns are split at `x+60 / x+100` and rows at `y+60 / y+100`:

   | Click zone | vx | vy |
   |---|---|---|
   | left, top (`cx < x+60`, `cy < y+60`) | +2.5s | +2.5s |
   | left, middle (`cx < x+60`, `y+60 ≤ cy < y+100`) | +3s | — |
   | left, bottom (`cx < x+60`, `cy ≥ y+100`) | +2.5s | −2.5s |
   | centre column, top (`x+60 ≤ cx < x+100`, `cy < y+60`) | — | +3s |
   | centre (`x+60 ≤ cx ≤ x+100`, `y+60 ≤ cy ≤ y+100`) | — | — |
   | centre column, bottom (`cx ≤ x+100`, `cy > y+100`) | — | −3s |
   | right, top (`cx > x+100`, `cy < y+60`) | −2.5s | +2.5s |
   | right, middle (`cx > x+100`, `y+60 ≤ cy ≤ y+100`) | −3s | — |
   | right, bottom (`cx > x+100`, `cy > y+100`) | −2.5s | −2.5s |

   "—" = unchanged. The original mixes `<` and `≤` at the 100 boundaries exactly as shown.
6. If health ≤ 0: die (§4.3) and return true. Otherwise set `+0x200 = 10` (cooldown, and the white flash at alpha `25·+0x200` in `0x004dae20`) and return false.

### 4.3 Death (`0x004f9c70(1)`), kind 1

1. Release the attached missile, if any.
2. If there is no boss and this is the normal game: **`AddCoin(ix + 25, iy + 25, kind 4 = diamond)`**, with fall speed −1.0 passed to the ctor (TODO: check against `coin.md`'s kind-4 handling). The diamond is worth 200 when collected. **No money is awarded directly.** The Coin ctor uses no RNG.
3. Remove the alien from its widget, its list and the master set, and delete its shadow (`0x004db250`).
4. `0x00539c90`: if no aliens, Bilaterus or missiles remain:
   - bytes `+0x4ec/+0x4ed = 0`;
   - **`board+0x2c8 = 36`** (food lockout);
   - tank music (unless a boss is present);
   - `0x00539ad0(1)`: guppy byte `+0x55 = 1`.
5. Debris, app RNG in this order:
   1. `n = rand() & 1`; n times:
      - `y = rand(70)+30+iy`, `x = rand(70)+30+ix`, `Bubble(x,y)`;
      - `y = rand(50)+50+iy`, `x = rand(50)+50+ix`, `Bubble(x,y)`.
   2. `n = rand(3)`; n times:
      - `size = rand(3)+3`, `y = rand(50)+20+iy`, `x = rand(50)+20+ix`, `EffectShot(x,y,size)`;
      - `size = rand(3)+3`, `y = rand(30)+40+iy`, `x = rand(30)+40+ix`, `EffectShot`.
   3. `n = rand() & 1`; n times: the same two shots with `size = (rand() & 1) + 6`.
   4. Sounds `0x11d`, `0x11e`.
6. `AddDeadAlien(ix, iy, cel, kind, facingRight = vx > 0)` (`0x005442c0` → ctor `0x004eec10`, type `0x1a`, list `+0x9c`, no ctor RNG).
7. Boss cleanup `0x004f9950` (kind `0x15` only).

### 4.4 `DeadAlien::Update` (`0x004f6050`)

The lifetime counter `+0x44` starts at 125. Each unpaused tick:
1. If it is < 1, remove the object.
2. Otherwise decrement it.
3. At counter < 105, an alpha fade.
4. Every 5th value above 50: a sound, no RNG.
5. **Whenever `counter % 7 == 0 && counter > 50`** (values 119, 112, …, 56: **10 bursts**), draw:
   - 2 bubbles (4 draws: y, x, y, x with `rand(70)+30` and `rand(50)+50`);
   - then 4 effect shots, each `size, y, x`. Sizes are `rand(3)+3` for the first two and `(rand()&1)+6` for the last two; offsets alternate `rand(50)+20` and `rand(30)+40`.

   That is 16 app draws, plus the effect-shot ctor draws.
6. It drifts and sinks (fixed constants 0.03, 0.05; clamp y ∈ [−10, 490]) with no RNG.

## 5. Guppies while an alien is present

The guppy-side rules are in `fish-guppy.md`, `food.md` and `coin.md`. In summary:
- **Hunger doesn't drop** and **no coins are dropped** while `+0xb8` or `+0xf4` is non-empty (`0x004da780`). `Fish::Update` `0x004f6970` skips its no-alien step (vtable slot 81, `0x004f1400`).
- Guppies **don't flee**. Without pet 12 (`board+0x174`), `Fish::Update` runs its normal swim/mode logic, including the 21-tick mode re-roll and its RNG. They still chase food, but none can be dropped: clicks become shots, and the 36-tick `+0x2c8` lockout follows the kill.
- A guppy is eaten whole (§3.2): removed, with no DeadFish and no RNG on its side.
- Mode 123 "startled" (global RNG) is set only by `0x004d6040`. Its sole caller is `0x0053bfb0`, which the Board's mouse-down calls for a negative click count (**right-click**, guess). The alien does not set it.

## 6. RNG summary (normal game, tank 1-2)

The update order follows `sim/board.md`: `Board::Update` first, then objects in z-order. DeadAlien sits early, Warp after the coin sub-list, guppies, then **Alien**, then Shots and Coins.

| When | Stream | Draws, in order |
|---|---|---|
| Every tick, `Board::Update` | app | `rand(1000)` ambient bubbles (unchanged by aliens) |
| Countdown hits 275 (warning) | app | `rand(450)`, `rand(195)`, `rand(450)`, `rand(195)` |
| Countdown ≤ 0 (spawn), inside `Board::Update` | app | Alien ctor: `rand()` (direction), `rand(10)` (wander mode); Shadow ctor: `rand(5)` |
| Alien tick, wandering, re-roll timer > 20 | app | `rand(10)`; if 0: `rand() & 3` |
| Alien eats a fish | app | `rand(3)` (sound, only if off cooldown); if `app+0x882`: `rand(3)` count, then per piece `rand(20)` y, `rand(20)` x, effect-shot ctor |
| Player click while an alien is present (input phase) | app | per hit: effect-shot ctor (in `Hit`); on a kill: §4.3 debris; then the shot sound: `rand()`, `rand(10)` |
| Alien death | app + **global** | §4.3 debris; each Bubble costs 4–5 `grand()` (below) |
| DeadAlien, 10 bursts | app + **global** | 16 app + effect-shot ctors, 2 bubbles (global) |
| Countdown == 30 with pet 21 | **global** | `grand()%100`, … (not on plain 1-2) |

Sub-costs:
- **Effect shot** (`0x005436f0`, only created if there are fewer than 20 shots in `+0xd8`): the 3-arg Shot ctor `0x004ec660`. It draws `rand(3)` **only if the stale value at `+0x160` of the freshly allocated block is neither 0 nor 2** (the ctor reads its kind before writing it; this is a bug in the original). It then always draws `rand(200)`. So each effect shot costs **1 or 2 app draws, depending on heap contents**. See open questions.
- **Bubble** (`0x00538ac0`, only if there are fewer than 30 bubbles): `0x004ffdc0` uses **global** `grand()`:
  - `grand() & 3`, redrawn once if it is 3;
  - `grand()%6`;
  - `grand()%22`;
  - `grand()%6`.

  That is 4–5 global draws, and it happens even in Adventure. The earlier observation "global RNG untouched in tank play" only holds without aliens.
- Click shot, Warp, Shot/Warp updates, Coin ctor, DeadAlien ctor, sounds: no RNG.

## 7. Drawing and sound (presentation)

`static` unless noted. Positions are relative to the object's widget top-left `(+0x30, +0x34)`. "Additive" means SexyApp draw mode 1 (`SetDrawMode(1)`): source colour × tint is added to the destination. "Tint" is `SetColorizeImages(1)` with the given RGBA, which multiplies the image.

Image globals were resolved by matching the writes in `WinFish::ExtractLoadingThreadResources` against the `IMAGE_*` id strings.

### 7.1 Alien (`Alien` slot 26 @ `0x004db210` → `Alien::DrawSprite` @ `0x004dae20`), kind 1

- Mirror (art faces left): `vx ≥ 0` → mirrored.
- Sheet `IMAGE_SYLV` (160×160 cels, 10×2). Column = cel `+0x1c0`. Row:
  - 1 while turning (`+0x1b4 ≠ 0`); then mirror = `turn > 0` instead.
  - 0 otherwise.
- **Warp-in** (`+0x158` = w > 0): w ≥ 10 → not drawn. For 1..9, draw stretched, centred in the 160 box: `s = trunc(160 · w/10)`, dest rect `(s/2, s/2, 160 − s, 160 − s)`. It grows as w counts down (w = 9 → 16 px, w = 1 → 144 px).
  - Filtered stretch (`SetFastStretch(!is3D)`) on the 3D renderer, which is what the original uses on modern systems.
  - Sylvester appears on the tick his counter reaches 9.
- **Otherwise** draw the cel normally. While the hit cooldown `+0x200` = c > 0, also draw the same cel again **additive with white at alpha `25·c`**: a white flash that fades over 10 ticks.
- The boss/health-bar branch (`+0x1ec == 0x15` or a boss present) doesn't apply on tank 1.

### 7.2 Warp hole (`Warp`, type 0x21, list `+0xf0`)

- Ctor `0x004ecf00`: widget at **(alien.ix + 30, alien.iy − 40)**, size 100×220, counter `+0x15c = 36`. No RNG.
- `Warp::Update` (`0x004f3e70`): runs only while the board is unpaused. If the counter < 1, remove the warp. Then `counter −= 1`. The warp lives 37 updates (36 → −1).
  - Confirmed: `tank1-2_full_02` shows the counter 35 → 0 alongside the alien's warp 14 → 0.
- Draw (`0x004da690`): frame `f = 17 − counter/2` (int division):
  - `IMAGE_WARPHOLE` (60×220 cels, 17 across) at (20, 0), cel f;
  - then `IMAGE_WARPGLOW` (100×220, 17 across) additive at (0, 0), cel f.

  Out-of-range cels (f ≥ 17 at counter 0 or −1) are skipped by `DrawImageCel` (it ignores column/row outside the sheet).
- Z-order: warps are after coins(sub) and before food (`board.md`), so behind fish and the alien.

### 7.3 Shot (`Shot` slot 26 @ `0x004d9340`)

Nothing is drawn while the start delay `+0x164` is non-zero. Otherwise the base tint is (255,255,255, `r`), where `r` = `+0x168` (255 for click flashes, 50..249 for effect shots).

- **Kind 0, click flash** (80×80): `IMAGE_LASERS` (80×80 cels, 10×6). Laser row pair `p` = 0 for laser < 7, 1 for 7..9, 2 for ≥ 10. Frame `+0x15c` (= age − 1, so 0..13).
  1. Tinted with the laser colour (laser 2 = (50,180,200)) at alpha 100: cel (frame, 2p).
  2. Additive, same colour, alpha `r`: cel (frame, 2p).
  3. Additive, laser-specific highlight (laser 2: white, alpha `r`): cel (frame, 2p + 1).

  Frames ≥ 10 are past the sheet's 10 columns and draw nothing. The flash is visible for its first 10 ticks and lives 15.

  Laser colour table (`0x004d9160`, alpha arg):

  | Laser | RGB |
  |---|---|
  | 2, 11 | (50, 180, 200) |
  | 3, 10 | (255, 0, 0) |
  | 4 | (255, 55, 0) |
  | 5 | (255, 200, 0) |
  | 6 | (125, 255, 90) |
  | 7 | (10, 200, 35) |
  | 8 | (5, 150, 250) |
  | 9 | (160, 0, 255) |
  | 12 | (235, 0, 215) |

  Highlight (`0x004d9280`): white for most levels, (255,255,100) at 4/7, (255,255,0) at 6, (100,255,255) at 10, (180,225,70) at 11, (0,255,0) at 12.
- **Kind 1, gore** (50×30): `IMAGE_CHOMP` (30×30 cels, 5 across), cel `frame` = age/2 − 1, tinted alpha `r`.
- **Kind 2, hit spark**: draws **nothing**. It sets and clears draw modes without drawing an image.
- **Kind 3** (80×80): `IMAGE_EXPLOSION`, additive, cel `frame`.
- **Kind 4** (60×60): `IMAGE_EXPLOSIONSMALL`, additive.
- **Kind 5** (80): `IMAGE_EXPLOSIONTINY`, normal blend.
- **Kind 6** (40): `IMAGE_SMOKETINY`, normal blend.
- **Kind 7** (60): `IMAGE_SMOKESMALL`, normal blend.
- **Kind 8** (40): `IMAGE_SMOKEMISSILE`, normal blend. Other kinds draw nothing.

  Frames past a sheet's column count draw nothing (`DrawImageCel` uses `cel % cols, cel / cols`, then bounds-checks the row).

### 7.4 Dead alien (`DeadAlien` slot 26 @ `0x004dddb0`)

DeadAlien fields: drift x/y velocity `+0x170/+0x178`, fade `+0x190` (f64, starts 1.0), cel `+0x188`, mirrored `+0x158` (byte), kind `+0x1a0`. Positions are x `+0x168` and y `+0x160` (doubles; note the order).

- `Board::AddDeadAlien(trunc(x), trunc(y), cel, kind, mirrored = vx > 0)` (`0x005442c0`, from `0x004fa6db`) passes the alien's **current cel** and facing. The position is the alien's float position, **not** its widget position: the stroke animation can move x/y after the widget was set that tick. Confirmed: corpse position, drift, fade and warp counters replay exactly in `tank1-2_full_01`/`_02`.
- **Update** (`0x004f6050`, after the debris RNG of §4.4):
  - Lifetime `life −= 1`. Once life < 105, `fade −= 0.02` (floored at 0).
  - Every 5th life above 50: sample `0x11d` (`SOUND_EXPLODE`), throttled 3, volume `clamp(lerp(25 → 100 over life−50 ∈ [0,75]), 25, 100) / 100`. It plays only if this corpse has the highest life among the dead aliens (`0x004e54f0`).
  - x velocity decays toward 0 by 0.03 per tick (`vx ≤ 0 → +0.03`, else −0.03). Since it starts at 0 it dithers ±0.03 around 0.
  - y velocity +0.05 per tick up to 2.0.
  - Clamp x to [−10, 490]. (Kinds 5/6 also clamp y; n/a.) Then `x += vx`, `y += vy`. The widget is at trunc.
  - Confirmed in `tank1-2_full_02`: y gains 0.05, 0.10, … up to 2.0 per tick (from y 204 to 407 over the lifetime).
- **Draw:** tint alpha `trunc(fade · 255)`. The kind 1/2 sheet is `IMAGE_SYLV` (through the image-id table, `0x5e0a10[0xbd]`), row 0, column = cel, mirrored per the flag.
  - While life > 115: also draw the same cel **additive white at alpha `(life − 115)·20`, five times over**. This is a bright flash on death that fades in 10 ticks.

### 7.5 Shadows

See `sim/board.md#shadows`. The alien's shadow is type 2.

### 7.6 Sounds (sample ids = manifest `<Sound>` order + 0x105)

| Event | Sample | Throttle | RNG |
|---|---|---|---|
| Warning (countdown 275) | 0x106 `SOUND_AWOOGA` | 3 | – |
| Alien spawned (kinds 1–3, `0x00545928`) | `SOUND_ROAR` (direct, unthrottled) | – | – |
| Click while aliens present (`0x00538ae0`) | 0x144 `ZAP`; if `r & 1`: also 0x145 `ZZAM`; if `r2 % 10 == 0`: 0x132 `RICOCHET` | 3 each | 2 draws (§4.1) |
| Hit (`Alien::Hit`) | 0x126 `HIT` | 3 | – |
| Alien ate a fish (`0x00538560`) | `r % 3 == 0` → 0x114 `CHOMP2`, else 0x113 `CHOMP` (`neg/sbb` + 0x114) | 3 (on 0x113) | 1 draw |
| Death (`0x4fa01e`, `0x4fa03e`) | 0x11d `EXPLODE`, 0x11e `EXPLOSION1` | 3 each | – |
| Dead-alien rumble | 0x11d at the volume above | 3 | – |
| Diamond collected (`0x00538470`) | 0x116 `DIAMOND` | 3 | – |

Correction to §3.2: the eat sound picks `CHOMP2` when `r % 3 == 0` and `CHOMP` otherwise.

The `0x00538230(id, gap, volume)` helper plays a sample only if the board clock is ≥ `gap` past that sample's last play (`board+0x1a4 + 4·(id − 0x106)`), and records the play. The sim already models this throttle for RNG purposes.

### 7.7 Music

- Warning (countdown 275): `PlayAlienMusic` (`0x0054c480`): fade the tank song out (speed 0.008/update) and start `Alien.mo3` from the start, fading in at 0.002/update.
- Last alien gone (`0x00539c90`, from death): `PlayTankMusic(1)`: back to the tank song.
- Fade (`BassMusicInterface` slot 19 @ `0x004997d0`) is linear: `volume += speed` per update, clamped to [0, max]. A song that reaches 0 while fading out is stopped.

### 7.8 Danger reticles

Board draw (`0x0053ffa2`..): while the countdown is in 1..225 and `board+0x178` is non-zero, the board draws `IMAGE_CROSSHAIR` (5 cels) additively:
- at (A.x + 40, 320), cel `(board+0x444 / 4) % 5`;
- for wave kinds 9..12 only, a second one at B.

The draw code **writes A.y = 280** (`+0x2d0 = 0x118`), and for kind 11 B.y too, so with `+0x178` set the alien spawns at y 280. It also plays sample 0x13c `SONAR` at countdown 225.

`board+0x178` is not set on 1-2: in `tank1-2_full_02` both aliens spawned at the drawn A.y (297 and 116), not 280. TODO: what sets it (likely a pet or a later level), and moving this gameplay write out of the draw path when it's implemented.

## Note: the 1-2 "DANGER!" dialog

On tank 1-2 (game not completed), when the countdown reaches **276**, the board raises hint dialog 0x0e: "DANGER!" on the first wave and "BATTLE TIP" on the second, each once (`framework/dialog.md#board-dialogs`, `0x005483e0`). That pauses the board; the countdown resumes when it is dismissed. Confirmed in `tank1-2_full_01` and `_02`. Modelled as `Dialog::AlienWarning`; objects skip the tick that raises it.

## Deviations

Implementation artifacts we deliberately don't reproduce (CLAUDE.md, "Bugs"):

- **Effect-shot start delay.** `Shot::Shot` (`0x004ec660`) tests its kind field `+0x160` *before* assigning it. If the recycled heap block still holds a kind other than 0/2, it draws `rand(3)` as a start delay (`+0x164`). The result depends on allocator history, so it isn't reproducible even in the original. We never draw it; sparks start immediately. Effect: the dead alien's debris sparks and any later RNG consumers differ from a given run of the original. The harness re-syncs the app RNG on every tick that creates an effect shot (debris, hit sparks, gore), and ignores the shot list while a delayed spark is alive (it outlives ours by its delay).
- **Prey and eat order by object address** (§3.1, §3.2). We iterate guppies in list order. This differs only on exact distance ties or two fish inside the eat box on the same tick.

## Open questions / TODO

- Heap-dependent behaviour is resolved by policy (see Deviations). Measured: every *hit* effect shot in `tank1-2_full_01` cost 1 draw, while the debris sparks cost 1–2.
- Whether an alien spawned inside `Board::Update` gets its own `Update` in the same tick (WidgetManager iteration).
- Meaning of GameObject `+0xac`/`+0x120` (prey filter), board `+0x2c4`, `+0x340`, `app+0x882`.
- Whether `Coin` kind 4 from this path uses the "rising" behaviour (ctor arg −1.0).
- Confirm the 10-tick hit cooldown dynamically (it limits damage to 1 hit per 280 ms).
- Kind 2 (1-3) presumably differs only in divisor and health; check.
