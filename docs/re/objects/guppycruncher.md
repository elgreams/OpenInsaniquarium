# Guppycruncher (`Sexy::Grubber`, type `0x09`)

Class `Sexy::Grubber`, vtable `0x0059698c`, board list `+0xc4`, census slot 0 (same function as Oscar, `0x004ea730`). It is **not a `Fish`**: it derives from `GameObject` directly (like `Sexy::Penta`, `board.md#class-hierarchy`) and has its own Update (slot 22 @ `0x004f8fe0`) and Draw (slot 26 @ `0x004e1bf0`).

In short: the Guppycruncher is a tank-3 bought fish that **eats small guppies** (like the Carnivore) and **drops Larvae** (Beetles, type `0x1d`, list `+0xc8`) on a timer (not per eat). It is the first half of the tank-3 economy chain: guppy → food → growth → Guppycruncher eats small guppy; Guppycruncher drops Larva on timer → Beetlemuncher eats Larva → drops coin.

**Slot numbering** is 0-based (Ghidra's `vfunctionN` = slot `N − 1`).

**Offsets** are raw `this`-relative, following the same convention as `starcatcher.md`.

RNG: `r` = app stream `MTRand::NextNoAssert` @ `0x0040aeb0`; `grand` = global `Sexy::Rand` @ `0x0040f9e0`.

`static`.

## Slot overrides

Compared with `GameObject` (vtable `0x00596844`, 81 slots):

| Slot | Grubber | Role |
|---|---|---|
| 0 | `0x004efaa0` | destructor |
| 22 | `0x004f8fe0` | **Update** |
| 26 | `0x004e1bf0` | **Draw** |
| 54 | `0x004d7dc0` (shared with Penta) | mouse-down |
| 58 | `0x004d8c10` (shared with Penta) | trivial |
| 70 | `0x004ea730` (same as Oscar) | census counter: increments `arg + 0` (slot 0) |
| 75 | `0x004f3ef0` | "eaten": **remove** with `true` |
| 76 | `0x004d8e40` (shared with Penta) | set position: widget x/y and the double x/y |
| 77 | `0x004d7ca0` | **eat** (argument = the guppy) |
| 78 | `0x004d9000` (shared with Penta) | icon animation: `+0x130 = (+0x12c / 2) % 10` |
| 79 | `0x004e1c20` | icon draw for the UI |
| 80 | `0x004d7880` | sync / serialise fields |

`static` (all slot addresses verified against the vtable in the binary).

## Fields

Based on the Starcatcher's field layout (both are direct `GameObject` subclasses) and the Grubber's own Init at `0x004d7970`. Field offsets for positions, hunger, mode, timers and widget are confirmed from the Init bytecode.

| Offset | Type | Meaning | Initial value |
|---|---|---|---|
| `+0x30`, `+0x34` | int | widget x / y | `trunc(x)`, `trunc(y)` |
| `+0x38`, `+0x3c` | int | width, height | 80, 80 |
| `+0x41` | byte | unknown (zeroed in Init) | 0 |
| `+0x55` | byte | widget mouse-visible | byte `0x005e8f14` (0) |
| `+0x88` | ptr | board pointer | set by `GameObject::GameObject` |
| `+0x8c` | int | type | **9** |
| `+0x94` | ptr | shadow | set by the shadow ctor |
| `+0x98` | ptr | attached missile | 0 |
| `+0x9c` | int | **hunger** | `r % 200 + 900` (900..1099) |
| `+0xa0` | int | colour fade (hungry look) | 0 |
| `+0xa4` | int | hunger-warning threshold | 300 |
| `+0xac` | int | Virtual Tank state (`< 0` in normal play) | −1 |
| `+0xf0` | int | prey-finding mode | 0 (`static`) |
| `+0xf8` | int | eat cooldown counter | 0; set to 100 on eat |
| `+0x104` | byte | chomp variant | `static` |
| `+0x11c` | int | hunger-decrement gate | −1 (decrement when −1) |
| `+0x158` | double | **x** | spawn x |
| `+0x160` | double | **y** | set by BuyGuppycruncher to 65.0 |
| `+0x168` | double | **vx** | 0.0 |
| `+0x170` | double | **vy** | 0.0 |
| `+0x178` | double | **speed divisor** | `[2.7, 2.5, 2.6][r % 3]` |
| `+0x180` | int | splash timer | **45** |
| `+0x188` | double | target vx | 0.0 |
| `+0x190` | int | **swim mode** 0..9 | `r % 10` |
| `+0x194` | int | chase-steer timer | 40 |
| `+0x198` | int | mode-reroll timer | 0 |
| `+0x19c`, `+0x1a0` | int | animation counter / cel | 0 |
| `+0x1a4` | int | unknown | 0 |
| `+0x1a8` | int | **Larva drop counter** | 0 |
| `+0x1ac` | int | **Larva drop interval** | `r % 200 + 300` (300..499); VT: `grand() % 200 + 4320` |

`static` (field layout and initial values confirmed from the Init bytecode at `0x004d7970`; some fields like `+0xf0`, `+0xf8`, `+0x104` are inferred from the Eat and FindPrey functions).

## Construction (`Grubber::Grubber(x)` @ `0x004ea740`)

1. `GameObject::GameObject` (`0x004e9cb0`). No RNG.
2. vtable = Grubber (`0x0059698c`).
3. **Init** `0x004d7970`, in order:
   1. `+0x41 = 0`; type = 9.
   2. Board pointer at `+0x88`, RNG at `[board+0x7b0]`.
   3. **R1:** `y = (double)(r % 5 + 360)` (360..364); widget y = `trunc(y)`. Discarded by BuyGuppycruncher.
   4. vx = vy = target vx = 0.0; size 80×80.
   5. **R2:** divisor = 2.7 if `r % 3 == 0`, 2.5 if 1, 2.6 if 2 (constants at `0x005966a8` / `0x005966a0` / `0x00596698`, same as Starcatcher).
   6. **R3:** hunger = `r % 200 + 900` (900..1099).
   7. **R4:** mode = `r % 10`. Then chase timer `+0x194 = 40`; reroll, anim, cel, `+0x1a4`, `+0x1a8` = 0; mode stored at `+0x190`.
   8. **R5:** Larva drop interval via `0x004d71d0`: in normal play, `r % 200 + 300` stored at `+0x1ac`; in VT (`board+0x87c == 5`), `grand() % 200 + 4320`.
   9. `+0x55` = byte `0x005e8f14` (0 → mouse-invisible); splash timer `+0x180 = 45`.
4. `x = (double)arg`; widget x = `trunc(x)`. The x is set **after** Init.

**5 app draws** (y, divisor, hunger, mode, interval). In VT, draw 5 uses the global stream instead.

`static` (Init bytecode at `0x004d7970` decoded directly from the PE; constants confirmed via `pe_read.py`).

## Spawn (`Board::BuyGuppycruncher` @ `0x00545430`)

Callers: shop slot 7 (`0x00549380`).

1. Allocate `0x1b0` bytes, then **RNG** `x = r % 520 + 20` (20..539).
2. `new Grubber(x)`: 5 draws (above).
3. y (`+0x160`) = **65.0** (double at `0x00596708`). Widget y (`+0x34`) = **65** (`0x41`). The ctor's y (360..364) is discarded, but its draw is still consumed.
4. `AddGameObject(obj, 1)` (`0x00542ee0`) appends it to `+0xc4`. Then the widget manager adds it (board `+0x10`, slot 3).
5. Shadow `0x00544800`: type 9 → shadow kind **1** (shared with type 8). `Shadow::Shadow` draws once (shadow y = `410 + r % 5`).
6. `RebuildZOrder`.

**7 app draws:** x, R1–R5, shadow.

**Shop purchase** (slot 7, `shop-bar.md#per-slot-behaviour`): `TrySpend(750)` → BuyGuppycruncher → `SOUND_GROW` (gap 3) → **PlaySplash** (1 draw `r % 3`). Then reveal: level 1 → reveal 11 (egg); level ≠ 1 → reveal 8 (Beetlemuncher). **A purchase costs 8 app draws on its tick** (7 from BuyGuppycruncher + 1 splash).

**Splash-in**: starts at y 65 with vy 0. Gravity pulls it down; it lands on the floor at y 359. During the first 45 ticks it emits bubbles (same pattern as Starcatcher splash).

`static` (BuyGuppycruncher bytecode at `0x00545430` decoded directly from the PE; y=65.0 constant confirmed).

## Per-tick update (slot 22 @ `0x004f8fe0`)

0. Return if the board is missing or paused.
1. PreUpdate `0x004f22c0` (generic timers).
2. **Behaviour** `0x004f8f60`: colour fade, hunger countdown (no aliens), starving check, prey chase.
   1. Colour-fade step (`0x004d6c50`, as for Starcatcher/Carnivore).
   2. Check aliens list and bilaterus list at `board+0x730`: if both empty, call hunger countdown (`0x004d6ef0`).
   3. **Starving check** (`0x004d69d0`): if true, call die (`0x004f2890` with arg 1), return.
   4. If **hunger >= 900**: return (no chasing, the Grubber is well fed).
   5. If **hunger < 900**: call the **guppy chase** (`0x004efad0`).
3. **Swim mode** (only when chase returned "not busy"): same 10-mode table as Starcatcher. Damping toward target vx.
4. **Reroll**: chase timer += 1; reroll timer += 1. If reroll timer > 20, or x ≤ 10.0, or x ≥ 540.0: reroll timer = 0, **RNG** `r % 10`; on 0, **RNG** `mode = r % 9`.
5. **Clamps**: x ≤ 550, x ≥ 10. If y > 359: y = 359, vy = 0 (the floor).
6. **Splash timer**: same pattern as Starcatcher (bubbles while timer ≥ 1; ceiling clamp when < 1).
7. **Edge steering**: vx -= 0.1 at x > 535, vx += 0.1 at x < 15 (`static`, same as Starcatcher).
8. **Gravity**: if y < 359: vy += 0.4 (`static`, same as Starcatcher).
9. **Larva drop timer** `0x004d7ae0`: `+0x1a8 += 1`; when `+0x1a8 >= +0x1ac` (interval): reset counter; in challenge mode, check hunger > 0; drop a Larva at `(widget_x + 4, widget_y − 10)` via `0x005445a0`.
10. **Animation** (`0x004d8f10`, shared with Starcatcher): cel from vx.
11. **Integrate**: x += vx / divisor, y += vy / divisor.
12. Widget position update.

`static` (Update structure confirmed from bytecode; behaviour step at `0x004f8f60` and hunger countdown at `0x004d6ef0` decoded; chase threshold 900 confirmed).

### Hunger countdown (`0x004d6ef0`)

If `+0x11c == −1`: hunger −= 1. If hunger equals `threshold + 4` (= 304 with threshold 300): start colour fade (`0x004d6920`). Clamp at −1000.

`static` (decoded from bytecode).

### Starving check (`0x004d69d0`)

- VT mode (`+0xac ≥ 0`): never starves (return false).
- Challenge mode (`board+0x880 ≠ 0`): types 7/8/9 starve at hunger ≤ −4320; other types at ≤ −2160.
- Normal Adventure: `static` — need to read the bytes at `0x004d6a20` to confirm the threshold (likely hunger ≤ 0).

`static`.

### Guppy chase (`0x004efad0`)

1. `FindPrey` (`0x004ea840`): nearest **small guppy** (via guppy list at `board+0x730→+0xa0`). The find-prey function checks `guppy+0x1a0 == 0` (filtering out spawning/hidden guppies). Centre-to-centre distance.
2. Load constant 40.0 (from `0x005965d8`).
3. If chase timer `+0x194` < 5: skip steering.
4. If no prey: skip.
5. Hungry threshold: hunger ≤ 300 (`0x12c`).
6. Steer x/y toward prey (steering constants are `static` — need decompilation of the chase body to confirm exact values).
7. **Bite**: the first small guppy in the bite box. On a bite → call Eat (slot 77).

`static` (chase function entry confirmed; threshold 300 for desperate mode confirmed; steering constants are `static`/`guess`).

### Larva drop timer (`0x004d7ae0`)

Counter `+0x1a8` increments each tick. When it reaches interval `+0x1ac`:
1. Reset counter to 0.
2. Gate (`0x004d6bd0`): in challenge mode, only drop if hunger > 0. In normal Adventure, always pass.
3. Drop a Larva (`0x005445a0`) at `(widget_x + 4, widget_y − 10)`.

The `AddBeetle` function (`0x005445a0`) allocates `0x178` bytes, calls the Beetle ctor (`0x004ead40`) with (x, y), then `AddGameObject(beetle, 0)`, widget add, `RebuildZOrder`. The Beetle ctor's RNG draws are not yet examined.

**The Larva drop is timer-based, not per-eat.** Eating only restores hunger; the Larva appears independently on the timer.

`static` (timer function at `0x004d7ae0` and gate at `0x004d6bd0` decoded from bytecode; AddBeetle call targets confirmed).

## Eat (slot 77 @ `0x004d7ca0`)

Called with the eaten guppy as argument.

1. Fade recovery (`0x004d6f30`).
2. `0x004d6a30` with arg 0 (`static` — possibly sound or state reset).
3. `+0xf8 = 100` (eat cooldown counter).
4. VT: extra call (`0x004d6f90`).
5. Push byte `+0x104` and call `0x00538560` (`static` — possibly statistics or sound).
6. **hunger += 1000**, cap at **1400**.
7. If `board+0x882 ≠ 0` (gore enabled?) and the eaten object's type ≠ `0x1c` (Beetle): **RNG** `n = r % 3 + 2` gore sprites (2–4), each with at least 1 app draw (`r % 20`, etc.).
8. If the eaten type is `0x1c` (Beetle), skip gore.

`static` (hunger increment +1000 and cap 1400 confirmed from bytecode; gore draws partially decoded).

## Death

### Starvation

The behaviour step calls `0x004f2890` with arg 1 when the starving check returns true. This is the die-with-corpse routine: `DIE` sample, `DeadFish` corpse of **kind 9**. No RNG.

`static`.

### Remove (slot 75 @ `0x004f3ef0`)

Called when eaten by an alien. Detach missile at `+0x98`, remove from WM and `board+0xc4`. If the argument is true: delete the shadow.

`static`.

### Eaten by an alien

Types 5–10 are always prey (`alien-sylvester.md#31-nearest-prey-0x004e8b60`). The alien calls slot 75 → remove(true): no corpse, shadow deleted.

`static`.

### Killed by Shrapnel

A Shrapnel bomb iterates `board+0xc4` (Grubbers) and calls `0x004f2890(1)` on each in the bomb's box (`pet-shrapnel.md#bomb-update`). This is the Grubber's die-with-corpse routine. **`DIE`** sample, then a `DeadFish` corpse.

`static`.

## Clicks

`+0x55` = byte `0x005e8f14` (0), so the widget manager doesn't deliver mouse events. Clicks pass through.

`static`.

## Drawing (slot 26 @ `0x004e1bf0`)

The image is **`IMAGE_GRUBBER`** (80×80 per cel, 10 cols × 5 rows, 800×400 px): row 0 = swim, row 1 = hungry swim, row 2 = death, rows 3–4 purpose unknown (`static` — the 10×5 grid confirmed by asset browser). Src rect: (cel × 80, row × 80, 80, 80). **Never mirrored** (`sprite-variants.md`: Grubber at `0x004e19f0` always passes mirror flag 0). Direction shows only in the cel order (the animation runs backwards when moving left, as with the Starcatcher).

The hungry fade overlay, missile overlay and hungry marker follow the same pattern as the Starcatcher.

**Shop icon**: `IMAGE_SCL_GRUBBER`, 8 cels × 1 row, animated (`shop-bar.md#button-art`, col offset (8, 1)).

**Dead fish**: `IMAGE_GRUBBER` row 2 (`DeadFish` kind 9, the same draw path as kind 8 for the Starcatcher).

`static` (sprite grid confirmed by asset browser; mirroring confirmed from `sprite-variants.md`).

## Interactions

- **Pets**: Shrapnel bombs kill Grubbers (`0x004f2890`). Angie can revive (`0x005454f0`). Gumbo's lure and Wadsworth's shelter interactions with Grubbers are unknown.
- **Food**: Grubbers never look at the food list.
- **Aliens**: always valid prey (types 5–10, `alien-sylvester.md`). Hunger frozen with aliens present.

`static`.

## RNG summary

| When | Stream | Draws |
|---|---|---|
| Purchase tick | app | x, y, divisor, hunger, mode, interval, shadow, splash (8) |
| Every 21st tick | app | `%10`; on 0, `%9` |
| Splash timer ≥ 1 | app | `% n`; on 0: `%60` x, `%60` y (+ AddBubble global) |
| Eat (gore) | app | `% 3 + 2` sprites; each ≥ 1 draw (`static`) |
| Larva drop | — | none in the drop; Beetle ctor draws TBD |
| Hunger, chase, bite, death, remove | — | none |

`static` (purchase draws confirmed from bytecode; gore and Beetle ctor draws need further analysis).

## Open questions

- The steering constants in the chase function (speeds, caps, thresholds at `0x004efad0` body).
- Bite box dimensions.
- The full gore draw count per sprite (partially decoded).
- Beetle ctor RNG draws (how many, what for).
- The exact starving threshold in normal Adventure mode (likely hunger ≤ 0, at `0x004d6a20`).
- Revival of a Grubber by Angie (`0x005454f0`): the exact draws.
- What `+0xf8 = 100` controls (possibly a cooldown that prevents immediate re-eating).
- The field `+0xf0` in FindPrey: what sets it to non-zero.
