# Ultravore (`Sexy::Ultra`, type `0x06`)

Class `Sexy::Ultra`, vtable `0x0059872c`, board list `+0xd4`, object size `0x230` (same as `Fish`). An Ultra **is a `Fish`**: it uses `Fish::Update` (slot 22 @ `0x004f6970`, `fish-guppy.md#fishupdate-order-per-tick`) and `Fish::Draw` (slot 26 @ `0x004e4a90`) unchanged, and replaces a dozen virtual slots, the same pattern as `Sexy::Oscar` (`carnivore.md`). It eats carnivores (Oscars from `board+0xa4`) and drops kind-7 coins ($2000).

The Ultravore is a tank-4 fish, $10 000 (slot 9, `shop-bar.md`). It is the top of the feeding chain: breeder → guppy → carnivore → ultravore → coin.

**Slot numbering** is 0-based (Ghidra's `vfunctionN` = slot `N − 1`).

**Offsets** are raw `this`-relative. Widget x/y are `+0x30`/`+0x34`.

RNG: `r` = app stream `MTRand::NextNoAssert` @ `0x0040aeb0`; `grand` = global `Sexy::Rand` @ `0x0040f9e0`.

`static` throughout; no Ultravore has been recorded or traced.

## Slot overrides

Vtable at `0x0059872c`, read from the binary (the 2-param ctor at `0x004f02a0` and 3-param ctor at `0x004f0310` both set it).

| Slot | Ultra | Fish (default) | Role |
|---|---|---|---|
| 0 | `0x004f0d70` | `0x004f0cb0` | destructor |
| 22 | `0x004f6970` (unchanged) | `0x004f6970` | Update |
| 26 | `0x004e4a90` (unchanged) | `0x004e4a90` | Draw |
| 55 | `0x0046e400` (unchanged) | `0x0046e400` | mouse-down (never reached) |
| 70 | `0x004f0290` | `0x0051a440` (no-op) | census counter |
| 75 | `0x004f01d0` (unchanged) | `0x004f01d0` | "eaten" (calls slot 87) |
| 77 | **`0x004e3c60`** | `0x004f1ba0` | **eat** (victim = an Oscar) |
| 79 | `0x004da180` | `0x004dee90` | icon draw for UI |
| 81 | **`0x004d9f90`** | `0x004f1400` | **coin drop** (kind 7) |
| 82 | **`0x004e38f0`** | `0x004f15c0` | **hunger** |
| 83 | `0x004e3a10` | `0x004dea00` | draw body |
| 84 | `0x004da260` | `0x004d5b20` | **prey chase** |
| 85 | **`0x004eca70`** | `0x004ef050` | **find prey** |
| 86 | **`0x004ecbf0`** | `0x004ef1b0` | **bite check** |
| 87 | `0x004da5e0` | `0x004df0b0` | **remove** |
| 88 | `0x004d5f90` (unchanged) | `0x004d5f90` | Die |
| 89 | `0x004da000` | `0x004de840` | animation |
| 90 | `0x004d5650` (unchanged) | `0x004d5650` | colour setup |

Shared and unchanged: 22 Update, 26 Draw, 55 mouse-down, 75 "eaten" (`0x004f01d0`), 88 Die `0x004d5f90`, 90 colour setup `0x004d5650`.

`static` (vtable pointer from both constructors' `MOV [esi], 0x0059872c` instructions; slot addresses read directly from the vtable in the binary at file offset corresponding to VA `0x0059872c`).

## Fields that differ from a guppy

| Offset | Guppy | Ultravore | Notes |
|---|---|---|---|
| `+0x8c` type | 0 | **6** | `Ultra::Init` @ `0x004d9ee0` |
| `+0x38`, `+0x3c` size | 80, 80 | **160, 160** | `Ultra::Init`: `MOV [esi+0x38], 0xa0` |
| `+0x1a0` stage | 0..4 | **6** (constant) | `Ultra::Init`: same register as type, stored at both offsets |
| `+0x9c` hunger | `r % 200 + 400` | **`r % 200 + 600`** | `Ultra::Init` R1; same range as Oscar |
| `+0x224` coin interval | `r % 200 + 150` | **`r % 250 + 200`** | `Ultra::Init` R2; overrides `Fish::Init`'s value |
| `+0x190` floor | 370 | **310** (0x136) | `Ultra::Init` |
| `+0x198` ceiling | 95 | **75** (0x4b) | `Ultra::Init` |
| `+0x194` | ? | **0** | `Ultra::Init`: zeroed; unknown field |
| `+0x19c` | ? | **480** (0x1e0) | `Ultra::Init`: unknown field |
| `+0x229` | 1 | **0** | `Ultra::Init`: disables follow-cursor and alien hide |
| `+0x55` widget mouse-visible | 1 | **byte `0x005e8f14`** (0) | `Ultra::Init`: carnivores never take clicks |

Everything else comes from `Fish::Init` (`fish-guppy.md#construction`): position, velocity, **divisor** ∈ {2.0, 1.8, 1.6}, `+0x1a8`, initial swim mode, hunger-warning threshold `+0xa4` = 300. Numeric types are as for guppies: position, velocity and divisor are `double`; the rest are `int`.

`static` (all fields read from `Ultra::Init` at `0x004d9ee0` via binary analysis; type and size confirmed from earlier hitbox code).

## Divisor

The Ultravore's divisor is **inherited from `Fish::Init`**, the same as Oscar. `Fish::Init` R3 picks from {2.0, 1.8, 1.6} via `r % 3` (`fish-guppy.md#construction`). `Ultra::Init` does **not** write to the divisor offset — the value from `Fish::Init` stands.

Divisor values: 2.0 (33%), 1.8 (33%), 1.6 (33%). Higher divisor → slower swimming.

`static` (confirmed by reading `Ultra::Init` at `0x004d9ee0` end-to-end: no store to the divisor field, which is at the same offset as the guppy divisor).

## Construction (`Ultra::Ultra` @ `0x004f02a0` / `0x004f0310`)

Two constructors exist:

**2-param** (`0x004f02a0`, used by `BuyUltravore`): `Ultra::Ultra(this, x, y)`.
1. `Fish::Fish(x, y)` (`0x004eef00`): `GameObject` ctor, then `Fish::Init(x, y)` (**6 app draws** R1–R6, `fish-guppy.md#construction`).
2. vtable = `0x0059872c` (Ultra).
3. `Ultra::Init` (`0x004d9ee0`): sets type, stage, size, floor, ceiling, `+0x229`, mouse-visible, then:
   - **R7:** hunger `+0x9c = r % 200 + 600` (600..799).
   - **R8:** coin interval `+0x224 = r % 250 + 200` (200..449).
4. `RET 8`.

**3-param** (`0x004f0310`, used by Angie revival): `Ultra::Ultra(this, x, y, facing)`.
1. Same `Fish::Fish(x, y)` + vtable + `Ultra::Init` sequence.
2. `RET 12`.

**8 app draws** in either constructor (6 from `Fish::Init` + 2 from `Ultra::Init`).

`static` (2-param ctor at `0x004f02a0`: CALL `0x004eef00` (Fish::Fish), MOV `[esi], 0x0059872c`, CALL `0x004d9ee0` (Ultra::Init), RET 8; 3-param at `0x004f0310`: same calls, RET 12; `Ultra::Init` traced through its full hex: type/stage write, size, floor/ceiling, hunger RNG, coin-interval RNG, RET).

## Spawn (`Board::BuyUltravore` @ `0x00544e60`)

Called from shop slot 9 (`shop-bar.md`). Decoded from the binary:

1. **R1:** `x = r % 520 + 20`.
2. **R2:** `y = r % 265 + 105`.
3. `new(0x230) Ultra::Ultra(x, y)` → 2-param ctor (**8 draws**: R3–R8 above).
4. **R9:** `vy = (double)(r % 5 + 25)` → stored at `+0x170` (25..29, splash-in velocity).
5. `+0x160` (vx) = 40.0 (constant, no RNG).
6. `+0x34` (widget y) = **40** (splash-in start y).
7. **R10:** damp timer `+0x208 = r % 10 + 45` (45..54).
8. `Board::AddGameObject(obj, 1)` → appended to `+0xd4`.
9. vtable dispatch (board widget list insert).
10. `FUN_00544800` → shadow kind **2** (from the type-6 switch case).
11. `Board::RebuildZOrder`.

**Shop purchase** (slot 9, `shop-bar.md`): `TrySpend(10000)` → BuyUltravore → reveal 10 (weapon), 11 (egg) → `SOUND_GROW` (gap 3) → **`SOUND_SPLASHBIG`** (`0x13c`). Unlike other fish, the Ultravore plays `SOUND_SPLASHBIG` directly instead of `PlaySplash`, so **no splash RNG draw** (`shop-bar.md`).

**13 app draws** on the purchase tick: R1 (x), R2 (y), R3–R8 (ctor 6 Fish::Init + 2 Ultra::Init), R9 (vy), R10 (damp), shadow (1). No splash draw.

`static` (hex decoded from the binary at `0x00544e60`: the alloc size 0x230, ctor call to `0x004f02a0`, the four post-ctor RNG draws with their moduli and addends, the constant stores at `+0x34` and `+0x160`, the calls to `0x00542ee0` (AddGameObject), `0x00544800` (shadow), and `0x0053aa40` (RebuildZOrder); SOUND_SPLASHBIG confirmed from `shop-bar.md`).

## Per-tick update (`Fish::Update` with overrides)

`fish-guppy.md#fishupdate-order-per-tick` with these differences (from `Ultra::Init` field values and the vtable):

- **Step 2**: `+0x229` = 0, so follow-cursor never runs. Slot 82 (`0x004e38f0`, Ultravore hunger) runs every tick.
- **Step 5**: slot 81 (`0x004d9f90`, coin drop) runs only with no aliens.
- **Step 8**: floor = 310 (not 370), adjusted for the 160-px sprite.
- **Step 10 bubbles**: the type-6 path is a **three-bubble branch** (confirmed from `Fish::Update` decomp at `0x004f6970`, lines around `0x004f78e0`). The stage-0 path (1 bubble) is never taken because the Ultravore has stage 6. The branch structure for stage ≠ 0 is:
  - `type != 6` (Oscar, Gekko, guppies): **2 bubbles**, 4 position draws + 1 gate = **5 app draws**. Offsets: bubble 1 at `wx + (55 − r%60)`, `wy + (55 − r%60)`; bubble 2 at `wx + 25 + (20 − r%40)`, `wy + 25 + (15 − r%30)`.
  - `type == 6` (Ultravore): **3 bubbles**, 6 position draws + 1 gate = **7 app draws**. Offsets: bubble 1 at `wx + (125 − r%120)`, `wy + (125 − r%120)`; bubbles 2–3 at `wx + (105 − r%80)`, `wy + (105 − r%80)`. The wider offsets reflect the 160-px sprite.
  - Gate: 1-in-2 chance when damp timer 31–40, always when > 40.
- Steps 12–14 (animation, integrate, widget) unchanged. Slot 89 (animation) is Ultra-specific at `0x004da000`.

Ultravores are in `+0xd4`, which `RebuildZOrder` places after carnivores. Each tick, every guppy and carnivore updates before any Ultravore (`sim/board.md`).

`static` (bubble branch confirmed from `Fish::Update` decomp `004f6970.c`; the type-6 check is at the `GameObject_data.offset_0x4 != 6` branch; `+0x229 = 0` and floor 310 from `Ultra::Init`; slot addresses from the vtable).

## Prey and eating

### Prey list

Ultra's **find-prey** (slot 85, `0x004eca70`) scans **`board+0xa4`** — the **carnivore (Oscar) list**. This is confirmed by the instruction `MOV edi, [ecx+0xa4]` at `0x004ecab9` inside the function (compare Oscar's find-prey at `0x004eb080` which reads `[ecx+0xa0]`, the guppy list).

The function also filters prey with two checks:
- `prey+0xac < 0` required (meaning not already claimed/being eaten).
- `prey+0x120 <= 0` required (meaning not in a special state).

The prey-chase slot (84, `0x004da260`) steers toward the nearest eligible Oscar. The bite-check slot (86, `0x004ecbf0`) tests whether the Ultra overlaps its prey.

`static` (from binary analysis of `0x004eca70`: the board list offset `0xa4` confirmed by the MOV instruction at `0x004ecab9`; the two filter comparisons at `0x004ecb33` and `0x004ecb3d`).

### Bite check (slot 86, `0x004ecbf0`)

Tests whether the Ultra overlaps its target. Two paths depending on whether prey is currently assigned (`+0x11c`).

**No-prey path** — scans `board+0xa4` (Oscar list) using double-precision positions (`+0x158` x, `+0x160` y):

| Box | x range | y range | Size | On match |
|---|---|---|---|---|
| Inner (eat) | `pos_x − 40` to `pos_x + 120` | `pos_y + 10` to `pos_y + 90` | 160 × 80 | slot 77 (Eat) + slot 87 (remove); approach timer `+0x1c8 = 8` |
| Outer (approach) | `pos_x − 70` to `pos_x + 150` | `pos_y − 10` to `pos_y + 110` | 220 × 120 | approach timer `+0x1c8 = 20` (only when `+0x1c8 == 0`) |

The inner box is checked first; the outer box is only tested when the approach timer `+0x1c8` is already 0.

**Prey-exists path** — computes the Ultra's centre as `(wx + 80, wy + 100)` (top-left + half-width, top-left + half-height + 20), then calls the shared collision function at `0x004ea2a0` (same function Oscar uses, `carnivore.md`). The centre formula is generic: `(wx + width/2, wy + height/2 + 20)` — Oscar (80×80) gets `(wx+40, wy+60)`, Ultra (160×160) gets `(wx+80, wy+100)`. Return value 1 (tight hit) sets `+0x1c8 = 8`; return value 2 (loose hit) sets `+0x1c8 = 20`.

Double constants used: `[0x596740]` = 80.0 (half Ultra width), `[0x5972b8]` = 90.0 (half-height + 10, y-centre offset for bite area).

`static` (from binary analysis of `0x004ecbf0`: the two box computations using double-precision arithmetic and the constants at `0x596740` and `0x5972b8`; the shared collision call to `0x004ea2a0`; approach timer field `+0x1c8` and its values 8/20 from the MOV instructions).

### Eat

Slot 77 (`0x004e3c60`). When the bite-check succeeds:

1. The prey Oscar is consumed (removed from the board).
2. **Hunger** `+0x9c += 900` (`0x384`), **capped at 1300** (`0x514`). Same as Oscar eating a guppy (`carnivore.md`).
3. **Gore burst** (gated by `board+0x882 != 0` and prey type ≠ `0x1c`): **`r % 3 + 2`** pieces (2–4). Each piece takes 2 app draws for position offsets (`r % 20`).
4. Chomp sound.

`static` (from binary analysis of `0x004e3c60`: the `ADD [esi+0x9c], 0x384` at `0x004e3cc5`, the `CMP eax, 0x514` / `MOV [esi+0x9c], 0x514` cap at `0x004e3cd0`, the gore RNG `DIV ECX` with ECX=3 and `ADD EDX, 2` at `0x004e3d10`).

### Coin drop

Slot 81 (`0x004d9f90`). Each tick (gated by no-aliens check):

1. Increment coin timer `+0x220` by 1.
2. When `+0x220 >= +0x224` (the interval): reset timer, drop a coin.
3. Coin: **kind 7** ($2000, `coin.md`), at `(wx + 40, wy + 90)`, initial vy = **−1.0** (rising). No RNG in the drop itself.

The interval `+0x224 = r % 250 + 200` (200..449 ticks) is set once in `Ultra::Init`.

`static` (from binary analysis of `0x004d9f90`: the `PUSH 7` at `0x004d9fdf` selects coin kind 7; `ADD ECX, 0x5a` and `ADD EDX, 0x28` give the position offsets 90 and 40; `FLD qword [0x005c3d80]` loads −1.0 as vy; the timer increment `ADD [esi+0x220], 1` at `0x004d9f96`).

### Hunger (slot 82, `0x004e38f0`)

Both Ultra's slot 82 (`0x004e38f0`) and the base Fish slot 82 (`0x004f15c0`) call the **same shared subroutine** at `0x004d6ef0`. Ultra does **not** override the decrement amount — the per-tick hunger logic is identical to all other fish:

1. If prey is assigned (`+0x11c != -1`): **skip** — hunger does not decrement while chasing/eating.
2. Otherwise: `+0x9c -= 1` (hunger decrements by exactly 1 per tick).
3. Notification: when hunger equals `threshold + 4` (i.e. **304** for the standard warning threshold of 300 at `+0xa4`), a hunger warning is triggered via `0x004d6920`.
4. Clamp: hunger is clamped to a minimum of **−1000** (`0xFFFFFC18` as signed int).

The assembly at `0x004d6ef0` is: `OR EAX, -1; CMP [ECX+0x11c], EAX; JNZ skip; ADD [ECX+0x9c], EAX` — the `OR EAX, -1` sets EAX to `0xFFFFFFFF` (= −1 signed), which serves double duty as both the "no prey" sentinel comparison value and the decrement amount.

`static` (from binary analysis of `0x004d6ef0`: the `OR EAX, -1` / `CMP` / `ADD` sequence; the threshold+4 notification check; the −1000 clamp; confirmed that both Ultra and Fish vtable entries for slot 82 call through to this same subroutine).

## Death

### Starvation (slot 88 `Fish::Die` @ `0x004d5f90`, shared)

1. **Die sound** `0x00538610(type)`: sample `0x117` `DIE`, type 6 → pitch adjust (the pitch table in `board.md` maps type 6 to pitch **2**).
2. Slot 87 (`0x004da5e0`, Ultra-specific remove) with `false` → remove from `+0xd4`, widget manager, app registry. Shadow kept.
3. `Board::AddDeadFish(trunc x, trunc y, vx, vy, divisor, stage, facing, shadow)`: a `DeadFish` of **kind 6**.

`DeadFish` kind 6 has special handling (`fish-guppy.md#dead-fish`): y clamped at **310** (not 380), life counts down while y > **300** (not 370). The corpse is 160×160 and drawn from the Ultravore's sprite sheet.

**No RNG** in the die path.

`static` (Fish::Die shared code from `carnivore.md` and `fish-guppy.md`; kind-6 DeadFish behaviour from `fish-guppy.md`; Shrapnel bomb calls Die via vtable at `004f5050.c`; slot 87 address `0x004da5e0` from the vtable).

### Eaten by an alien

Types 5–10 are always prey (`alien-sylvester.md#31-nearest-prey-0x004e8b60`): the Ultravore (type 6) is included. The alien's eat box uses half-extents **70** (x) and **85** (y) for type 6, wider than the standard 45/65 for other fish (`alien-sylvester.md`). The gore burst offset is **+55** instead of +15 (`alien-sylvester.md`).

On a hit the alien calls the victim's slot 75 → slot 87 (remove with `true`): the Ultravore is removed with no `DeadFish`, shadow deleted.

`static` (prey filter and eat box from `alien-sylvester.md`; gore offset from the same spec).

### Killed by Shrapnel

A Shrapnel bomb iterates `board+0xd4` (Ultravores) and calls slot 88 (Die) on each in the bomb's box (`pet-shrapnel.md`, `004f5050.c`). The hitbox uses `wx + 10` to `wx + 0x96` (150) on both axes → **160×160** (from `004f5050.c`).

`static`.

### Killed by a missile

Missiles (`004fc070.c`) walk `board+0xd4` (Ultravores) with the 160×160 hitbox (`wx + 10` to `wx + 0x96`, `wy + 10` to `wy + 0x96`). The missile type-6 hit box for aliens uses `wx + 0x8c` (140) / `wx + 0x1e` (30) (`alien-kind5.md`).

`static`.

## Interactions

- **Aliens**: always prey (types 5–10). Eat box 70/85 half-extents (wider than standard). Hunger frozen with aliens present.
- **Destructor missiles** (`alien-kind5.md`): target the farthest fish including Ultravores. The targeting uses wide bands: `tx + 200`, `tx − 40`, centre `tx + 120` (from the 160-px widget). The hit box is `tx + 8 < p < tx + 140`, `ty + 4 < q < ty + 140`.
- **Rhubarb** (`pet-rhubarb.md`): walks `board+0xd4`, centre `wx + 80` / `wy + 80`. Fling sets vy to −20 or −30 (1 app draw) and damp timer `+0x208` to 50.
- **Shrapnel** (`pet-shrapnel.md`): bomb kill list includes `+0xd4` Ultravores.
- **Angie** (`pet-angie.md`): revives via `0x00544fe0(wx, wy, facing)` → 3-param ctor at `0x004f0310`. Dead fish kind 6 → Ultravore. The corpse centre for Angie targeting uses `wx + 40` (not `wx + 80`) and the steering centre is `wx + 80` for kind-6 corpses.
- **Walter** (`pet-walter.md`): Ultravores (type `0x06`) are in the punch target list (pushed with timer 50, vy ±15/±10, coin clock reset).
- **Gumbo** (`pet-gumbo.md`): whether `board+0x174` (the lure flag) affects Ultravores is unknown (`+0x229` = 0 means they check the flag in `Fish::Update` — same as Oscar, so Gumbo's lure **does** steer them).
- **Seymour** (`pet-seymour.md`): whether his slow-fall rate applies to Ultravore coins is `static`.
- **Wadsworth** (`pet-wadsworth.md`): shelter only protects stage 0/1 guppies, not Ultravores.
- **Food**: Ultravores never look at the food list (find-prey scans `+0xa4` only).
- **Clicks**: mouse-invisible (`+0x55` = 0 from `Ultra::Init`), same as Oscar.

`static`.

## Drawing

**Widget**: 160×160 (confirmed by hitbox code in `004f5050.c` and `004fc070.c`, and by `Ultra::Init` setting `+0x38`/`+0x3c` to 0xa0).

**DrawBody** (slot 83, `0x004e3a10`): references image global at `0x005e8cf4` as the main sprite sheet (distinct from Oscar's `0x005e8aa8`), and `0x005e8bc4` as a secondary image (shared with Oscar — likely a glow/overlay). The function uses offsets 40 (`0x28`) for centering and 500 for an animation constant. The expected image name is `IMAGE_ULTRA` (160×160 cels). The death row of this sheet is used by `DeadFish` kind 6.

**Shop icon**: expected `IMAGE_SCL_ULTRA`.

**Shadow**: type **2** (`board.md#shadows`, from the type-6 case in `FUN_00544800`), same as aliens. This is the larger, more transparent shadow variant with the +0.4 scale factor and `x += 40` offset.

`static` (DrawBody address `0x004e3a10` from the vtable; image globals `0x005e8cf4` and `0x005e8bc4` from MOV instructions in the function; shadow type 2 from `FUN_00544800` switch case at `0x0054483c`; widget size from `Ultra::Init`; image name `IMAGE_ULTRA` and `IMAGE_SCL_ULTRA` are conventional names, not yet confirmed from resources.xml).

## RNG summary

| When | Stream | Draws |
|---|---|---|
| Shop purchase tick | app | **13**: R1 x, R2 y, R3–R8 ctor (Fish::Init 6 + Init hunger + Init coin-interval), R9 vy, R10 damp, shadow 1 |
| Splash-in | app | gate + 7 (**three-bubble** branch, type 6) while damp timer ≥ 31 |
| Swim modes | app/global | as guppy: mode 123 exit, 21-tick reroll |
| Eat | app | 1 (remove target) + gore: `r%3+2` pieces × 2 position draws each = 5–9 app draws |
| Coin drop | — | **none** (no RNG; just timer tick and constant coin creation) |
| Die, remove, hunger | — | none |

`static` (purchase draw count from `BuyUltravore` hex at `0x00544e60`; splash-in bubble branch confirmed from `004f6970.c`; eat gore from `0x004e3c60`; coin drop from `0x004d9f90`).

## Open questions

- **Animation** (slot 89, `0x004da000`): cel selection and frame rate for the 160-px sheet.
- **Census counter** (slot 70, `0x004f0290`): what it counts and where.
- **Growth**: the Ultravore has stage 6 (constant). Whether the stage ever changes (expected no) requires checking `Fish::UpdateHunger` for the stage override.
- **Drawing details**: sprite sheet grid, mirroring, the 160-px DrawBody function's row selection.
- **Unknown fields**: `+0x194` = 0, `+0x19c` = 480 (set by `Ultra::Init`); and the prey filters `+0xac` and `+0x120`.
