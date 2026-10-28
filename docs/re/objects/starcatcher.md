# Starcatcher (`Sexy::Penta`, type `0x08`)

Status: `draft (static)`. Nothing here has been checked against a capture yet. Every section is `static` until a 2-2 recording is replayed (see [Recording](#recording)).

Class `Sexy::Penta`, vtable `0x00596ee4`, board list `+0xd0`, object size **`0x1b0`** (the `operator new` size in both spawn routines). It is **not a `Fish`**: it derives from `GameObject` directly and has its own update (slot 22 @ `0x004f94e0`) and draw (slot 26 @ `0x004e3030`). It shares a few small slots with `Sexy::Grubber`.

In short: the Starcatcher drops in from the top of the tank and **sinks to the floor** (gravity 0.4 per tick, floor at y 359). It then crawls left and right along the bottom; it only ever steers in x. Below hunger 900 it homes in on the nearest **star** (coin kinds 3 and 10, plus 0x12). Every star it touches is removed (not banked), feeds it +900 hunger (capped at 1300), and makes it drop a **kind-5 coin worth $200** with the `BUY` sound. It dies (with a corpse) on the tick its hunger reaches 0. On 2-2 its death shows a chain of three hint messages. In the normal game it consumes **no RNG** apart from its swim-mode reroll and its splash-in bubbles.

**Slot numbering** is 0-based, as in `carnivore.md` (Ghidra's `vfunctionN` = slot `N − 1`).

**Offsets** are raw `this`-relative. How the Ghidra names convert:
- `GameObject_data` starts at `+0x88`, so `GameObject_data.offset_0xN` = `+0x88 + N` (hunger `offset_0x14` = `+0x9c`; `offset_0x24` = `+0xac`).
- `WidgetContainer_data` starts at `+0x4`, so `WidgetContainer_data.offset_0x2c`/`0x30` = widget x/y at `+0x30`/`+0x34`.
- `Penta_data` starts at `+0x154`, so `Penta_data.offset_0x4` = x at `+0x158` and `offset_0xc` = y at `+0x160`. The ctor's `FST [ESI+0x158]` and BuyStarcatcher's `FSTP [EDI+0x160]` confirm this.
- Ghidra's recovered `Penta` struct is only `0x168` bytes, so `this + 1` is `+0x168` (vx). `this[1].WidgetContainer_data.offset_0xN` is therefore `+0x168 + 0x4 + N` = **`+0x16c + N`**. This was checked against the disassembly of `Penta::Update`: `offset_0x24` (the swim mode) is `[ESI+0x190]`, `offset_0x14` (the splash timer) is `[ESI+0x180]`, `offset_0x1c` (the target vx) is `[ESI+0x188]`, and `offset_0x28`/`0x2c` are `[ESI+0x194]`/`[ESI+0x198]`.
- The sync/serialise slot 80 (`0x004d8c20`) lists the class's own fields in order: x, y, vx, vy, divisor, splash timer, target vx, mode, chase timer, reroll timer, anim counter, cel, eat counter, `+0x1a8`. That matches the table below.

RNG: `r` = app stream `MTRand::NextNoAssert` @ `0x0040aeb0`. `grand` = global `Sexy::Rand` @ `0x0040f9e0`. The Starcatcher reaches the global stream only through `Board::AddBubble` `0x00538ac0`.

Confidence: per section below.


Captures: `tank2-2_full_01` (2-2, star potions turning large guppies into star guppies, stars left uncollected) and `tank2-2_full_02` (the next chunk: a Starcatcher bought, five stars eaten for $200 coins, then eaten by the wave's alien), replayed exactly from the game's own demo. No starvation and no Virtual Tank behaviour was captured. `traced` where a section says so, else `static`.

## Slot overrides

Compared with `GameObject` (vtable `0x00596844`):

| Slot | Penta | Role |
|---|---|---|
| 0 | `0x004eff20` | destructor |
| 22 | `0x004f94e0` | **Update** |
| 26 | `0x004e3030` | **Draw** |
| 54 | `0x004d7dc0` (shared with Grubber) | mouse-down: only a click with a negative click count (right button) does anything; it forwards `(wx + mx, wy + my)` to `0x0053bfb0`. Never reached (see Clicks) |
| 58 | `0x004d8c10` (shared with Grubber) | trivial |
| 70 | `0x004ec070` | census counter: increments the int at `arg + 4` (census slot 1; the carnivore's is slot 0) |
| 71 | `0x004ec060` | returns 4 |
| 75 | `0x004f3ee0` | "eaten": **remove** (`0x004f3ad0`) with `true` |
| 76 | `0x004d8e40` (shared with Grubber) | set position: widget x/y and the double x/y |
| 77 | `0x004e3140` | **eat** (argument = the star) |
| 78 | `0x004d9000` (shared with Grubber) | icon animation: `+0x130 = (+0x12c / 2) % 10` |
| 79 | `0x004e3060` | icon draw for the UI (row 0) |
| 80 | `0x004d8c20` | sync / serialise fields |

Helpers that aren't slots: behaviour step `0x004f9460`, star chase `0x004eff50`, find star `0x004ec180`, bite `0x004ec2f0`, star filter `0x004d6bf0`, drop `0x004d8e70`, die `0x004f3c60`, remove `0x004f3ad0`, animation `0x004d8f10`, init `0x004d8d00`.

`static`.

## Fields

| Offset | Type | Meaning | Initial value |
|---|---|---|---|
| `+0x30`, `+0x34` | int | widget x / y | `trunc(x)`, `trunc(y)` |
| `+0x38`, `+0x3c` | int | width, height | 80, 80 |
| `+0x41` | byte | (GameObject flag) | 0 |
| `+0x55` | byte | widget mouse-visible | byte `0x005e8f14` (0; no writer known, see `carnivore.md`) |
| `+0x8c` | int | type | **8** |
| `+0x94` | ptr | shadow | set by the shadow ctor |
| `+0x98` | ptr | attached missile | 0 |
| `+0x9c` | int | **hunger** | **`r % 200 + 900`** (900..1099) |
| `+0xa0` | int | colour fade (hungry look) | 0 |
| `+0xa4` | int | hunger-warning threshold | 300 (GameObject ctor) |
| `+0xac` | int | Virtual Tank state (`< 0` in normal play) | −1 |
| `+0xf0`, `+0xf6`, `+0xf8` | | food-mode switch, divisor-override byte, its timer | 0 (`+0xf8` is set to 100 when a star is within 100 px; it has no effect while `+0xf6` is 0) |
| `+0x104` | byte | "big slurp" flag for the eat sound | 0 |
| `+0x158` | double | **x** | spawn x |
| `+0x160` | double | **y** | `r % 5 + 360`, then overwritten with **65.0** by BuyStarcatcher |
| `+0x168` | double | **vx** | 0.0 |
| `+0x170` | double | **vy** | 0.0 |
| `+0x178` | double | **speed divisor** | **2.7 / 2.5 / 2.6** for `r % 3` = 0 / 1 / 2 |
| `+0x180` | int | splash timer (bubbles; the ceiling clamp is skipped while > 0) | **45** |
| `+0x188` | double | target vx of the swim mode | 0.0 |
| `+0x190` | int | **swim mode** 0..9 | **`r % 10`** |
| `+0x194` | int | chase-steer timer | **40** |
| `+0x198` | int | mode-reroll timer | 0 |
| `+0x19c` | int | animation counter | 0 |
| `+0x1a0` | int | cel 0..9 | 0 |
| `+0x1a4` | int | eat counter (Virtual Tank drop rhythm) | 0 |
| `+0x1a8` | int | ? (serialised; not read by the update) | 1 |

Positions, velocities and the divisor are `double` (x87). Everything else is `int`. Double → int conversions use `_ftol2` (truncation toward zero).

`traced` by `tank_2_2_starcatcher` (`tank2-2_full_02`) (every field in `starcatcher_json`).

## Construction (`Penta::Penta(x)` @ `0x004ec080`)

1. `GameObject::GameObject` (`0x004e9cb0`). No RNG.
2. vtable = Penta.
3. **Init** `0x004d8d00`, in order:
   1. `+0x41 = 0`; type = 8.
   2. **R1:** `y = (double)(r % 5 + 360)` (360..364); widget y = `trunc(y)`. (An unsigned→double fixup exists in the code but is unreachable.)
   3. vx = vy = target vx = 0.0; size 80×80.
   4. **R2:** divisor = 2.7 if `r % 3 == 0`, 2.5 if 1, 2.6 if 2.
   5. **R3:** hunger = `r % 200 + 900`.
   6. **R4:** `m = r`. Then anim counter, cel, eat counter and reroll timer = 0; chase timer `+0x194 = 40`; `+0x1a8 = 1`; mode `+0x190 = m % 10`; `+0x55` = byte `0x005e8f14`; splash timer `+0x180 = 45`.
4. `x = (double)arg`; widget x = `trunc(x)`. The x is set **after** Init.

**4 app draws** (y, divisor, hunger, mode). No Virtual Tank colour draw (unlike the Oscar).

`traced` by `tank_2_2_starcatcher` (`tank2-2_full_02`).

## Spawn (`Board::BuyStarcatcher` @ `0x00545260`)

Callers: shop slot 6 (`0x0054964b`, in `0x00549380`) and the debug key `'4'` (`0x00549e21`).

1. Allocate `0x1b0` bytes, then **RNG** `x = r % 520 + 20` (20..539). This draw comes **before** the ctor's draws.
2. `new Penta(x)`: 4 draws (above).
3. y (`+0x160`) = **65.0** (double at `0x00596708`). Widget y (`+0x34`) = **65** (`0x41`). The ctor's y (360..364) is discarded, but its draw is still consumed.
4. `AddGameObject(obj, 1)` (`0x00542ee0`) appends it to `+0xd0`. Then the widget manager adds it (board `+0x10`, slot 3).
5. Max-starcatchers statistic: `board+0x494` = max(itself, length of `+0xd0`).
6. Shadow `0x00544800`: type 8 → shadow kind **1** (shared with type 9). `0x00544370` creates a `Shadow` only if `+0xe4` holds fewer than 40 shadows. **`Shadow::Shadow` draws once** (shadow y = `410 + r % 5`, `fish-guppy.md#spawn`).
7. `RebuildZOrder`.

**6 app draws:** x, R1–R4, shadow.

**Shop purchase** (slot 6, `sim/shop-bar.md`): `TrySpend(750)` → BuyStarcatcher → reveal slots 10 and 11 (`0x005409b0`, no RNG) → `SOUND_GROW` `0x122` (gap 3, no RNG) → **PlaySplash** `0x00538360` (1 app draw `r % 3`, `SPLASH`/`2`/`3`). **A purchase costs 7 app draws on its tick.**

**Re-creation** `0x00545360(x)` (DeadFish kind-8 revival, `pet-angie.md`): the same as BuyStarcatcher but without the x draw and without the y override. The Starcatcher starts at the ctor's y (360..364), which is already on the floor. That's 4 + 1 (shadow) draws.

**Splash-in:** a bought Starcatcher starts at y 65 with vy 0. Gravity (+0.4 per tick to vy, applied before integrating) gives `y ≈ 65 + (0.2 / div)·n(n+1)` after n ticks. It hits the floor at y 359 after about **60 ticks** (≈ 1.7 s; 60–63 depending on the divisor), at vy ≈ 24, which the floor clamp zeroes. During the first 45 ticks it emits bubbles (see Update step 6).

`traced` by `tank_2_2_starcatcher` (`tank2-2_full_02`) (the 7 draws, the drop-in from y 65). Re-creation is `static`.

## Per-tick update (slot 22 @ `0x004f94e0`)

0. Return if the board is missing or paused (`board+0x94`).
1. `PreUpdate` `0x004f22c0` (generic timers).
2. **Behaviour** `busy = 0x004f9460()`:
   1. Colour-fade step `0x004d6c50` (as for guppies).
   2. If there are no aliens (`+0xb8` and `+0xf4` empty): `0x004d6ef0` (`carnivore.md#carnivore-hunger`). While `+0x11c == −1`, hunger −= 1. At hunger 304 the turning-hungry fade starts. Hunger is clamped at −1000.
   3. **Starving** (`0x004d69d0`: `+0xac < 0` and hunger ≤ 0; Virtual Tank uses its own threshold): **die** (`0x004f3c60(1)`, below), return false.
   4. Else if **hunger < 900**: return the **star chase** (`0x004eff50`, below).
   5. Else return false.
3. **Swim mode**, only when `busy` is false. It sets the target vx `+0x188` from the mode:

   | Mode | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
   |---|---|---|---|---|---|---|---|---|---|---|
   | target vx | 0.0 | −0.5 | 0.5 | −1.0 | 1.0 | 0.0 | 0.0 | −2.5 | 2.5 | unchanged |

   Then: if `target < vx`, `vx −= 0.1`. After that, as a separate check: if `vx < target`, `vx += 0.1`. Both are double compares with no epsilon. When vx is within 0.1 of the target, the two steps cancel, so vx is left near its old value (up to rounding).
4. **Timers and reroll** (every tick, chasing or not): chase timer `+0x194 += 1`; reroll timer `+0x198 += 1`. If the reroll timer is **> 20**, **or x ≤ 10.0, or x ≥ 540.0**: reroll timer = 0, then **RNG** `r % 10`; on 0, **RNG** `mode = r % 9` (0..8). Mode 9 only ever comes from the ctor.
   - Because x is clamped to [10, 550], a Starcatcher resting against the left wall (x = 10), or at x ≥ 540, rerolls **every tick**: one draw per tick, plus another with probability 1/10.
5. **Clamps:** x ≤ 550.0; x ≥ 10.0. **If y > 359.0: y = 359.0 and vy = 0.0** (the floor).
6. **Splash timer** `+0x180`:
   - **≥ 1:** `n = 8` if the timer > 35, else 5. **RNG** `r % n`; on 0: **RNG** `a`, **RNG** `b`, then `AddBubble(wx + 55 − a % 60, wy + 55 − b % 60)` (global-stream draws inside). Then timer −= 1.
   - **< 1:** if y < 95.0: y = 95.0 (the ceiling).
7. **Edge steering:** if x > 535.0 and vx > 0.1: `vx −= 0.1`. If x < 15.0 and vx < −0.1: `vx += 0.1`.
8. **Gravity:** if y < 359.0: `vy += 0.4`. There is no cap on vy.
9. **Animation** `0x004d8f10` (the `+0xfc` cut-out pre-pass `0x004d6c90` is never set):
   - vx ≥ 1.0: counter = (counter + 1) mod 20; cel = counter / 2.
   - −1.0 < vx < 1.0: counter = (counter + 1) mod 40 if vx > 0, else (counter − 1) mod 40 (wrapped into 0..39); cel = counter / 4.
   - vx ≤ −1.0: counter = (counter − 1) mod 20 (wrapped into 0..19); cel = counter / 2.
   - cel → `+0x1a0`. If a missile is attached, `0x004d7020` updates it.
10. **Integrate:** div = `+0x178` (if byte `+0xf6` is set: 0.8 while `+0xf8 ≠ 0`, else 0.3). `x += vx / div`, `y += vy / div`.
11. Widget position (slot 42) = `(trunc(x), trunc(y))`.

**Death tick:** when step 2.3 kills it, the update **carries on** for the removed object: the swim mode, timers, reroll (a possible RNG draw), clamps, splash and integrate all run. Only the position of an object that no longer exists changes, but **the reroll draw on that tick is real**.

So the Starcatcher lives on the floor: the clamp holds it at y 359 with vy 0. Only the swim mode and the chase steer vx, and nothing steers it upward. A Starcatcher catches only stars that fall to it. With vx 0 (modes 0/5/6) it still animates backwards: −1 < 0 < 1 and "vx > 0" is false.

`+0xd0` comes before the guppies in `RebuildZOrder` (`sim/board.md`). So **Starcatchers update before guppies** (and before food). Coins update last. A star dropped by a guppy this tick is first seen by the Starcatcher next tick.

`traced` by `tank_2_2_starcatcher` (`tank2-2_full_02`). The death-tick continuation is `static`.

### Star chase (`0x004eff50`)

1. `star = FindStar()` (`0x004ec180`).
2. If chase timer `+0x194 < 5`: skip to step 5.
3. Else if there is no star: return false (no bite).
4. Else steer x only: `+0x194 = 0`; `fx = x + 40.0` (double); `tx = star.wx` (int, compared as double). First matching row, **peckish** = hunger > 300, **hungry** = hunger ≤ 300:

   | Condition | peckish | hungry | only if |
   |---|---|---|---|
   | `fx > tx + 46` | `vx −= 1.8` | `vx −= 2.5` | vx > −cap_far |
   | `fx < tx + 26` | `vx += 1.8` | `vx += 2.5` | vx < cap_far |
   | `fx > tx + 36` | `vx −= 1.0` | `vx −= 1.2` | vx > −cap_near |
   | `fx < tx + 36` | `vx += 1.0` | `vx += 1.2` | vx < cap_near |
   | cap_far / cap_near | 4.5 / 1.5 | 6.0 / 2.5 | |

   Rows don't fall through: if a row's condition holds but its cap check fails, nothing changes this tick. `fx == tx + 36` does nothing. It aims its centre x + 40 at the star's centre wx + 36. There's no y steering.
5. If there is a star: **bite** (`0x004ec2f0`).
6. Return "chasing" iff there is a star. While chasing, step 3 (swim-mode damping toward the target vx) doesn't run, so vx keeps whatever the steering gave it.

The timer starts at 40, so the first chase tick steers at once. After that it steers every 5th tick. **No RNG.**

`traced` by `tank_2_2_starcatcher` (`tank2-2_full_02`).

### Find star (`0x004ec180`)

If `+0xf0 ≠ 0`: the food finder `0x004ea010` (never for a Starcatcher).

Otherwise walk the **coin list `board+0xa8` in list order**, with `best = 100 000 000`. For each coin `c` that passes the **star filter**:
- `dx = trunc((c.wx + 36) − (x + 40.0))`, `dy = trunc((c.wy + 36) − (y + 40.0))`, `d = dx² + dy²` (int).
- If `d < best`: take it (strict, so the first in list order wins ties).

If `best < 10000`: `+0xf8 = 100` (no effect, see Fields). Return the star or none. **No RNG.**

**Star filter** (`0x004d6bf0`): the coin isn't collected (`+0x198 == 0`), and
- normal game (`app+0x880 == 0`): kind **3** (star), **10** (rising star, `pet-nimbus.md`) or **0x12**;
- with `app+0x880` set (Virtual Tank, per `carnivore.md`): kind **1** or **2** (silver, gold). The death message "The purple guys eat coins!" belongs to this mode.

Only list `+0xa8` is searched. `AddGameObject` (`0x00543182`) puts every coin with subtype `+0x1a0 ≤ 1` there, except kinds 0xf/0x10.

`traced` by `tank_2_2_starcatcher` (`tank2-2_full_02`) for kind 3 (the Virtual Tank filter is `static`).

### Bite (`0x004ec2f0`)

If `+0xf0 ≠ 0`: food bites (`0x004ea2a0`), never. Otherwise `cx = x + 40.0`, `cy = y + 40.0`. Walk `board+0xa8` in order. The **first** coin with

`c.wx + 16 < cx < c.wx + 56` and `c.wy + 16 < cy < c.wy + 66` (strict, double vs int)

that passes the star filter is eaten:
1. `0x004d6cc0` (no-op);
2. the coin's slot 75 (`0x004d5510`) removes it: WM, app registry, its board list. **Not banked: no money, no collect sound.** (The bomb branch there is for kind 0x11 only.)
3. this Starcatcher's **eat** (slot 77) with the coin.

Then return: **at most one star per Starcatcher per tick.** The filter doesn't look at a coin's expiry, so a star that has begun to expire can still be eaten. On the floor (y = 359, cy = 399), a star resting at y 370 is inside the y range (333 < wy < 383). **No RNG.**

`traced` by `tank_2_2_starcatcher` (`tank2-2_full_02`).

### Eat (slot 77 @ `0x004e3140`)

1. `was_hungry_look = 0x004d6f30()`.
2. `0x004d6a30(0)` (`+0xf8 = 0`, `board+0x4ee = 1`; Virtual Tank parts don't apply). If `+0xac ≥ 0`: `0x004d6f90` (Virtual Tank).
3. **hunger += 900; if > 1300, hunger = 1300.**
4. **Drop** `0x004d8e70`:
   - eat counter `+0x1a4 += 1`;
   - outside the Virtual Tank (`app+0x87c ≠ 5`), and in the Virtual Tank once the counter is > 2: counter = 0. The Virtual Tank also needs `0x004d70b0` and `board+0x500`;
   - if `0x004d6bd0` allows it (always outside the Virtual Tank): **`AddCoin(wx + 5, wy − 12, kind 5, 0, −1.0, 0)`**, return true.
5. If it dropped: sample **`0x111` (`BUY`)**, gap 3, pitch 1.0. **No RNG.**
   Otherwise (Virtual Tank only): slurp `0x005384c0(byte +0x104)`. If `SLURP` `0x138` is off its 3-tick cooldown: **RNG** `r % 5`: 0 → `SLURP3` `0x13a`, 1 → `SLURP2` `0x139`, else `SLURP` `0x138`. (The three sample globals `0x005e8de0`/`0x005e8bdc`/`0x005e8de4` are loaded from the names `SOUND_SLURP3`/`SOUND_SLURP2`/`SOUND_SLURP` at `0x0050dd72`..`0x0050dde6`; the same function as the guppy slurp in `food.md`.)
6. `0x004e13d0(was_hungry_look)`: fade back to the fed colours.

So in Adventure, **every star eaten yields one kind-5 coin and the `BUY` sound, with no RNG.** There's no growth and no gore.

**The kind-5 coin** (`coin.md`: "pearl", **$200**; `pet-amp.md` drops the same kind): list `+0xa8` (subtype 0). In `Coin::Update` it is a **rising** kind (5, 8–14, 0x12 rise until the "risen" byte `+0x168` is set). Per tick, y moves by −8.0 while y ≥ 150, −2.5 for 135..150, −1.5 for 130..135, −1.0 for 125..130 and −0.5 for 120..125. Below 120 it sets risen and then falls and rests like any coin (1.5 per tick to y 370; `pet-nimbus.md`). From the floor (spawn y 347) it climbs to y 150 in about 25 ticks and creeps up to just under 120 in about 25 more. It then falls back to 370 in about 167 ticks. How it's drawn hasn't been checked.

`traced` by `tank_2_2_starcatcher` (`tank2-2_full_02`) (hunger, the kind-5 coin, `BUY`). The Virtual Tank slurp is `static`.

## Death

### Starvation (`0x004f3c60`, argument 1)

1. Die sound `0x00538610(8)`: `DIE` `0x117`, gap 3, **pitch +4** (type 8). No RNG.
2. **Remove** `0x004f3ad0(false)` (below). The shadow is kept.
3. `AddDeadFish(trunc x, trunc y, vx, vy = 0.0, divisor, kind 8, facing right = vx > 0, shadow)`. The dead fish gets **vy 0.0**, not the Starcatcher's vy (a `FLDZ` left on the FPU stack is stored there).

The `DeadFish` of **kind 8** (`fish-guppy.md#dead-fish`, update `0x004f64e0`):
- AddDeadFish's start-vy rule turns vy 0 into −2 (y ≥ 115), so the corpse hops up, then settles at +0.05 per tick toward 2.0.
- Cel: kinds 8/9 use only the first rule: 9 − (life − 106)/2 while life > 105, else **9**.
- Life counts down while y > **365** (or y > 370, or life > 105). The y clamp is at most **370** (kinds 8/9 keep the default), and at least 85.
- Draw (`0x004de030`): `IMAGE_STARCATCHER` src (cel·80, **160**, 80, 80) = **row 2**, *not* mirrored, colourised white at the fade alpha, sinking as for guppies.
- Revival (turn counter brought into 0..10, e.g. by a reviving pet): sample `0x125`, then `0x00545360` re-creates a Starcatcher (4 + 1 draws).

**No RNG** in the death itself.

`static`.

### Remove (`0x004f3ad0`)

Used by starvation (`false`), by being eaten (slot 75, `true`) and by Shrapnel's bomb (`pet-shrapnel.md`, which calls the die `0x004f3c60(1)`).

1. Detach a missile at `+0x98`. Remove from the WM, the app registry and `board+0xd0` (`0x00541d90`). If the argument is true and there is a shadow: delete the shadow.
2. **`board+0x490 += 1`** (starcatchers lost).
3. Messages, **only while the Starcatcher is starving** (`0x004d69d0` again: hunger ≤ 0). So a well-fed Starcatcher eaten by an alien shows nothing.
   - `app+0x880 == 0`, and `0x00537bb0(2, 2)` (tank 2, level 2, `profile+0x59` clear). `ShowMessage(text, flash 0, id)` in this order:
     1. if one-time byte `0x23` (`board+0x4d3`) is set: "Hint: Star potions allow guppies to produce stars!", id `0x24`;
     2. if byte `0x22` (`board+0x4d2`) is set: "Hint: Starcatchers need stars to stay alive!", id `0x23`;
     3. "Warning! Your starcatcher has died!", id `0x22`.

     As with the carnivore chain (`carnivore.md#carnivore-remove`), the 1st death on 2-2 shows the warning, the 2nd the "need stars" hint, the 3rd the "star potions" hint, and later deaths show nothing new.
   - `app+0x880 ≠ 0`: "The purple guys eat coins!", id `0x22`.

`static`.

### Eaten by an alien

Sylvester's prey filter accepts types 5..10 always (`alien-sylvester.md#31-nearest-prey-0x004e8b60`), so **Starcatchers are prey**. The eat box is 45/65 against its 80×80 centre. The alien calls slot 75 → remove(`true`): no corpse, shadow deleted, `board+0x490 += 1`, messages only if it was starving. `+0xd0` also counts for "alien has prey" and the empty-tank check (`0x005392a0`).

With aliens present, hunger is frozen (step 2.2 is skipped) but the chase and bite still run. Guppies don't drop coins while aliens are in the tank, so in practice there are no new stars then.

`traced` by `tank_2_2_starcatcher` (`tank2-2_full_02`) (the alien picked it as prey and it left without a corpse).

## Interactions with other objects

- **Pets:** Stinky, Clyde and Nimbus skip kinds 3/10 while `+0xd0` is non-empty (`pet-stinky.md`, `pet-clyde.md`, `pet-nimbus.md`), so pets leave stars to the Starcatcher.
- **Food:** never looked at (`+0xf0` = 0), so pellets and potions pass through.
- **Player clicks on stars** compete with the Starcatcher. A clicked star is "collected" (`+0x198`), and the filter then skips it.

`static`.

## Clicks

`+0x55` = byte `0x005e8f14` (0), so the widget manager doesn't deliver mouse events. A click on a Starcatcher goes through to whatever is below it (the board's food drop). Even if one arrived, slot 54 reacts only to a negative click count (right button), forwarding the point to `0x0053bfb0`. `+0xd0` isn't among the lists `0x00539ad0` toggles when aliens come and go (that function touches `+0xa0`, `+0xb4`, `+0xc0`).

`static` (`+0x55` as "mouse visible" is `guess`, as in `carnivore.md`).

## Drawing (slot 26 @ `0x004e3030`)

1. `0x004d6ae0`: if global `0x005e89cd` is set, `0x00538060` (an unknown debug overlay).
2. **Body** `0x004e2e30`. The image is **`IMAGE_STARCATCHER`** (global `0x005e8e04`, loaded by name at `0x0050c335`; `resources.xml`: 10 cols × 3 rows of 80×80; image id **`0xe4`**):
   - Normal: row = 1 if the hungry look (`0x004d6f30`) holds, else 0. Src (**cel `+0x1a0`·80, row·80, 80, 80**), **never mirrored**: direction shows only as the cel order (step 9 runs the cycle backwards when moving left). Steps:
     - The `+0xfc` cut-out pre-pass (`0x004d6e70`) can suppress the draw; not used.
     - Colour setup `0x004d6db0`, then the draw.
     - Fade overlay: while `+0xa0 ≠ 0`, row 1 is drawn again at alpha `fade·255/5`.
     - The missile overlay `0x004d7040`, if one is attached.
     - The hungry marker: if `0x004d6980(500)` holds (hunger < 500 and `board+0x178`), `IMAGE_MISCITEMS` cel 2 at (0, −10).
   - "Zombie Mode" (`0x005e89ce`): src cel 4 (hungry) or 9, row 2 (`DrawImageMirror_SrcRect` with flag 0), then the same missile/marker overlays.
3. If `+0xc4 ≠ 0`: `0x004e1400(g, 0)`, a text label gated by `board+0x4fc` (probably a Virtual Tank name tag).

Row 0 = swim, row 1 = hungry swim, row 2 = death (DeadFish kind 8) and the Zombie cels.

**Slot 79** `0x004e3060` (UI icon): translate by style (0: (20, 17), 1: (5, 17), 2: (15, −5), 3: (5, −1), 4: (−40, 0)), then `IMAGE_STARCATCHER` src (`+0x130`·80, 0, 80, 80), colourised.

`static`.

## Stage-3 guppy coin (the star)

A large guppy that eats a star potion becomes stage 3 (`food.md`). `Fish::UpdateCoin` (`0x004f1400`) picks the coin kind like this:
- `+0x124 == 0` (pet-linked): kind 7; `+0x124 == 5`: kind 7 from stage 2.
- Otherwise, if `+0x22a` (gold-capable) is set: stage 1 → 4, 2 → 6, 4 → 7, any other (including 3) → 2. `Fish::Init` clears `+0x22a`.
- Otherwise (a plain guppy): **kind = stage** when stage > 0. So **stage 3 drops kind 3, a star, worth $40** (`coin.md` value table), and **stage 4 drops kind 4 (diamond, $200)**.

It is dropped at `(wx + 5, wy + 10)` with the same −1.0 argument and the guppy's usual interval `+0x224`. Kind 3 is **not** a rising kind (only 5, 8–14 and 0x12 rise), so a star **falls** at 1.5 per tick, rests on the floor (y 370) for the usual 20 ticks, then expires over 5. Its cel is the "other kinds" rule `(anim / 2) % 10`. Clicked, it plays the coin sound (`0x005383d0`, 1 draw if off cooldown) and banks $40.

`static`.

## RNG summary

| When | Stream | Draws, in order |
|---|---|---|
| Purchase tick | app | x `%520`, y `%5`, divisor `%3`, hunger `%200`, mode `%10`, shadow `%5`, splash `%3` (7) |
| Every tick the reroll fires (timer > 20, or x ≤ 10 / x ≥ 540) | app | `%10`; on 0, `%9` |
| Splash timer ≥ 1 (first 45 ticks) | app (+ global) | `% n` (n = 8 while > 35, else 5); on 0: `%60` x, `%60` y, then AddBubble (global-stream draws) |
| Eat (Adventure) | – | none (the `BUY` sound draws nothing) |
| Eat (Virtual Tank, no drop) | app | slurp `%5` if off cooldown |
| Hunger, chase, find, bite, drop, death, remove | – | none |
| Re-creation (`0x00545360`) | app | y, divisor, hunger, mode, shadow (5) |

Within its update the order is: behaviour (no draws in Adventure), then the reroll, then the splash bubbles. The reroll still runs on the tick it starves (see Per-tick update).

`static`.

## Open questions

- Everything is `static`. See [Recording](#recording).
- Hunger timeline (static arithmetic): it starts at 900–1099, so the first chase begins 1–200 ticks after purchase. A meal caps at 1300 (400 ticks of no chasing). An unfed Starcatcher dies 900–1099 ticks (25–31 s) after purchase. Stars from one stage-3 guppy come every 150–349 ticks, so one star guppy can keep a Starcatcher alive (+900 per star), and each star is turned into $200 instead of $40.
- Coin kind 0x12 passes the star filter (and is a rising kind worth $15). Who drops kind 0x12 isn't known yet.
- `app+0x880`: taken as "Virtual Tank" from `carnivore.md`/`alien-sylvester.md`. In that mode the Starcatcher eats silver and gold coins, and its death message is "The purple guys eat coins!". This hasn't been verified.
- How the kind-5 coin is drawn (diamond art? pearl art?) and its collect sound (`coin.md` says pearls play a fixed sample).
- The swim-mode damping (step 3) runs both checks back to back. Near the target, vx can stay a hair off the target forever because of rounding. Worth confirming in a trace.
- `+0x1a8` (= 1) is serialised but no reader has been found.
- Slot 54's right-click path `0x0053bfb0` (also open in `pet-clyde.md`).
- `0x004e1400` label (`+0xc4`, `board+0x4fc`).

## Recording

A 2-2 recording (where shop slot 6 is mapped) must exercise:
1. **Buying a Starcatcher:** 7 draws on the purchase tick, the drop from y 65 with gravity, the 45 splash-bubble ticks, and the landing at y 359.
2. **Star guppies:** buy star potions (slot 5) and drop them on **large** guppies (stage 2 → 3; also one on a small/medium guppy for the explosion). Then wait for a stage-3 guppy to drop a **star** (kind 3, falls).
3. **The Starcatcher eating stars:** hunger below 900, the chase steering every 5th tick, the bite (star removed, not banked), hunger +900 capped at 1300, the **kind-5 coin** (rises, then falls; click it for $200) and the `BUY` sound. Ideally include two stars in reach at once (list-order tie rules) and one star clicked away by the player.
4. **Starving:** let one Starcatcher run out of stars: hunger 0 → `DIE` (pitch +4), a kind-8 corpse (row 2), and "Warning! Your starcatcher has died!". A second and a third death would cover the hint chain.
5. Optionally, a Starcatcher reaching a wall (x ≤ 10 or ≥ 540) to show the reroll draw every tick, and an alien wave eating one (no message when fed).

`static` (what to record is derived from the static reading above).
