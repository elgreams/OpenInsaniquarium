# Ultravore (`Sexy::Ultra`, type `0x06`)

Class `Sexy::Ultra`, board list `+0xd4`, object size `0x230` (same as `Fish`). An Ultra **is a `Fish`**: it uses `Fish::Update` (slot 22 @ `0x004f6970`, `fish-guppy.md#fishupdate-order-per-tick`) and `Fish::Draw` (slot 26 @ `0x004e4a90`) unchanged, and replaces a few virtual slots, the same pattern as `Sexy::Oscar` (`carnivore.md`). It eats carnivores (and possibly other medium/large fish) instead of food, and drops high-value coins.

The Ultravore is a tank-4 fish, $10 000 (slot 9, `shop-bar.md`). It is the top of the feeding chain: breeder → guppy → carnivore → ultravore → coin.

**Slot numbering** is 0-based (Ghidra's `vfunctionN` = slot `N − 1`).

**Offsets** are raw `this`-relative. Widget x/y are `+0x30`/`+0x34`.

RNG: `r` = app stream `MTRand::NextNoAssert` @ `0x0040aeb0`; `grand` = global `Sexy::Rand` @ `0x0040f9e0`.

`static` throughout; no Ultravore has been recorded or traced.

## Slot overrides

The vtable address is not yet identified. Known overrides from cross-references:

| Slot | Ultra (known) | Fish (default) | Role | Evidence |
|---|---|---|---|---|
| 0 | — | `0x004f0cb0` | destructor | unknown |
| 22 | `0x004f6970` (unchanged) | `0x004f6970` | Update | `fish-guppy.md` |
| 26 | `0x004e4a90` (unchanged) | `0x004e4a90` | Draw | `fish-guppy.md` |
| 70 | `0x004ea730` (same as Oscar) | `0x0051a440` | census counter | `carnivore.md` analogy |
| 88 | `0x004d5f90` (unchanged) | `0x004d5f90` | Die | `alien-kind5.md`: Shrapnel calls vtable `+0x160` |

Slots 77 (eat), 79 (icon draw), 81 (coin drop), 82 (hunger), 83 (draw body), 84 (prey chase), 85 (find prey), 86 (bite check), and 87 (remove) are expected to be overridden (as for Oscar), but their addresses require Ghidra access to the vtable.

`static` (Die slot from Shrapnel bomb code at `004f5050.c`; the unchanged slots from `fish-guppy.md`; the expected overrides are `guess` by analogy with Oscar).

## Fields that differ from a guppy

| Offset | Guppy | Ultravore | Notes |
|---|---|---|---|
| `+0x8c` type | 0 | **6** | `board.md` |
| `+0x38`, `+0x3c` size | 80, 80 | **160, 160** | from Shrapnel bomb box and missile hit box |
| `+0x1a0` stage | 0..4 | **6** (expected constant) | `guess`: no growth, same pattern as Oscar's stage 5 |
| `+0x9c` hunger | `r % 200 + 400` | unknown | expected higher initial range (by analogy with Oscar's `r % 200 + 600`) |
| `+0x190` floor | 370 | unknown | might be lower to accommodate the 160-px sprite |
| `+0x198` ceiling | 95 | unknown | |
| `+0x229` | 1 | **0** (expected) | `guess`: same as Oscar; disables follow-cursor and alien hide |
| `+0x55` widget mouse-visible | 1 | **byte `0x005e8f14`** (0, expected) | `guess`: same as Oscar |

Everything else comes from `Fish::Init` (`fish-guppy.md#construction`): position, velocity, divisor, swim mode, coin interval. Numeric types are as for guppies: position, velocity and divisor are `double`; the rest are `int`.

`static` (type and size from `board.md` and hitbox code); other fields are `guess` by analogy with Oscar.

## Construction (`Ultra::Ultra` via `0x00544fe0`)

The re-creation function at `0x00544fe0` (Angie revival, `pet-angie.md`) shows:

1. Allocate `0x230` bytes.
2. `Sexy::Ultra::Ultra(this, x, y, facing)` — **4 parameters** (one fewer than Breeder: no stage, since the Ultravore likely has a fixed stage).
3. `AddGameObject(obj, 1)` → appended to `+0xd4`.
4. Shadow `0x00544800`: type 6 → shadow kind **2** (`board.md#shadows`, same as aliens).
5. `RebuildZOrder`.

The constructor likely calls `Fish::Fish(x, y)` (6 app draws, `fish-guppy.md#construction`), sets the vtable, then runs `Ultra::Init` to override the type, stage, hunger, floor, ceiling and `+0x229` (same pattern as `Oscar::Oscar`, `carnivore.md#construction`). Expected **7 app draws** in the ctor (6 from Fish::Init + 1 for hunger, as Oscar), but this requires Ghidra confirmation.

`static` (re-creation function decoded from decomp `00544fe0.c`; shadow type from `board.md`; constructor internals are `guess` by analogy with Oscar).

## Spawn (`Board::BuyUltravore` @ `0x00544e60`)

Called from shop slot 9 (`shop-bar.md`). The function body is not in the decomp cache. Expected pattern (by analogy with BuyCarnivore):

1. **RNG** `x = r % 520 + 20`; **RNG** `y = r % 265 + 105`.
2. `new Ultra(x, y)`: constructor draws (expected 7, as Oscar).
3. **RNG** `vy = (double)(r % 5 + 23)` (splash-in velocity).
4. y override to splash-in start (e.g. 40.0).
5. **RNG** damp timer `r % 10 + 45`.
6. `AddGameObject(obj, 1)` → appended to `+0xd4`.
7. Shadow (`0x00544800`): 1 draw.
8. `RebuildZOrder`.

**Shop purchase** (slot 9, `shop-bar.md`): `TrySpend(10000)` → BuyUltravore → reveal 10 (weapon), 11 (egg) → `SOUND_GROW` (gap 3) → **`SOUND_SPLASHBIG`** (`0x13c`). Unlike other fish, the Ultravore plays `SOUND_SPLASHBIG` directly instead of `PlaySplash`, so **no splash RNG draw** (`shop-bar.md`).

Expected **12 app draws** on the purchase tick (x, y, ctor 7, vy, damp, shadow), same as Oscar but without the splash draw. Actual count requires Ghidra.

`static` (shop slot and price from `shop-bar.md`; SOUND_SPLASHBIG confirmed; internal BuyUltravore steps are `guess` by analogy with BuyCarnivore).

## Per-tick update (`Fish::Update` with overrides)

`fish-guppy.md#fishupdate-order-per-tick` with these expected differences (by analogy with Oscar):

- **Step 2**: `+0x229` = 0, so follow-cursor never runs. Slot 82 (Ultravore hunger) runs every tick.
- **Step 5**: slot 81 (coin drop) runs only with no aliens.
- **Step 8**: floor likely adjusted for the 160-px sprite.
- **Step 10 bubbles**: the type-6 path is a **three-bubble branch** (confirmed from `Fish::Update` decomp at `0x004f6970`, lines around `0x004f78e0`). The stage-0 path (1 bubble) is never taken because the Ultravore has a fixed non-zero stage. The branch structure for stage ≠ 0 is:
  - `type != 6` (Oscar, Gekko, guppies): **2 bubbles**, 4 position draws + 1 gate = **5 app draws**. Offsets: bubble 1 at `wx + (55 − r%60)`, `wy + (55 − r%60)`; bubble 2 at `wx + 25 + (20 − r%40)`, `wy + 25 + (15 − r%30)`.
  - `type == 6` (Ultravore): **3 bubbles**, 6 position draws + 1 gate = **7 app draws**. Offsets: bubble 1 at `wx + (125 − r%120)`, `wy + (125 − r%120)`; bubbles 2–3 at `wx + (105 − r%80)`, `wy + (105 − r%80)`. The wider offsets reflect the 160-px sprite.
  - Gate: 1-in-2 chance when damp timer 31–40, always when > 40.
- Steps 12–14 (animation, integrate, widget) unchanged.

Ultravores are in `+0xd4`, which `RebuildZOrder` places after carnivores. Each tick, every guppy and carnivore updates before any Ultravore (`sim/board.md`).

`static` (bubble branch confirmed from `Fish::Update` decomp `004f6970.c`; the type-6 check is at the `GameObject_data.offset_0x4 != 6` branch; other steps are `guess` by analogy with Oscar).

## Prey and eating

### Expected prey

The Ultravore is expected to eat **carnivores** (Oscars, `+0xa4`), by analogy with the food chain: guppy eats food, carnivore eats small guppies, ultravore eats carnivores. The find-prey and bite-check slot overrides would scan `board+0xa4` instead of `board+0xa0`.

`guess` (the food chain is implied by the game design; the actual prey list requires Ghidra).

### Eat

Expected to follow the Oscar eat pattern: hunger increase, gore burst, chomp sound. The hunger cap and increment are unknown (Oscar: +900 cap 1300; Grubber: +1000 cap 1400).

`guess`.

### Coin drop

Expected to drop a high-value coin on a timer (Oscar drops diamonds, kind 4 = $200). The Ultravore's coin kind might be **kind 7** (treasure chest, $500 from `coin.md`) or higher.

`guess`.

## Death

### Starvation (slot 88 `Fish::Die` @ `0x004d5f90`, shared)

1. **Die sound** `0x00538610(type)`: sample `0x117` `DIE`, type 6 → pitch adjust (the pitch table in `board.md` maps type 6 to pitch **2**).
2. Slot 87 (remove) with `false` → remove from `+0xd4`, widget manager, app registry. Shadow kept.
3. `Board::AddDeadFish(trunc x, trunc y, vx, vy, divisor, stage, facing, shadow)`: a `DeadFish` of **kind 6**.

`DeadFish` kind 6 has special handling (`fish-guppy.md#dead-fish`): y clamped at **310** (not 380), life counts down while y > **300** (not 370). The corpse is 160×160 and drawn from the Ultravore's sprite sheet.

**No RNG** in the die path.

`static` (Fish::Die shared code from `carnivore.md` and `fish-guppy.md`; kind-6 DeadFish behaviour from `fish-guppy.md`; Shrapnel bomb calls Die via vtable at `004f5050.c`).

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
- **Angie** (`pet-angie.md`): revives via `0x00544fe0(wx, wy, facing)`. Dead fish kind 6 → Ultravore. The corpse centre for Angie targeting uses `wx + 40` (not `wx + 80`) and the steering centre is `wx + 80` for kind-6 corpses.
- **Walter** (`pet-walter.md`): Ultravores (type `0x06`) are in the punch target list (pushed with timer 50, vy ±15/±10, coin clock reset).
- **Gumbo** (`pet-gumbo.md`): whether `board+0x174` (the lure flag) affects Ultravores is unknown (`+0x229` state unconfirmed for type 6).
- **Seymour** (`pet-seymour.md`): whether his slow-fall rate applies to Ultravore coins is `static`.
- **Wadsworth** (`pet-wadsworth.md`): shelter only protects stage 0/1 guppies, not Ultravores.
- **Food**: Ultravores never look at the food list (expected, same as Oscar).
- **Clicks**: expected mouse-invisible (`+0x55` = 0), same as Oscar.

`static`.

## Drawing

**Widget**: 160×160 (confirmed by hitbox code in `004f5050.c` and `004fc070.c`).

**Sprite sheet**: expected **`IMAGE_ULTRA`** (160×160 cels). The DrawBody slot override renders a 160-px fish with the same sheet-selection rules as Oscar (swim/turn/eat/hungry variants from `Fish::GetSwimImage`, row selected by stage). The death row of `IMAGE_ULTRA` is used by `DeadFish` kind 6.

**Shop icon**: expected `IMAGE_SCL_ULTRA`.

**Shadow**: type **2** (`board.md#shadows`), same as aliens. This is the larger, more transparent shadow variant with the +0.4 scale factor and `x += 40` offset.

`guess` (sprite sheet name and layout inferred by analogy with Oscar; no draw function examined).

## RNG summary

| When | Stream | Draws |
|---|---|---|
| Shop purchase tick | app | expected 11–12: x, y, ctor, vy, damp, shadow (no splash draw) |
| Splash-in | app | gate + 6 (**three-bubble** branch, type 6) while damp timer ≥ 31 |
| Swim modes | app/global | as guppy: mode 123 exit, 21-tick reroll |
| Eat | app | chomp, gore (expected same pattern as Oscar) |
| Coin drop | — | expected no RNG (same as Oscar's diamond) |
| Die, remove, hunger | — | none |

`static` (no-splash-draw from `shop-bar.md`; splash-in bubble branch confirmed from `004f6970.c`; the rest are `guess` by analogy with Oscar).

## Open questions

- **Vtable and slot overrides**: the full slot table (especially eat, find-prey, bite-check, hunger, coin-drop, draw-body). Requires Ghidra.
- **Prey**: confirm that the Ultravore eats carnivores (and whether it also eats Gekkos or other medium fish).
- **Coin kind and interval**: what coin the Ultravore drops and how often.
- **Constructor**: the Ultra::Init fields (hunger range, floor, ceiling, stage value, `+0x229`).
- **BuyUltravore** (`0x00544e60`): full draw count, splash-in start y.
- **Drawing**: sprite sheet grid, mirroring, the 160-px DrawBody function.
- **Growth**: whether the Ultravore has a fixed stage (expected yes, like Oscar's stage 5).
