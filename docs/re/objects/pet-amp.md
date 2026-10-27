# Amp the electric eel (`Sexy::FishTypePet`, pet id 16)

Amp is a long (160 × 80) eel who drifts along like a guppy at double speed. A charge clock runs on calm ticks; when it is full (2700 calm ticks after he is added, 75.6 s) he glows and becomes **clickable**. The zap is **player-activated**: the first two clicks on the charged eel only play a crackle and change the glow's colour; the **third click** electrocutes the tank. Every live **guppy** dies (it leaves a normal corpse) and drops a **kind-5 coin worth $200** at its spot (capped at `max(20, 20 + (n − 20)/2)` coins for `n` guppies), with a spark per victim. If no fish is left afterwards, a free guppy drops in. Each zap makes the next charge 200 ticks longer. Carnivores and the other fish are not touched.

Same class, update and fields as Itchy (`pet-itchy.md`). Only the id-16 differences are listed here. All offsets are raw `this`-relative.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`, id-0x10 branch
- Update (slot 22) @ `0x004f7ad0`, id-0x10 cases (no drift, vy clamp)
- Abilities (slot 81) @ `0x004e5900`, id-0x10 branch (the charge)
- Mouse-down (slot 54) @ `0x004fbe20` → `0x004fb680` (the clicks and the zap)
- Animation (slot 89) @ `0x004dfc50`, id-0x10 branch; draw body `0x004f8610` case 0x10 → `0x004df2f0`
- Victims: guppy `Fish::Die` (slot 88 @ `0x004d5f90`, argument 0); pet removal `0x004d6830`; coin `Board::AddCoin` @ `0x00544430`; spark `EffectShot` @ `0x005436f0`; free guppy `Board::AddRandomGuppy` @ `0x00546d70`; "no fish left" test `0x005392a0`

Confidence per section. `amp_zap` (`tank1_1.rs`) replays 1-2 with Amp from the game's own demo (`amp_zap_01`: his charge pushed to 2900 with Cheat Engine, the EEL2 charge, two EEL3 crackles, then a zap of 7 guppies into corpses, $200 coins and sparks, and the free guppy) and compares the pets, guppies, corpses, coins, sounds and the app RNG every tick. Drawing and the id-21 pets stay `static`.

## Fields used by id 16

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), plus:

| Offset | Type | Meaning |
|---|---|---|
| `+0x180` | double | divisor **0.5** (ctor, replacing `Fish::Init`'s draw): he moves 2 × his velocity per tick |
| `+0x19c` | int | max x **460** (his widget is 160 wide); min x 10, floor 370, ceiling 95 stay `Fish::Init`'s |
| `+0x220` | int | charge clock: **300** at construction, +1 per calm tick, set to **−20** by a zap |
| `+0x224` | int | charge threshold: **3000**, **+200 per zap** (not in the Virtual Tank) |
| `+0x260` | int | clicks taken while charged (0..2), reset by the zap |
| `+0x258` | double | glow phase, 0.0 at construction and after a zap, ramps in the animation |
| `+0x55` | byte | widget "mouse visible": byte `0x005e8f14` (0) at construction; **1 while charged**; back to `0x005e8f14` on the zap |
| `+0x7c` | byte | widget hand-cursor flag (probably): 1 while charged, 0 on the zap |
| `+0x70`, `+0x78` | int | ctor writes 0 and 25 (Ghidra `Widget_data.field_0x1c/0x24`; probably mouse insets of the click area; not checked) |
| widget size | | **160 × 80** (`+0x88/+0x8c` in Ghidra's WidgetContainer: `0xa0`, `0x50`) |

`traced` (`amp_zap`).

## Construction

`Board::AddPet(16, -1, -1)` (`0x00544a90`), **app MTRand** in this order:

1. Position: `r % 265 + 105`, `r % 520 + 20` (2 draws).
2. `Fish::Fish` → `Fish::Init` (`0x004de370`): the guppy's 6 draws (facing, divisor, hunger, growth, mode, coin interval).
3. Ctor id-0x10 branch: the fields above. **No RNG.**
4. Shadow (`0x00544800`): FishTypePet id 16 gets a **type-2 shadow** (the alien's large one; `alien-sylvester.md`), id 20 a type-1 shadow, every other FishTypePet type 0. The Shadow ctor makes **1 draw** (`% 5`) whatever the type.

So adding Amp costs **9 app draws**, as Itchy.

`traced` (`amp_zap`).

## Update (`0x004f7ad0`), id 16

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the gate has no id-16 branch: **the swim modes always run**, aliens or not.
- **Step 6:** the charge (below), only on calm ticks (no alien, no Bilaterus; not on tank 5, not paused).
- **Step 8: no vx-bucket drift.** Instead, **vy is clamped to [−0.5, 0.5]** (`vy > 0.5 → 0.5`, else `vy < −0.5 → −0.5`).
- Clamp and edge steer use max x 460 (steer when x > 455).
- **Animation:** own branch (below).

The re-roll draws (`r % 10`, then `r % 9 + 1` on 0, every 21 ticks) are unchanged.

`traced` (`amp_zap`).

## Charge (slot 81, `0x004e5900`, id 16)

Each calm tick:

1. `clock (+0x220) += 1`. If `clock < threshold (+0x224)`, stop.
2. If `clock == threshold`: sample **`0x11a` (`EEL2`)**, gap 3 (the "charged" sound, once).
3. `+0x55 = 1`, `+0x7c = 1`: from now on clicks inside his widget reach his mouse-down instead of the board.

The clock keeps counting past the threshold (only the `==` tick plays the sound). It freezes during waves. **No RNG.**

First charge: `3000 − 300 = 2700` calm ticks after he is added. After a zap: `threshold + 200 + 20` calm ticks (3220, 3420, …), since the clock restarts at −20.

`traced` (`amp_zap`).

## Clicks (slot 54 `0x004fbe20` → `0x004fb680`)

Slot 54 calls `0x004fb680(x, y, clicks)`; if that returns false (and a re-entry guard byte `0x005e89f9` is clear) the click is passed on to the board's mouse-down at the same tank position. For id 16:

- **Right button** (`clicks < 0`): handled by the drop-in code shared by all FishTypePets (not Amp-specific).
- **Not charged** (`clock < threshold`): returns false → an ordinary tank click. (He isn't mouse-visible then anyway.)
- **Charged, `+0x260 < 2`:** `+0x260 += 1`, sample **`0x11b` (`EEL3`)**, gap 3. Returns true. No RNG.
- **Charged, third click:** the **zap** (below). Returns true.

The click handler doesn't check for aliens: a charged Amp can be fired during a wave (the clock only grows on calm ticks).

`traced` (`amp_zap`).

## The zap (`0x004fb680`, id 16)

In order, all in the click handler (input handling, before the tick's object updates):

1. `+0x55 = byte 0x005e8f14` (0), `+0x7c = 0`, his widget slot 38 is called with 0 (cursor/visibility reset; not followed), glow phase `+0x258 = 0.0`.
2. **Collect** the live guppies: walk `+0xa0` in list order and copy every guppy with `+0xac < 0` into a temporary list; `n` = how many.
3. **Coin cap** `cap = (n − 20) / 2 + 20` (C division, truncating toward 0), then at least 20. So up to 20 guppies all pay; above that only half of the extra ones do (n = 30 → 25).
4. For each collected guppy `g` in order, with `i` = 0, 1, …:
   1. `g`'s slot 88 (`Fish::Die`) with argument **0**: no per-fish `DIE` sound; `g` leaves its list and a normal **`DeadFish`** corpse is made from it (`fish-guppy.md#dead-fish`; Angie can revive it, `pet-angie.md`).
   2. If `i < cap`: **`AddCoin(trunc(g.x + 15), trunc(g.y + 10), kind 5, 0, −1.0, 0)`**: a kind-5 coin, value **200** (`coin.md#values-0x004d53a0-normal-game`). No RNG.
   3. **Draw `s = r % 3 + 3`** (app), then `EffectShot(trunc g.x, trunc g.y, s)` (`0x005436f0`): a spark of size 3..5. The shot ctor costs 1 more app draw (plus the heap-dependent one, `alien-sylvester.md#deviations`) when it is created, i.e. while `+0xd8` holds fewer than 20 shots.
   4. `i += 1`.
5. **Pets:** walk the FishTypePet list `+0xb4` and collect every pet with drop-in byte `+0x230 == 0`, **id 21** and `+0x240 > 0`. For each (continuing the same `i`, but with **no cap**): remove it (`0x004d6830(1)`: off the board list, deleted), `AddCoin(trunc(p.x + 15), trunc(p.y + 10), 5, 0, −1.0, 0)`, then `s = r % 3 + 3` and `EffectShot(trunc p.x, trunc p.y, s)`, `i += 1`. (What id 21's `+0x240` means is not followed here.)
6. If the tank now holds no fish at all (`0x005392a0`: no guppies `+0xa0`, carnivores `+0xa4`, Pentas `+0xd0`, nor the four lists behind `0x004e3ed0`/`0x004e3f00`/`0x004e3f30`/`0x004e3f60`) and this isn't the Virtual Tank: **`Board::AddRandomGuppy`** (`fish-guppy.md#spawn-boardaddrandomguppy-0x00546d70`: 11 app draws, a guppy splashing in from the surface). This keeps a zap from ending the game.
7. Sample **`0x119` (`EEL1`)**, gap 3.
8. If `i > 0`: the die sound `0x00538610(−1)`: `DIE` (`0x117`), gap 3, no pitch shift. No RNG.
9. Unless in the Virtual Tank: `threshold += 200`. Then `clock = −20`, `+0x260 = 0`.

**RNG per zap** (app MTRand, in this order): per victim 1 spark draw + the spark ctor's draws; then 11 for a free guppy if the tank is empty. Every coin and corpse is RNG-free. The coins and corpses are created during input handling, so they get their first update in the same tick (`amp_zap_01`: corpse life 124 on the zap tick).

`traced` (`amp_zap`).

## Animation (`0x004dfc50`), id 16

Turning is Itchy's (turn ±20, cel `9 − turn/2` / `turn/2 + 9`). When not turning:

- Swim cycle `+0x1b8 += 1` every tick (the speed bucket doesn't matter), reset to 0 above 19; cel = cycle / 2.
- Glow phase `+0x258 += 0.5` while not charged, **`+= 0.1`** while charged (`clock ≥ threshold`); once it is ≥ 1.0 it is set to −1.0. So the phase cycles every 4 ticks uncharged, every 20 ticks charged.

Facing follows vx (guppy rule). No RNG.

`traced` (`amp_zap`).

## Drawing (`0x004df2f0`)

`IMAGE_AMP` (`0x005e8de8`, `resources.xml`: 10 × 2 cels), cels of **160 × 60**, source `(cel · 160, row · 60)`:

| when | row | mirror |
|---|---|---|
| turning (`+0x1c4 ≠ 0`) | 1 | iff turn > 0 |
| otherwise | 0 | guppy rule |

Then a glow, `IMAGE_AMPCHARGE` (`0x005e8ce8`, 10 × 2) with the same source rect and mirror, additive and colourised, alpha **`trunc(min(255, 510 · |phase|))`** (the x87 sequence at `0x004df380`: `FMUL ST(1),ST(0)` then keep the smaller of 255 and the product).

| state | colour (r, g, b) |
|---|---|
| charged, 0 clicks | (255, 255, 200) |
| charged, 1 click | (255, 200, 100) |
| charged, 2 clicks | (255, 100, 100) |
| just zapped (`clock < 0`, the 20 ticks after a zap) and no alien | (255, 255, 200) |
| otherwise | no glow |

His shadow is type 2.

`static`.

## Open questions

- Everything is static. Capture a charge and a three-click zap with a dozen guppies: the `EEL2` stamp at the charge tick, the corpses, the kind-5 coins, the spark draws.
- Coin kind 5: `coin.md` lists 5 as "pearl"; the value table puts it with the diamond (200). Its fall/draw (probably the diamond art) isn't checked.
- Id 21's `+0x240` (the pets the zap also kills) and the widget slot 38 call weren't followed.
- The mouse insets (`+0x70 = 0`, `+0x78 = 25`) are a guess from SexyApp's `Widget` layout.
