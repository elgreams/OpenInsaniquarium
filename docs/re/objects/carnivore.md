# Carnivore (`Sexy::Oscar`, type `0x05`)

Status: `traced`. `tank1_1.rs` compares every carnivore field and the app RNG each tick against two captures: `tank_1_3_from_start` (`tank1-3_full_01`: bought, a diamond, starved, the 1-3 death message) and `tank_1_3_carnivore_eats` (`tank1-3_eat_01`: a carnivore kept fed with fresh small guppies through six meals: hunt, bite, eat to hunger 1300, chomp and gore draws, diamonds, alien waves). Sections not exercised by either are marked `static`.

Class `Sexy::Oscar`, vtable `0x0059843c`, board list `+0xa4`, object size `0x230` (same as `Fish`). An Oscar **is a `Fish`**: it uses `Fish::Update` (slot 22 @ `0x004f6970`, `fish-guppy.md#fishupdate-order-per-tick`) and `Fish::Draw` (slot 26 @ `0x004e4a90`) unchanged, and replaces a few virtual slots. It eats small guppies instead of food, drops diamonds, and dies as soon as its hunger runs out.

**Slot numbering.** Slots are 0-based vtable indices (the slot at byte offset `4·n` is slot `n`). Ghidra's `vfunctionN` names are 1-based (`vfunctionN` = slot `N − 1`). By this numbering, `Fish::UpdateHunger` `0x004f15c0` is slot **82** and `Fish::DrawBody` `0x004dea00` is slot **83** (`food.md` uses this numbering; `fish-guppy.md` labels both 83).

**Offsets** are raw `this`-relative. Widget x/y are `+0x30`/`+0x34`. That's what the code reads, e.g. the coin drop at `[+0x30] + 5`, `[+0x34] + 10`.

RNG: `r` = app stream `MTRand::NextNoAssert` @ `0x0040aeb0` (31-bit output, `framework/rng.md`); `grand` = global `Sexy::Rand` @ `0x0040f9e0`.

Confidence: per section below.

## Slot overrides

| Slot | Oscar | Fish | Role |
|---|---|---|---|
| 0 | `0x004f03a0` | `0x004f0cb0` | destructor |
| 70 | `0x004ea730` | `0x0051a440` (no-op) | census counter (below) |
| 77 | `0x004f2b00` | `0x004f1ba0` | **eat** (victim = a guppy) |
| 79 | `0x004e2800` | `0x004dee90` | icon draw for UI (row 4) |
| 81 | `0x004d8560` | `0x004f1400` | **coin drop** (diamond) |
| 82 | `0x004e1550` | `0x004f15c0` | **hunger** |
| 83 | `0x004e2480` | `0x004dea00` | draw body |
| 84 | `0x004d85d0` | `0x004d5b20` | **prey chase** |
| 85 | `0x004eb080` | `0x004ef050` | **find prey** |
| 86 | `0x004eb230` | `0x004ef1b0` | **bite check** |
| 87 | `0x004f2d30` | `0x004df0b0` | **remove** (with 1-3 death messages) |

Shared and unchanged: 22 Update, 26 Draw, 55 mouse-down `0x004e4c10` (never reached, see Construction), 75 `0x004f01d0` ("eaten": calls slot 87 with `true`), 88 Die `0x004d5f90`, 89 animation `0x004de840`, 90 colour setup `0x004d5650`.

Slot 82 `0x004e1550` is also used by `Sexy::Gekko`; Ghidra names it `Gekko::vfunction83`.

`static`.

## Fields that differ from a guppy

| Offset | Guppy | Carnivore | Notes |
|---|---|---|---|
| `+0x8c` type | 0 | **5** | |
| `+0x1a0` stage | 0..4 | **5** (constant) | No growth. The `DeadFish` kind and the bubble branch read it |
| `+0x9c` hunger | `r % 200 + 400` | **`r % 200 + 600`** | Overwritten after `Fish::Init` |
| `+0x190` floor (clamp y ≤) | 370 | **360** | `> 320`, so no slow sinking (`Fish::Update` step 8) |
| `+0x198` ceiling (clamp y ≥) | 95 | **105** | |
| `+0x229` | 1 | **0** | Disables follow-cursor, the guppy click-to-feed and the alien "hide" branch |
| `+0x55` (widget mouse-visible) | 1 (0 with aliens) | **byte `0x005e8f14`** | Nothing in the binary writes `0x005e8f14` (0 in the image). Carnivores never take clicks |
| `+0x224` coin interval | `r % 200 + 150` | same value | Not overwritten. Now the **diamond** interval |

Everything else comes from `Fish::Init` (`fish-guppy.md#construction`): x bounds [10, 540], divisor ∈ {2.0, 1.8, 1.6}, `+0x1a8`, initial swim mode, hunger-warning threshold `+0xa4` = 300. Numeric types are as for guppies: position, velocity and divisor are `double`; the rest are `int`.

`traced` for the fields compared (`fish_json`: hunger, bounds, stage, coin interval); `+0x55` and the census are `static`.

## Construction (`Oscar::Oscar(x, y)` @ `0x004efda0`)

1. `Fish::Fish(x, y)` (`0x004eef00`): `GameObject` ctor, then `Fish::Init(x, y)` (`0x004de370`, **6 app draws** R1–R6, `fish-guppy.md#construction`), then stage = 0.
2. vtable = Oscar.
3. `Oscar::Init` (`0x004d84c0`), in order:
   1. type `+0x8c` = 5; stage `+0x1a0` = 5; `+0x229` = 0; ceiling `+0x198` = 105; floor `+0x190` = 360.
   2. **R7:** hunger `+0x9c = r % 200 + 600` (600..799).
   3. `+0x55` = byte `0x005e8f14` (0).
   4. Virtual Tank only (`app+0x880`): `grand() % 12 + 30` → slot 90 (colour), `+0x1d0 = 1`. Not in Adventure.

So the constructor costs **7 app draws**.

`traced` (draw count and values via `rng_mti` and the carnivore's fields on its first tick).

## Spawn (`Board::BuyCarnivore` @ `0x00544c00`)

Called from the shop (slot 4, `sim/shop-bar.md`, `0x0054958d`), the debug key `'2'` (`0x00549df7`), and `0x005474c0` (case 3 of the respawner `0x00547530`: `SOUND_GROW`, PlaySplash (1 draw), then BuyCarnivore; afterwards the respawner sets `+0x120 = 40`).

1. **RNG** `x = r % 520 + 20`; **RNG** `y = r % 265 + 105`.
2. `new Oscar(x, y)`: 7 draws (above).
3. **RNG** `vy (+0x170) = (double)(r % 5 + 23)` (23..27). This is an unsigned→double conversion; the 2³² fixup is unreachable.
4. **RNG** next draw `t`. Before using it, set y (`+0x160`) = **40.0** and widget y (`+0x34`) = **40**. Then vy-damp timer `+0x208 = t % 10 + 45` (45..54).
5. `AddGameObject(obj, 1)` → appended to `+0xa4`, WM add. Update the max-carnivores statistic `board+0x46c` = max(itself, list length).
6. Shadow (`0x00544800`: type 5 → shadow kind 0): `Shadow::Shadow` draws **once** (`shadow y = 410 + r % 5`, `fish-guppy.md#spawn`).
7. `RebuildZOrder`.

**12 app draws** in total, all in this order: x, y, R1–R6, hunger, vy, damp, shadow.

**Shop purchase** (`0x00549380` case 4): `TrySpend(price)` → BuyCarnivore → shop reveals (`0x005409b0`, no RNG): tank 4 level > 1 reveals 9, 10, 11; tank 4 level 1 reveals 11; any other tank reveals 10, 11. Then `SOUND_GROW` (`0x122`, 3-tick throttle, no RNG), then **PlaySplash** `0x00538360`: **1 app draw**, `r % 3` = 0/1/2 → `SPLASH` `0x13d` / `SPLASH2` `0x13e` / `SPLASH3` `0x13f`. It is not throttled and always draws. **A purchase costs 13 app draws on its tick.**

Compared with the guppy spawn (`fish-guppy.md#spawn`): there is one more ctor draw (hunger), vy is `% 5 + 23` instead of `+ 18`, and the start y is 40.0 instead of 30.0.

**Splash-in.** As with a bought guppy, the fish falls in at 23–27 px/step, with vy ×0.9 per tick while `+0x208` > 0. Bubbles: while `+0x208 ≥ 31` and vy > 0, `Fish::Update` step 10 runs. Stage 5 is not 0 and type 5 is not 6, so it takes the **two-bubble branch**: gate draw `r % n` (n = 1 while the timer > 40, so always emitted; else n = 2), then **4 draws**: `x₁ = wx + 55 − r % 60`, `y₁ = wy + 55 − r % 60`, `x₂ = wx + 45 − r % 40`, `y₂ = wy + 45 − r % 40`. Each pair is followed by `Board::AddBubble` (global stream, not modelled). So the first ~10–14 ticks cost 1 + 4 draws each, then 0 or 4 + 1 while the timer is in 31..40.

`traced` (13 draws on the purchase tick, start y 40, splash-in). The respawner and debug-key callers are `static`.

## Per-tick update (`Fish::Update` with these overrides)

`fish-guppy.md#fishupdate-order-per-tick` with these differences:

- **Step 2 behaviour branch.** Follow-cursor needs `+0x229`, so it never runs. Slot 82 (**Carnivore hunger**, below) runs every tick. When it returns false, the swim-mode state machine runs (step 3) exactly as for guppies. The alien "hide" branch (aliens present, `board+0x174` set, `+0x229` set) is never taken, because `+0x229` is 0.
- **Step 5 coin** (slot 81 = **Diamond drop**) still runs only when there are no aliens (`0x004da780`).
- **Step 8:** floor 360 > 320, so no `y −= 0.25`.
- **Step 9/10 clamps:** y ≤ 360, and y ≥ 105 when not splashing.
- **Step 10 bubbles:** the two-bubble branch (see Spawn).
- Steps 12–14 (animation, integrate, widget) are unchanged. The eat timer `+0x1c8` set by the bite check drives the eat cels as for guppies.

Carnivores are in `+0xa4`, which `RebuildZOrder` places right after guppies (`+0xa0`). So each tick, every guppy updates before any carnivore, and carnivores update in purchase order (`sim/board.md`).

`traced` without prey.

### Carnivore hunger (slot 82 @ `0x004e1550`)

1. Colour-fade step `0x004d6c50` (as for guppies, `food.md`).
2. If there are **no aliens** (`+0xb8` and `+0xf4` empty): `0x004d6ef0`. While `+0x11c == −1`, hunger −= 1. If hunger == 304 (`+0xa4 + 4`), start the turning-hungry fade (`+0xa8 = 1`, `fade = 1`). Otherwise, if hunger < −1000, clamp it at −1000.
3. **Starving** (`0x004d69d0`): `+0xac < 0` and hunger ≤ 0. In Virtual Tank the threshold is hunger ≤ −2160 instead. If starving: **die** (slot 88, `true`) and return false.
4. If hunger < 500: return the **prey chase** (slot 84).
5. Otherwise return false.

Differences from `Fish::UpdateHunger`: no hungry messages at −1/−200/−400, no −500 grace, no `+0x228` check, and no "first alien is type 4" check before chasing. **A carnivore dies on the tick its hunger reaches 0.** An unfed carnivore therefore lives exactly its starting hunger (600–799 ticks, 16.8–22.4 s), and starts hunting at hunger 499 (101–300 ticks after purchase). With aliens present, hunger is frozen but hunting continues.

`traced` (hunger countdown and death at 0). The hunt branch below 500 ran with no small guppies (no prey), which is also traced: no chase, swim modes instead.

### Find prey (slot 85 @ `0x004eb080`)

If `+0xf0 ≠ 0`, it runs the food-finder `0x004ea010` instead. That's never the case for a carnivore (`+0xf0` = 0 from the GameObject ctor).

Otherwise, iterate the **guppy list `board+0xa0` in list order**, with `best = 100 000 000` and `prey = none`. For each guppy `g` (widget `gx = g+0x30`, `gy = g+0x34`):

- `dx = trunc((gx + 40) − (x + 40.0))` and `dy = trunc((gy + 40) − (y + 40.0))`. Both are double subtractions (`int` → double), truncated toward zero (`_ftol2`).
- `d = dx·dx + dy·dy` (int).
- If `d < best` **and** `g.stage (+0x1a0) == 0` **and** `g+0xac < 0` **and** `g+0x120 ≤ 0`: `best = d`, `prey = g`.

So the prey is the nearest **small** guppy, and on an exact tie the **first in list order** wins (strict `<`). Medium/large/king guppies, carnivores and other fish are never prey. `+0x120` is a spawn-grace countdown: the census respawner `0x00547530` sets it to 40, and `PreUpdate` decrements it. `+0xac ≥ 0` is a Virtual-Tank state.

After the loop: if `best < 10000` (within 100 px), call `0x004d6b00(150)`. It does nothing unless byte `+0x104` is set; when it is set, it plays `PRIMALSCREAM` `0x10e` and sets `+0x108 = 150`, `+0x10c = 80`. A plain carnivore never has it. Then set `+0xf8 = 100`. `+0xf8` only changes the divisor when byte `+0xf6` is set (`Fish::Update` step 13), which it isn't. `PreUpdate` decrements it.

Return `prey`. **No RNG.**

`traced` (`tank_1_3_carnivore_eats`).

### Prey chase (slot 84 @ `0x004d85d0`)

1. `prey = FindPrey()` (slot 85). This runs every tick the chase runs, even when steering is skipped.
2. `fx = trunc(x + 40.0)`, `fy = trunc(y + 50.0)` (ints). `+0xf0 ≠ 0` would add 20 to fy; never for carnivores.
3. If `mode_timer (+0x1b0) < 3`: skip to step 6.
4. Else if there is no prey: return false.
5. Else steer: `mode_timer = 0`; `tx = prey.wx + 40`, `ty = prey.wy + 40` (ints). The constants depend on **hunger > 300** ("peckish") or **≤ 300** ("hungry"):

   **x** (first matching row; the step applies only while vx is strictly inside the cap, i.e. `vx > −cap` for a decrease and `vx < cap` for an increase):

   | Condition | peckish | hungry |
   |---|---|---|
   | `fx > tx + 8` | `vx −= 1.5` | `vx −= 1.5` |
   | `fx < tx − 8` | `vx += 1.5` | `vx += 1.5` |
   | `fx > tx + 4` | `vx −= 0.2` | `vx −= 0.2` |
   | `fx < tx − 4` | `vx += 0.2` | `vx += 0.2` |
   | `fx > tx` | `vx −= 0.1` | `vx −= 0.05` |
   | `fx < tx` | `vx += 0.1` | `vx += 0.05` |
   | cap | 4.0 | 5.0 |

   **y** (first matching row; decreases need `vy > −3.0`, increases need `vy < 4.0`, in both moods):

   | Condition | peckish | hungry |
   |---|---|---|
   | `fy > ty + 6` | `vy −= 0.8` | `vy −= 1.3` |
   | `fy < ty − 6` | `vy += 1.3` | `vy += 1.5` |
   | `fy > ty` | `vy −= 0.3` | `vy −= 0.5` |
   | `fy < ty` | `vy += 0.5` | `vy += 0.7` |

   Comparisons are int vs. int for positions, and double vs. double constant for the caps (all as written above, with no epsilon). A fish already beyond a cap keeps its speed: there's no clamp.

   Then the speed bucket `+0x1bc` += 1 if < 5.
6. If there is prey: run the **bite check** (slot 86).
7. Return "chasing" iff there is prey.

`mode_timer` is incremented once per tick by `Fish::Update` step 4, so steering happens on every 3rd tick while chasing (the same rhythm as a guppy's food chase). The carnivore aims its point (x + 40, y + 50) at the guppy's centre. **No RNG.**

`traced` (`tank_1_3_carnivore_eats`).

### Bite check (slot 86 @ `0x004eb230`)

The `+0xf0 ≠ 0` branch (food bites via `0x004ea2a0`) is never taken by carnivores.

`cx = x + 40.0`, `cy = y + 45.0` (doubles). Iterate the **guppy list in order**. For each guppy `g` (`gx`, `gy` = widget ints):

1. **Bite box**: `gx + 10 < cx < gx + 70` and `gy + 22 < cy < gy + 58` (all strict), and `g.stage == 0`.
   - If g is also available (`g+0xac < 0` and `g+0x120 ≤ 0`): **eat** (slot 77 with `g`), then **remove g** (g's slot 87 with `true`). If `eat_timer (+0x1c8) == 0`, set it to 8 (after a no-op `0x004d6cc0`). **Stop iterating.**
   - If g is not available: skip to the next guppy. The mouth-open test is not tried for it.
2. Otherwise (not in the bite box, or stage ≠ 0), if `eat_timer == 0`: **mouth-open box** `gx − 7 < cx < gx + 87` and `gy + 10 < cy < gy + 70`, `g.stage == 0`, and g available. If so: `eat_timer = 20`. Keep iterating.

So **at most one guppy is eaten per carnivore per tick**, and it is the first one in list order whose box the carnivore's mouth is in. That need not be the guppy it was chasing. **No RNG** here; the eat (slot 77) draws.

**Removing the guppy** (Fish slot 87 @ `0x004df0b0`, `true`): detach a missile at `+0x98` if any; remove it from the widget manager, the app's object registry and its board list (`0x00541d90`); delete its shadow; `board+0x488 += 1`. **No `DeadFish`, no sound, no RNG.** This is the same path as an alien eating a guppy (`alien-sylvester.md#32-steering-and-eating-0x004ed460--0x004e8f70`). Because guppies update before carnivores, the eaten guppy has already had its update this tick.

`traced` (`tank_1_3_carnivore_eats`: six eats; the eaten guppy disappears with no corpse).

### Eat (slot 77 @ `0x004f2b00`, argument = the victim)

1. **Chomp sound** `0x00538560(byte +0x104)`: if sample `0x113` is off its 3-tick cooldown (`0x005381f0`, `board+0x28` clock), **1 app draw** `r % 3`: 0 → `CHOMP2` `0x114`, 1/2 → `CHOMP` `0x113`. (If the byte argument were set, the sample would be `SLURP` `0x138` with pitch −18. Not for plain carnivores.) There is no draw while throttled.
2. `was_hungry_look = 0x004d6f30()`: hunger < 301, fade `+0xa0` == 0, and (`+0x148 < 3` or `board+0x4ff`).
3. `0x004d6a30(0)`: `+0xf8 = 0`, `board+0x4ee = 1`. Its "hunger ≥ 300" floor and `+0x110` branch apply only when `+0xac ≥ 0` or in Virtual Tank. If `+0xac ≥ 0`, it also calls `0x004d6f90` (Virtual Tank feeding clock). Neither applies in Adventure.
4. **hunger += 900; if hunger > 1300, hunger = 1300.**
5. **Gore burst**, if byte `app+0x882` is set (on by default, `alien-sylvester.md`) and the victim's type isn't `0x1c`: **1 draw** `n = r % 3 + 2`; then n times: **draw** `y = r % 20 + 15 + victim.wy`, **draw** `x = r % 20 + 15 + victim.wx` (y first), then `EffectShot(x, y, 1)` (`0x005436f0`). An effect shot is created only if there are fewer than 20 shots, and its ctor costs 1 more draw, plus a heap-dependent extra draw (see `alien-sylvester.md#deviations`). The coordinates are always drawn, even when no shot is created.
6. `0x004e13d0(was_hungry_look)`: if the fish looked hungry before and doesn't now, `+0xa8 = 0`, `fade = 5` (fade back to the fed colours).
7. Only when `+0x124 == 3` (a pet-linked fish; plain carnivores have −1 from the GameObject ctor): `grand() % 3 == 0` spawns the floating texts "This is my last pizza," / "I swear!". Not in normal play.

Nothing else changes: **no growth** (`+0x1a4` untouched), **no coin-timer change**, no stage change.

Draws per eat (normal game, `app+0x882` set): `[chomp]` + 1 + n × (2 + effect-shot ctor draws), with n ∈ 2..4. That is 6–15 draws, plus the heap extras.

`traced` for steps 1, 4 and 5 (chomp and gore draws, hunger +900 capped at 1300). Steps 6–7 are `static`.

### Diamond drop (slot 81 @ `0x004d8560`)

Runs from `Fish::Update` step 5, only with no aliens on the board.

1. `coin_timer (+0x220) += 1`. Unlike the guppy version: no stage gate and no `0x005e89cc` (35-tick) override.
2. `0x004d7100(&coin_timer, interval +0x224)`. In the normal game this is `coin_timer ≥ interval`. Virtual Tank has its own rules there.
3. On a drop: `coin_timer = 0`. If `0x004d6bd0` allows it (always outside Virtual Tank; in Virtual Tank it needs hunger ≥ 1): `Board::AddCoin(wx + 5, wy + 10, kind 4, 0, vy −1.0, owner 0)`, the same arguments as a guppy coin except the kind.

Kind 4 = **diamond**, $200 (`coin.md#values-0x004d53a0-normal-game`), list `+0xa8`, with the normal fall/rest/expiry. Its collect sound is fixed (no RNG). **No RNG** in the drop. No coin hint messages: those live in the guppy version's tail, `0x004f1502`.

The interval is `Fish::Init`'s R6, `r % 200 + 150`. So **one diamond every 150–349 ticks (4.2–9.8 s)** of alien-free time, fixed per carnivore, with the first after the same delay. The timer keeps counting while the carnivore is hungry or chasing.

`traced` (one diamond after 340 ticks = the carnivore's `+0x224`).

## Death

### Starvation (slot 88 `Fish::Die` @ `0x004d5f90`, shared)

1. **Die sound** `0x00538610(type)`: sample `DIE` `0x117`, 3-tick throttle. Type 5 → `AdjustPitch(−6)` (guppies: 0). No RNG.
2. Slot 87 with `false` → **Carnivore remove** (below). The shadow is kept and passed on.
3. `Board::AddDeadFish(trunc x, trunc y, vx, vy, divisor, stage = 5, facing right = vx > 0, shadow)`: a `DeadFish` of **kind 5**. It behaves like a guppy's (`fish-guppy.md#dead-fish`): kind 5 isn't 6/8/9, so y is clamped at most 380, life counts down while y > 370, and the cel rules are the guppy ones. It is drawn from `IMAGE_SMALLDIE` **row 4** (src y 320), mirrored when facing right.

**No RNG.**

`traced` (death, the `DeadFish` and its update).

### Carnivore remove (slot 87 @ `0x004f2d30`)

Used by both starvation (argument `false`) and being eaten (argument `true`, via slot 75).

1. Same as Fish slot 87: detach a missile at `+0x98`, remove it from the WM, the app registry and `board+0xa4`, and delete the shadow if the argument is `true`.
2. **`board+0x468 += 1`** (carnivores lost; guppies count in `+0x488`).
3. **Tank 1-3 hint chain**, only when `0x00537bb0(1, 3)` holds (tank 1, level 3, `profile+0x59` clear). `ShowMessage(text, flash = 0, id)` (`sim/board.md`), called in this order:
   1. if one-time byte `0x1c` is set (`board+0x4cc`): "Try feeding small guppies to your carnivores!", id `0x1d`;
   2. if byte `0x1b` is set (`board+0x4cb`): "Hint: Carnivores won't eat fish food!", id `0x1c`;
   3. "Warning! Your carnivore has died!", id `0x1b`.

   Only unused ids take effect (the timer is 185). Result: the **1st** carnivore death on 1-3 shows "Warning! Your carnivore has died!", the **2nd** shows "Hint: Carnivores won't eat fish food!", the **3rd** shows "Try feeding small guppies to your carnivores!", and later deaths show nothing new. There's no "message line idle" check, so the hint replaces whatever is showing.

`traced` for the first death on 1-3 ("Warning! Your carnivore has died!", id 0x1b). The 2nd/3rd messages are `static`.

### Eaten by an alien

Sylvester's prey filter (`alien-sylvester.md#31-nearest-prey-0x004e8b60`) accepts types 5..10 **always**: carnivores are prey, and they aren't covered by the protector-pet exemption that small guppies get. The eat box uses half-extents 45/65 (the Ultravore's 70/85 apply to type 6 only), against the carnivore's 80×80 widget centre. On a hit the alien calls the victim's slot 75 → **slot 87 (`true`)**: the carnivore is removed with no `DeadFish`, `board+0x468` += 1, and on 1-3 **the death-hint chain above also fires**.

Carnivores also count as "fish" for the empty-tank check (`0x005392a0` looks at `+0xa0`, `+0xa4`, `+0xd0`, …, `fish-guppy.md#death-and-the-empty-tank`) and for "alien has prey".

`static` (no carnivore was eaten by an alien in either capture).

## Food and clicks

- **Food:** carnivores never look at the food list. FindPrey reads only `board+0xa0`, and the bite check only guppies, so pellets pass through them.
- **Clicks:** `+0x55` is 0, so the widget manager doesn't deliver mouse events to a carnivore, and a click on one goes to whatever is below it (another fish, or the board's food drop). Even if one arrived, `+0x229 = 0` makes `Fish` slot 55 return at once. `+0xa4` is also not in the list that `0x00539ad0` toggles when aliens come and go. (`+0x55` as "mouse visible" follows `screens/hatch-screen.md`; `guess` for the behaviour.)

`static`.

## Drawing

`Fish::Draw` (slot 26, shared) picks the mirror flag exactly as for guppies, then calls slot 83:

**`Oscar` DrawBody (slot 83 @ `0x004e2480`):**
- If global byte `0x005e89ce` ("Zombie Mode", toggled by a debug key with the texts "Zombie Mode Enabled/Disabled", seeded from a profile bit in the Board ctor): draw `IMAGE_SMALLDIE` at cel 9 (fed) or cel 6 (hungry look), row 4 (src (cel·80, 320, 80, 80)), mirrored. Then the missile and hunger-icon overlays below.
- Otherwise, src rect = (**cel `+0x1c0` · 80, 320, 80, 80**): **row 4** of the same sheets the guppy uses, chosen by `Fish::GetSwimImage(hungry_look)` (`fish-guppy.md#animation-and-drawing`): `SMALLTURN`/`HUNGRYTURN` while turning, `SMALLEAT`/`HUNGRYEAT` while `+0x1c8 ≥ 1` or `+0x108 ≥ 101`, else `SMALLSWIM`/`HUNGRYSWIM`. The row is fixed at 4 and does not follow the stage.
  - Byte `+0xfc` set: the backdrop cut-out pre-pass `0x004d6e70` (may suppress the draw). Not used by plain carnivores.
  - Byte `+0x1d0` set and not hungry (Virtual Tank colouring): `IMAGE_SIMCARNIVORE` in 4 colourised layers (rows chosen by turn/eat state). Virtual Tank only.
  - Fade overlay: while `+0xa0 > 0`, the hungry-look sheet is drawn again over it at alpha `fade·255/5`.
- If a missile is attached (`+0x98`): its overlay (`0x004d7040`).
- If `0x004d6980(500)` holds (hunger < 500 and `board+0x178` ≠ 0, plus the `+0x148`/`board+0x4ff` rule): `IMAGE_MISCITEMS` cel 2 at (0, −10), a "hungry" marker. `board+0x178` is the same unknown flag as in `alien-sylvester.md`.

**Slot 79 @ `0x004e2800`** (an icon-style draw; callers not identified, presumably UI): `IMAGE_SMALLSWIM`, src (`+0x130`·80, 320, 80, 80), with a per-style translate (style 0: (17, 17), 1: (7, 17), 2: (17, 0), 3: (8, 2), 4: (−40, 0)). With `+0x1d0` it uses `IMAGE_SIMCARNIVORE` layers instead. Not part of the tank draw.

**Slot 70 @ `0x004ea730`**: increments the int its argument points to. It is called by the object-census visitor `0x004d7210` for objects with byte `+0xf4` set and `+0xf0 == 0`. That visitor runs via `0x0053a3c0`, over the master set, from the button handler `0x005492b0`, which compares the census with `0x005397a0` and respawns kinds through `0x00547530`. So a carnivore counts in census slot 0. Not part of the tank update; the purpose of that button path is unclear. (Guppies' slot 70 is a no-op.)

`traced` for the plain swim cels, row 4, in front of the guppies (`frames.rs` `tank_1_3_carnivore_frames_match_original`: eight frames of `tank13_carnivore_01`, pixel-exact). Turning, eating, the fade, the hungry marker, Zombie Mode, the Virtual Tank layers and slots 79/70 are `static`.

## RNG summary

| When | Stream | Draws, in order |
|---|---|---|
| Purchase tick | app | x `%520`, y `%265`, `Fish::Init` R1–R6, hunger `%200`, vy `%5`, damp `%10`, shadow `%5`, splash `%3` (13) |
| Splash-in (`+0x208` ≥ 31, vy > 0) | app | gate `% n`; if 0: `%60`, `%60`, `%40`, `%40` (+ bubbles on global) |
| Swim modes (not chasing) | app/global | as guppy: mode 123 exit `%9` (+ `grand() % 3`) |
| Every 21st tick | app | as guppy: `%10`, w.p. 1/10 `%9` |
| Eat | app | chomp `%3` (if off cooldown); gore `%3`, then per piece `%20` y, `%20` x, effect-shot ctor |
| Find prey, chase, bite check, diamond, hunger, death, remove | – | none |

Within a tick the carnivore's draws come where `Fish::Update` places them: the behaviour branch (eat draws, or startle-exit draws), then the reroll, then splash bubbles.

`static`.

## Open questions

- Everything is `static`. First check: buy a carnivore on 1-3 (13 draws on the purchase tick), then the first hunt and eat (hunger +900, gore draws), a diamond at 150..349 ticks, and the 1-3 death messages.
- The guppy coin interval reused as the diamond interval (150–349 ticks) seems fast. Confirm in a capture that no other code rewrites `+0x224` for carnivores (e.g. `0x004d71d0` outside Virtual Tank only returns its argument; checked statically).
- `alien-sylvester.md` §3.2 says the eat-sound draw picks `0x113` for r = 0. The code (`NEG/SBB` at `0x00538598`) gives `r % 3 == 0` → `0x114` (`CHOMP2`), else `0x113`, which is what `board.rs` implements. That doc line should be corrected.
- `fish-guppy.md`'s field table lists the widget x/y at `+0x2c`/`+0x30`. The code reads `+0x30`/`+0x34`. It also labels `Fish::UpdateHunger` as slot 83 (it is slot 82 by vtable index).
- `+0x55` = byte `0x005e8f14`: no writer found by an xref or byte-pattern search. Possibly written through a pointer (an option?). If it can be 1, clicks on carnivores would reach `Fish` slot 55, which still does nothing (`+0x229` = 0) but would stop the click reaching the board.
- `DeadFish` kind 5 revival: when its turn counter `+0x190` is brought into 0..10 by something (probably a reviving pet), it plays `0x125` and recreates an Oscar via `0x00544d80` (3-arg ctor `0x004efe00`, not analysed) at the corpse.
- Byte `+0x104` (bigger chomp sample, the `PRIMALSCREAM` within 100 px): which objects set it (copied by `0x004f21e0`, which is used when a fish is transformed).
- `+0x124 == 3` "last pizza" eat texts: which pet links set `+0x124`.
