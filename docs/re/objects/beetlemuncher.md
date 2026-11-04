# Beetlemuncher (`Sexy::Gekko`, type `0x07`)

Class `Sexy::Gekko`, vtable `0x005982c4`, board list `+0xcc`, object size `0x230` (same as `Fish`). A Gekko **is a `Fish`**: it uses `Fish::Update` (slot 22 @ `0x004f6970`, `fish-guppy.md#fishupdate-order-per-tick`) and `Fish::Draw` (slot 26 @ `0x004e4a90`) unchanged, and replaces a few virtual slots. It eats **Larvae** (Beetles, type `0x1d`, list `+0xc8`, dropped by the Guppycruncher) instead of food, and drops **kind 6 coins** ($500).

The second half of the tank-3 economy chain: Guppycruncher eats small guppy → drops Larva → **Beetlemuncher eats Larva → drops kind 6 coin**.

**Slot numbering** is 0-based (Ghidra's `vfunctionN` = slot `N − 1`). `Fish::UpdateHunger` is slot **82** (shared with Oscar).

**Offsets** are raw `this`-relative. Widget x/y are `+0x30`/`+0x34`.

RNG: `r` = app stream `MTRand::NextNoAssert` @ `0x0040aeb0`; `grand` = global `Sexy::Rand` @ `0x0040f9e0`.

`static`.

## Slot overrides

Compared with `Fish` (vtable `0x00597d54`, 91 slots):

| Slot | Gekko | Fish | Role |
|---|---|---|---|
| 0 | `0x004f0d10` | `0x004f0cb0` | destructor |
| 70 | `0x004ef990` | `0x0051a440` (no-op) | census counter |
| 77 | `0x004e1950` | `0x004f1ba0` | **eat** (victim = a Larva or food) |
| 79 | `0x004e1850` | `0x004dee90` | icon draw for UI |
| 81 | `0x004fbe80` | `0x004f1400` | **coin drop** (kind 6) |
| 82 | `0x004e1550` | `0x004f15c0` | **hunger** (shared with Oscar) |
| 83 | `0x004e17f0` | `0x004dea00` | draw body |
| 84 | `0x004d73a0` | `0x004d5b20` | **prey chase** |
| 85 | `0x004ea3f0` | `0x004ef050` | **find prey** |
| 86 | `0x004ea590` | `0x004ef1b0` | **bite check** |
| 87 | `0x004d9e60` | `0x004df0b0` | **remove** (no death messages) |

Shared and unchanged: 22 Update, 26 Draw, 55 mouse-down, 75 "eaten" (calls slot 87 with `true`), 88 Die `0x004d5f90`, 89 animation, 90 colour setup.

`static` (all slot addresses verified against the vtable at `0x005982c4` in the binary).

## Fields that differ from a guppy

| Offset | Guppy | Gekko | Notes |
|---|---|---|---|
| `+0x8c` type | 0 | **7** | |
| `+0x1a0` stage | 0..4 | **7** (constant) | No growth. Controls the `DeadFish` kind and the draw row |
| `+0x9c` hunger | `r % 200 + 400` | same (from `Fish::Init`) | Not overwritten by `Gekko::Init` |
| `+0x190` floor (clamp y ≤) | 370 | **360** | `> 320`, so no slow sinking |
| `+0x198` ceiling (clamp y ≥) | 95 | **105** | |
| `+0x229` | 1 | **0** | Disables follow-cursor, guppy click-to-feed and the alien "hide" branch |
| `+0x55` (widget mouse-visible) | 1 (0 with aliens) | **byte `0x005e8f14`** (0) | Clicks pass through |
| `+0x224` coin interval | `r % 200 + 150` | **`r % 250 + 200`** | Overwritten by `Gekko::Init` |

Everything else comes from `Fish::Init` (`fish-guppy.md#construction`): x bounds [10, 540], divisor ∈ {2.0, 1.8, 1.6}, `+0x1a8` (growth needed), initial swim mode, hunger-warning threshold `+0xa4` = 300.

`static` (field values confirmed from `Gekko::Init` bytecode at `0x004d72b0`).

## Construction (`Gekko::Gekko(x, y)` @ `0x004ef9a0`)

1. `Fish::Fish(x, y)` (`0x004eef00`): `GameObject` ctor, then `Fish::Init(x, y)` (`0x004de370`, **6 app draws** R1–R6, `fish-guppy.md#construction`), then stage = 0.
2. vtable = Gekko (`0x005982c4`).
3. `Gekko::Init` (`0x004d72b0`), in order:
   1. type `+0x8c` = 7; `+0x229` = 0; stage `+0x1a0` = 7; ceiling `+0x198` = 105; floor `+0x190` = 360.
   2. `+0x55` = byte `0x005e8f14` (0).
   3. **R7:** coin interval `+0x224` = `r % 250 + 200` (200..449) via `0x004d71d0` (in VT: `grand() % 200 + 4320`).

So the constructor costs **7 app draws** (Fish::Init R1–R6, then the interval draw R7).

Note: unlike Oscar, the Gekko does **not** overwrite hunger. It keeps `Fish::Init`'s R3 hunger value (`r % 200 + 400`, 400..599).

`static` (Init bytecode at `0x004d72b0` decoded directly from the PE).

## Spawn (`Board::BuyBeetlemuncher` @ `0x00545080`)

Called from the shop (slot 8, `sim/shop-bar.md`).

1. **RNG** `x = r % 520 + 20`; **RNG** `y = r % 265 + 105`.
2. `new Gekko(x, y)`: 7 draws (above).
3. **RNG** `vy (+0x170) = (double)(r % 5 + 23)` (23..27).
4. **RNG** next draw `t`. Before using it, set y (`+0x160`) = **40.0** (constant at `0x005965d8`) and widget y (`+0x34`) = **40**. Then vy-damp timer `+0x208 = t % 10 + 45` (45..54).
5. `AddGameObject(obj, 1)` → appended to `+0xcc`, WM add.
6. Shadow (`0x00544800`: type 7 → shadow kind **0**): `Shadow::Shadow` draws **once** (`shadow y = 410 + r % 5`).
7. `RebuildZOrder`.

**12 app draws** in total: x, y, R1–R6, interval, vy, damp, shadow.

**Shop purchase** (`0x00549380` case 8): `++(+0x4a4)` → `TrySpend(price)` → BuyBeetlemuncher → reveal 10, 11 → `SOUND_GROW` → **PlaySplash** (1 draw `r % 3`). **A purchase costs 13 app draws on its tick.**

**Splash-in.** Falls from y = 40.0 at vy 23..27, with vy × 0.9 per tick while `+0x208 > 0`. Bubbles while `+0x208 ≥ 31` and vy > 0: stage 7 is not 0 and type 7 is not 6, so the two-bubble branch: gate `r % n`, then 4 draws (`%60`, `%60`, `%40`, `%40`).

`static` (BuyBeetlemuncher bytecode at `0x00545080` decoded; y = 40.0 constant and draw sequence confirmed).

## Per-tick update (`Fish::Update` with these overrides)

`fish-guppy.md#fishupdate-order-per-tick` with these differences:

- **Step 2 behaviour branch.** Follow-cursor needs `+0x229`, so it never runs. Slot 82 (**Gekko hunger**, below) runs every tick. When it returns false, the swim-mode state machine runs (step 3) exactly as for guppies. The alien "hide" branch is never taken because `+0x229` is 0.
- **Step 5 coin** (slot 81 = **Coin drop**) still runs only when there are no aliens.
- **Step 8:** floor 360 > 320, so no `y −= 0.25`.
- **Step 9/10 clamps:** y ≤ 360, and y ≥ 105 when not splashing.
- **Step 10 bubbles:** the two-bubble branch (stage ≠ 0 and type ≠ 6; same as Carnivore).
- Steps 12–14 (animation, integrate, widget) are unchanged.

Gekkos are in `+0xcc`, which `RebuildZOrder` places after Larvæ and before Ultravores. They update after guppies, carnivores and Larvæ, and before Ultravores and Breeders.

`static`.

### Gekko hunger (slot 82 @ `0x004e1550`, shared with Oscar)

Identical to `carnivore.md#carnivore-hunger`:

1. Colour-fade step `0x004d6c50`.
2. If there are **no aliens** (`+0xb8` and `+0xf4` empty): hunger −= 1. If hunger == 304 (`+0xa4 + 4`): start fade. Clamp at −1000.
3. **Starving** (`0x004d69d0`): `+0xac < 0` and hunger ≤ 0 → **die** (slot 88, `true`), return false.
4. If hunger < 500: return the **prey chase** (slot 84).
5. Otherwise return false.

A Gekko dies on the tick its hunger reaches 0. An unfed Gekko lives for its starting hunger (400–599 ticks from `Fish::Init`), and starts hunting at hunger 499 (1–100 ticks after purchase). With aliens present, hunger is frozen but hunting continues.

`static`.

### Find prey (slot 85 @ `0x004ea3f0`)

If `+0xf0 ≠ 0`: runs the food-finder `0x004ea010` instead. That's never the case for a normal Gekko.

Otherwise, iterate the **Larva list `board+0xc8`** in list order, with `best = 100 000 000` and `prey = none`. For each Larva `l`:

- `dx = trunc((l.wx + 40) − (x + 40.0))` and `dy = trunc((l.wy + 40) − (y + 40.0))`.
- `d = dx·dx + dy·dy` (int).
- If `d < best` and `l` is alive (`+0x174 == 0` — the Larva's alive flag, `static`): `best = d`, `prey = l`.

Prey is the nearest Larva; on a tie the first in list order wins. **No RNG.**

`static` (function entry confirmed; the Larva filter condition needs `Larva::Update` details).

### Prey chase (slot 84 @ `0x004d73a0`)

1. `prey = FindPrey()` (slot 85). Runs every tick the chase runs.
2. `fx = trunc(x + 40.0)`, `fy = trunc(y + 45.0)` (ints). (`+0xf0 ≠ 0` would add 20 to fy; never for Gekkos.)
3. If `mode_timer (+0x1b0) < 3`: skip to step 6.
4. Else if there is no prey: return false.
5. Else steer: `mode_timer = 0`; `tx = prey.wx + 40`, `ty = prey.wy + 40` (target Larva centre). Constants depend on **hunger > 300** ("peckish") or **≤ 300** ("hungry"):

   **x** (first matching row; step applies only while vx is strictly inside the cap):

   | Condition | peckish | hungry |
   |---|---|---|
   | `fx > tx + 40` | `vx −= 1.3` | `vx −= 1.3` |
   | `fx < tx − 32` | `vx += 1.3` | `vx += 1.3` |
   | `fx > tx + 38` | `vx −= 0.2` | `vx −= 0.2` |
   | `fx < tx − 30` | `vx += 0.2` | `vx += 0.2` |
   | `fx > tx` | `vx −= 0.1` | `vx −= 0.05` |
   | `fx < tx` | `vx += 0.1` | `vx += 0.05` |
   | cap | 4.0 | 4.0 |

   **y** (first matching row; decreases need `vy > −3.0`, increases need `vy < 3.0`):

   | Condition | peckish | hungry |
   |---|---|---|
   | `fy > ty + 6` | `vy −= 0.5` | `vy −= 1.3` |
   | `fy < ty − 6` | `vy += 1.3` | `vy += 1.3` |
   | `fy > ty` | `vy −= 0.5` | `vy −= 0.5` |
   | `fy < ty` | `vy += 0.5` | `vy += 0.5` |

   Then speed bucket `+0x1bc` += 1 if < 5.
6. If there is prey: run the **bite check** (slot 86).
7. Return "chasing" iff there is prey.

Steering happens every 3rd tick. The Gekko aims (x + 40, y + 45) at the Larva's centre. **No RNG.**

`static` (chase function bytecode decoded; x thresholds 40/32/38/30 and y constants from the FPU constant references; the exact x thresholds need confirmation — the byte-level decode is partially reconstructed from comparison patterns).

### Bite check (slot 86 @ `0x004ea590`)

The `+0xf0 ≠ 0` branch runs the food bite check; never for normal Gekkos.

Otherwise:
1. `cx = x + 40.0`, `cy = y + 45.0` (doubles).
2. Iterate the **Larva list** in order. For each Larva `l`:
   - Call `0x004d7740(l)` to check whether the Larva is within the bite box and alive.
   - If result `== 1`: **eat** (slot 77 with `l`), then remove the Larva (slot 75 with `true`). Set `eat_timer = 8`. **Stop iterating.**
   - If result `== 2`: `eat_timer = 20` (mouth open). Keep iterating.

So **at most one Larva is eaten per Gekko per tick**, first in list order. **No RNG** here; the eat (slot 77) may draw.

`static` (bite-check function entry confirmed; the exact bite box dimensions are in the `0x004d7740` helper).

## Eat (slot 77 @ `0x004e1950`)

Called with the eaten object as argument.

1. `was_hungry = 0x004d6f30()` (same as Carnivore).
2. `0x004d6a30(0)`: reset feeding state (`+0xf8 = 0`, `board+0x4ee = 1`).
3. VT: extra call (`0x004d6f90`).
4. If the eaten object's type is `0x1c` (Food): **SLURP** sound (`0x005384c0`, 1 app draw `r % 5`).
5. If the eaten object is not Food (i.e. a Larva): **CHOMP** sound (`0x00538560`, 1 app draw `r % 3` if off cooldown).
6. **hunger += 700; if hunger > 1000, hunger = 1000.**
7. `0x004e13d0(was_hungry)`: if the fish looked hungry before and doesn't now, start the fed fade.

**No gore burst.** Unlike the Carnivore, the Gekko eats cleanly.

Draws per eat: 1 (chomp or slurp).

`static` (Eat bytecode at `0x004e1950` decoded; hunger +700 cap 1000 confirmed from the `add [esi+0x9c], 0x2bc` and `cmp eax, 0x3e8` instructions).

## Coin drop (slot 81 @ `0x004fbe80`)

Runs from `Fish::Update` step 5, only with no aliens.

1. `coin_timer (+0x220) += 1`.
2. For a normal Gekko (`+0x124 != 2`): `0x004d7100(&coin_timer, interval +0x224)`, returning `coin_timer >= interval`.
3. On a drop: `coin_timer = 0`. If `0x004d6bd0` allows it (always outside VT): `Board::AddCoin(wx + 5, wy + 10, kind 6, 0, vy −1.0, owner 0)`.

Kind 6 = **$500** (`coin.md`, `Coin::GetValue` @ `0x004d53a0`). The coin falls normally. **No RNG** in the drop.

The interval is `Gekko::Init` R7: `r % 250 + 200`. So **one $500 coin every 200–449 ticks** (5.6–12.6 s) of alien-free time, fixed per Gekko.

`static` (CoinDrop bytecode decoded; kind 6, vy -1.0 at `0x005c3d80`, the interval path confirmed).

## Death

### Starvation (slot 88 `Fish::Die` @ `0x004d5f90`, shared)

1. **Die sound** `0x00538610(type)`: sample `DIE` `0x117`, 3-tick throttle. Type 7 → pitch adjustment (same mechanism as other fish types). No RNG.
2. Slot 87 with `false` → **Gekko remove** (below). Shadow is kept and passed on.
3. `Board::AddDeadFish(trunc x, trunc y, vx, vy, divisor, stage = 7, facing right = vx > 0, shadow)`: a `DeadFish` of **kind 7**. Kind 7 isn't 6/8/9, so it behaves like a guppy's: y clamped at 380, life counts down while y > 370, standard cel rules.

**No RNG.**

`static`.

### Gekko remove (slot 87 @ `0x004d9e60`)

Used by both starvation (argument `false`) and being eaten (argument `true`, via slot 75).

1. Detach a missile at `+0x98` if any.
2. Remove from the widget manager, the app registry, and `board+0xcc`.
3. Delete the shadow if the argument is `true`.

**No death messages** (unlike Oscar's 1-3 hint chain).

`static`.

### Eaten by an alien

Types 5–10 are always prey (`alien-sylvester.md#31-nearest-prey`). The alien calls slot 75 → slot 87 (`true`): the Gekko is removed with no `DeadFish`, shadow deleted.

`static`.

### Killed by Shrapnel

A Shrapnel bomb iterates `board+0xcc` (Gekkos) and calls slot 88 (Die) on each in the bomb's box (`pet-shrapnel.md#bomb-update`). Die sound, then a `DeadFish` corpse of kind 7.

`static`.

## Food and clicks

- **Food:** Gekkos never look at the food list (unless `+0xf0` is set, which it isn't for normal ones). FindPrey reads only `board+0xc8` (Larvæ).
- **Clicks:** `+0x55` is 0, so the widget manager doesn't deliver mouse events. Clicks pass through.

`static`.

## Drawing (slot 83 @ `0x004e17f0`)

The image is **`IMAGE_GEKKO`** (its own sprite sheet, not shared with `IMAGE_SMALLSWIM`). The DrawBody function at `0x004e17f0` is Gekko-specific. The Gekko is **never mirrored** in the same way as the Grubber: direction shows only in the animation order.

**Shop icon**: `IMAGE_SCL_GEKKO`, animated.

**Dead fish**: `IMAGE_SMALLDIE` row corresponding to kind 7 (`DeadFish` draw path).

`static` (the image name confirmed by the asset browser; the draw function details need decompilation).

## Interactions

- **Pets**: Shrapnel bombs kill Gekkos (`+0xcc`, slot 88). Rhubarb flings Gekkos (`pet-rhubarb.md`: `board+0xcc`, vy −20/−30). Angie revives Gekkos (`0x005451c0(wx, wy, facing)`). Walter can punch Gekkos. Gumbo's lure interaction with Gekkos is unknown.
- **Larvæ**: Gekkos eat them; the Larva is removed (slot 75) when eaten.
- **Aliens**: Gekkos are always valid prey (types 5–10). Hunger frozen with aliens present.

`static`.

## RNG summary

| When | Stream | Draws, in order |
|---|---|---|
| Purchase tick | app | x `%520`, y `%265`, `Fish::Init` R1–R6, interval `%250`, vy `%5`, damp `%10`, shadow `%5`, splash `%3` (13) |
| Splash-in (`+0x208` ≥ 31, vy > 0) | app | gate `% n`; if 0: `%60`, `%60`, `%40`, `%40` (+ bubbles on global) |
| Swim modes (not chasing) | app/global | as guppy |
| Every 21st tick | app | as guppy: `%10`, w.p. 1/10 `%9` |
| Eat | app | chomp `%3` (if off cooldown) — or slurp `%5` for food |
| Find prey, chase, bite, coin drop, hunger, death, remove | – | none |

`static`.

## Open questions

- The exact steering thresholds in the chase function (the x thresholds 40/32/38/30 are reconstructed from the comparison patterns and need verification).
- The bite box dimensions (inside `0x004d7740`).
- Whether `IMAGE_GEKKO` has the same cel grid as `IMAGE_GRUBBER` (10×5, 80×80).
- The pitch adjustment for the DIE sound with type 7.
- Angie revival draw count (`0x005451c0`).
- The `+0x124 == 2` path in the coin drop function (pet-linked Gekkos; not relevant for shop-bought ones).
