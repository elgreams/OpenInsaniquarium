# Breeder (`Sexy::Breeder`, type `0x0a`)

Class `Sexy::Breeder`, board list `+0xc0`, object size `0x1e0`. It is **not a `Fish`**: it derives from `GameObject` directly (like `Sexy::Penta` and `Sexy::Grubber`, `board.md#class-hierarchy`) and has its own Update and Draw.

In short: the Breeder is a tank-4 bought fish ($200, slot 1) that **eats food** (like a guppy) and **produces baby guppies** periodically (like Prego). It replaces the guppy buy for the tank 4 economy: buy food → feed breeder → breeder spawns guppies → guppies grow → carnivore eats small guppy → ultravore eats carnivore.

**Slot numbering** is 0-based (Ghidra's `vfunctionN` = slot `N − 1`).

**Offsets** are raw `this`-relative, following the same convention as `guppycruncher.md`.

RNG: `r` = app stream `MTRand::NextNoAssert` @ `0x0040aeb0`; `grand` = global `Sexy::Rand` @ `0x0040f9e0`.

`static` throughout. Recorded in `tank4-1_full_01` (2026-11-17): breeder spawned 4 guppies, eaten by Balrog; replay test pending.

## Slot overrides

The vtable address is not yet identified. Known function addresses from cross-references:

| Function | Address | Source |
|---|---|---|
| Die (with corpse) | `0x004dd5d0` | decomp `004dd5d0.c` |
| Remove | `0x004dd530` | called from Die |
| Re-creation (Angie) | `0x00547270` | `pet-angie.md` |
| BuyBreeder | `0x005470a0` | `shop-bar.md` |
| Level-start spawn | `0x005471e0` | `level-start.md` |
| Hunger update | `0x004f1340` | `alien-kind4.md` |

The full slot table (destructor, Update, Draw, mouse-down, census, eat, icon, sync) requires Ghidra access to the vtable.

`static`.

## Fields

Inferred from the Grubber's field layout (both are direct `GameObject` subclasses with similar structure) and the Die function at `0x004dd5d0`. The Breeder is `0x30` bytes larger than the Grubber (`0x1e0` vs `0x1b0`), with extra fields in the `+0x1b0`..`+0x1dc` range.

| Offset | Type | Meaning | Evidence |
|---|---|---|---|
| `+0x30`, `+0x34` | int | widget x / y | shared with all GameObjects |
| `+0x38`, `+0x3c` | int | width, height | 80, 80 (from Shrapnel bomb box and missile hit box) |
| `+0x88` | ptr | board pointer | `GameObject` field |
| `+0x8c` | int | type | **`0x0a`** (10) (`board.md`) |
| `+0x94` | ptr | shadow | read by Die (`004dd5d0.c`) |
| `+0x9c` | int | **hunger** | Zorf checks `+0x9c < 300` on Breeders (`pet-zorf.md`) |
| `+0xa0` | int | colour fade | by analogy with Grubber |
| `+0xa4` | int | hunger-warning threshold | by analogy with Grubber (300) |
| `+0xac` | int | Virtual Tank state | by analogy with Grubber |
| `+0x168` | double | **vx** | read by Die for facing |
| `+0x170` | double | **vy** | read by Die; set by Rhubarb fling (`pet-rhubarb.md`) |
| `+0x180` | double | divisor or vy (used by AddDeadFish) | read by Die |
| `+0x1a0` | int | **stage / sub-kind** | dead fish kind = `+0x1a0 + 10`; Angie revival passes `kind − 10` as stage |
| `+0x1a4` | int | **growth counter** | set to 2 at level start (`level-start.md`) |
| `+0x1d8` | int | **vy-damp timer** | Rhubarb sets this to 50 (`pet-rhubarb.md`) |

Fields from `+0x1b0` to `+0x1dc` are unidentified; they likely include a guppy-spawn timer and interval (by analogy with the Grubber's Larva drop fields at `+0x1a8`/`+0x1ac`).

`static` (field offsets verified from cross-references in decomp and existing specs; initial values and unidentified fields are `guess`).

## Construction (`Sexy::Breeder::Breeder` via `0x00547270`)

The re-creation function at `0x00547270` (Angie revival, `pet-angie.md`) shows:

1. Allocate `0x1e0` bytes.
2. `Sexy::Breeder::Breeder(this, x, y, stage, facing)` — 5 parameters.
3. `AddGameObject(obj, 1)` → appended to `+0xc0`.
4. Shadow `0x00544800`: type `0x0a` → shadow kind **0** (`board.md#shadows`). `Shadow::Shadow` draws **once** (`shadow y = 410 + r % 5`).
5. `RebuildZOrder`.

The constructor's internal RNG draws are unknown (Grubber has 5; the Breeder likely has a similar count). The 5th parameter (facing) distinguishes the Breeder from the 4-param Ultra and Oscar ctors and the 1-param Grubber ctor.

`static` (re-creation function decoded from decomp `00547270.c`; shadow type from `board.md`; constructor internals are `guess`).

## Spawn

### Shop purchase (`Board::BuyBreeder` @ `0x005470a0`)

Called from shop slot 1 (`shop-bar.md#per-slot-behaviour`). The function body is not in the decomp cache. Expected pattern (by analogy with BuyGuppycruncher):

1. **RNG** `x = r % 520 + 20` (position draw).
2. `new Breeder(x, ...)`: constructor draws (unknown count).
3. y override (probably to 65.0, same as Grubber and Starcatcher for floor fish).
4. `AddGameObject(obj, 1)` → appended to `+0xc0`.
5. Shadow (`0x00544800`): 1 draw.
6. `RebuildZOrder`.

**Shop purchase** (slot 1, `shop-bar.md`): `TrySpend(200)` → BuyBreeder → `SOUND_GROW` (gap 3) → **PlaySplash** (1 draw `r % 3`). Then reveal: buy carnivore → reveal chain.

`static` (shop slot and price from `shop-bar.md`; the internal BuyBreeder steps are `guess` by analogy).

### Level start (`0x005471e0`)

Tank 4 starts with one breeder via `0x005471e0` at a random position (**2 draws**, `level-start.md`), then sets `+0x1a4 = 2` (growth counter preset: 2 steps toward the next stage, like starting guppies).

`static` (from `level-start.md`; the 2 draws are for position, same pattern as AddPet).

## Per-tick update

The Update function address is not identified, but cross-references reveal the structure:

1. **Hunger update** (`0x004f1340`, called from `Breeder::Update`, `alien-kind4.md`): when the first alien is kind 4 (Gus), Breeders stop chasing food (at `0x004f13e5`), same as guppies. This confirms the Breeder has food-chasing behaviour.
2. **Colour fade**: by analogy with Grubber/Starcatcher.
3. **Hunger countdown**: with no aliens present, hunger decrements (same `0x004d6ef0` shared routine).
4. **Starving check** (`0x004d69d0`): if hunger ≤ 0, die.
5. **Food chase**: when hungry (hunger below threshold), chase the nearest food pellet.
6. **Swim modes**: expected 10-mode table (same as Grubber/Starcatcher).
7. **Guppy spawning**: on a timer (see below).
8. **Animation, integrate, widget update**.

The Breeder eats food pellets, not fish. Zorf checks the breeder list for hungry breeders (`pet-zorf.md`: `board+0xc0`, hunger `+0x9c < 300`), confirming hunger mechanics. Nostradamus's spec notes "Breeders eat food too" (`pet-nostradamus.md`).

`static` (hunger update address from `alien-kind4.md`; food chasing confirmed by multiple cross-references; update structure is `guess` by analogy with Grubber).

## Guppy spawning

The Breeder produces baby guppies periodically, replacing the guppy buy in tank 4. The mechanism is expected to be similar to Prego's birth (`pet-prego.md`: interval 930 ticks, counter freezes during waves, 7 draws per birth) or the Grubber's Larva drop (`guppycruncher.md`: timer at `+0x1a8`/`+0x1ac`, interval 300..499 ticks).

Specifics (interval, timer fields, spawn position, whether the counter freezes during waves, RNG draws per spawn) require Ghidra access to the Breeder's Update function.

`guess` (expected by analogy with Prego and Grubber; no code examined for the spawn mechanism).

## Death

### Die (`0x004dd5d0`)

Decoded from `004dd5d0.c`:

1. If arg ≠ 0: **die sound** `0x00538610(type)` — sample `0x117` `DIE` with a type-dependent pitch adjust (type `0x0a`).
2. **Remove** `0x004dd530(0)`: removes the Breeder from its board list and widget manager. The shadow is **not** deleted (arg 0 = keep shadow, passed to `AddDeadFish`).
3. **Facing**: `vx (+0x168) > 0.0` → facing right.
4. **Dead fish kind** = `*(int*)(this + 0x1a0) + 10`: stage 0 → kind 10, stage 1 → kind 11, stage 2 → kind 12.
5. `Board::AddDeadFish(trunc x, trunc y, vx, vy, +0x180, kind, facing, shadow)` at `0x005441f0`: allocates `0x1a8` bytes, creates a `DeadFish` corpse, `AddGameObject(0)`, attaches shadow.

**No RNG** in the die path.

`static` (decoded from decomp `004dd5d0.c`; AddDeadFish at `005441f0.c`).

### Starvation

The starving check at `0x004d69d0` (shared with Grubber, `guppycruncher.md#starving-check`): in normal Adventure, starves at hunger ≤ 0. Calls Die with arg 1 (plays sound).

`static`.

### Eaten by an alien

Types 5–10 are always prey (`alien-sylvester.md#31-nearest-prey-0x004e8b60`): the Breeder (type `0x0a` = 10) is in this range. The alien eats the Breeder (removes it, no corpse, no sound).

`static`.

### Killed by Shrapnel

A Shrapnel bomb iterates `board+0xc0` (Breeders) and calls `0x004dd5d0(1)` on each in the bomb's box (`pet-shrapnel.md`, `alien-kind5.md`). This is the die-with-corpse routine above. The bomb box uses `wx + 10` to `wx + 70` and `wy + 10` to `wy + 70` (80×80 widget, `004f5050.c`).

`static`.

### Killed by a missile

Missiles (`004fc070.c`) walk `board+0xc0` (Breeders) with the 80×80 hitbox (`wx + 10` to `wx + 70`, `wy + 10` to `wy + 70`). On a hit the Breeder is destroyed (same die path as Shrapnel).

`static`.

## Interactions

- **Aliens**: always prey (types 5–10). Alien eat box uses half-extents 45/65 (standard, `alien-sylvester.md`). Hunger frozen with aliens present (from the shared hunger countdown gate).
- **Rhubarb** (`pet-rhubarb.md`): walks `board+0xc0`, centre `wx + 40` / `wy + 40`. Fling sets vy to −20 or −30 (1 app draw) and damp timer `+0x1d8` to 50.
- **Zorf** (`pet-zorf.md`): checks the breeder list for hunger `+0x9c < 300`, same as guppies.
- **Shrapnel** (`pet-shrapnel.md`): bomb kill list includes `+0xc0` Breeders.
- **Angie** (`pet-angie.md`): revives via `0x00547270(wx, wy, kind − 10, facing)`. Dead fish kinds 10/11/12 → Breeder stages 0/1/2.
- **Gumbo** (`pet-gumbo.md`): whether `board+0x174` (the lure flag) affects Breeders is unknown. The Breeder is not a Fish and doesn't have `+0x229`.
- **Seymour** (`pet-seymour.md`): whether his slow-fall rate applies to coins produced by Breeders is `static`.
- **Wadsworth** (`pet-wadsworth.md`): shelter only protects stage 0/1 guppies, not Breeders.
- **Walter** (`pet-walter.md`): Breeders (type `0x0a`) are not in the punch target list.
- **Food**: Breeders eat food pellets (confirmed by `alien-kind4.md` and `pet-nostradamus.md`).
- **Clicks**: expected mouse-invisible (`+0x55` = byte `0x005e8f14`, 0), as with Grubbers. Right-clicks pass to `0x0053bfb0` (Presto dialog, `pet-presto.md`).

`static`.

## Drawing

The sprite sheet is expected to be **`IMAGE_BREEDER`** (80×80 cels). The draw function address is not identified.

**Widget**: 80×80. The Breeder is drawn at its widget position with cels selected by animation state.

**Dead fish**: `IMAGE_BREEDER` death row (DeadFish kinds 10/11/12). The draw path for kinds ≥ 10 likely uses the Breeder's sprite sheet row for the stage, by analogy with kinds 8/9 using `IMAGE_GRUBBER`.

**Shop icon**: expected `IMAGE_SCL_BREEDER` (cel-animated, same pattern as other SCL images).

**Shadow**: type 0 (`board.md#shadows`).

`guess` (sprite sheet name and layout inferred by analogy; no draw function examined).

## RNG summary

| When | Stream | Draws |
|---|---|---|
| Shop purchase tick | app | position, ctor draws (unknown count), shadow, splash |
| Level start | app | 2 (position) + ctor draws + shadow |
| Rhubarb fling | app | 1 per flung Breeder |
| Per-tick (swim reroll) | app | expected same as Grubber |
| Guppy spawn | app | unknown (expected similar to Prego or Grubber Larva drop) |
| Die, remove, hunger | — | none |

`static` (purchase and level-start draws from `shop-bar.md` / `level-start.md`; Rhubarb from `pet-rhubarb.md`; swim and spawn draws are `guess`).

## Open questions

- **Constructor internals**: how many RNG draws, what fields are initialised (hunger range, divisor, swim mode, spawn timer). Requires the ctor or Init function in Ghidra.
- **BuyBreeder** (`0x005470a0`): full draw count, y override, any extra setup.
- **Update function**: address, full step order, food chase mechanics (bite box, eat effect, hunger gain/cap).
- **Guppy spawn mechanism**: timer fields, interval, whether it freezes during waves, RNG draws per spawn, spawn position relative to the Breeder.
- **Growth**: the Breeder has stages (0, 1, 2 from `+0x1a0`); whether it grows by eating food (like a guppy) or stays at one stage.
- **Drawing**: sprite sheet, cel layout, mirroring, hungry overlay.
- **Full field table**: offsets `+0x1b0`..`+0x1dc` are unidentified.
- **Coin drop**: whether the Breeder drops coins directly (unlikely; guppies from the Breeder drop them).
