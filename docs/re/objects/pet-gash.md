# Gash the shark (`Sexy::FishTypePet`, pet id 17)

Gash swims like a guppy while the tank is calm. When an alien or Bilaterus is in, he charges the nearest one and bites it for **3 HP per tick** while his centre is within 20 px of its centre; unlike Itchy, his bite **kills the alien on the spot** once its health reaches 0. He also has a hunger clock that only runs on calm ticks: once it passes 1570, he hunts the **nearest guppy** (any stage) and swallows it whole (no corpse, chomp sound), and the clock restarts. The first snack comes 3121 calm ticks (87 s) after he is added, each later one 1571 calm ticks (44 s) after the previous.

Same class, update and fields as Itchy (`pet-itchy.md`). Only the id-17 differences are listed here. All offsets are raw `this`-relative.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`, id-0x11 branch (last in the ctor)
- Behaviour gate (slot 82) @ `0x004e9790`, id-0x11 test
- Abilities (slot 81) @ `0x004e5900`, id-0x11 branch (the clock)
- Chase (slot 84) @ `0x004e02d0`, id-0x11 branch; target (slot 85) @ `0x004e5fc0`, id-0x11 branch at `0x004e63a2`; attack (slot 86) @ `0x004fe740`, id-0x11 branch at `0x004fe9a4`..`0x004fee5b`
- Alien damage `0x004fb5b0` (id-0x11 half); alien death `0x004f9c70`; Bilaterus death `0x004fb4c0`
- Guppy removal: Fish slot 87 @ `0x004df0b0` (vtable `0x00597d54` + `0x15c`); chomp sound `0x00538560`
- Animation (slot 89) @ `0x004dfc50`, id-0x11 branch; draw body `0x004f8610` case 0x11 → `0x004df810`

Confidence per section. `zoo_e_shrapnel_gash_angie` (`tank1_1.rs`) replays 1-2 with Gash from the game's own demo (`zoo_e_01`, ~8,600 ticks, 3 waves he fought): construction, the calm clock, the hunt and the alien bite are `traced`. **The guppy snack is `static`**: it happened only in a later capture chunk that can't be replayed (`dynamic-analysis.md`). Drawing and Bilaterus stay `static`.

## Fields used by id 17

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), plus:

| Offset | Type | Meaning |
|---|---|---|
| `+0x220` | int | hunger clock: **−1550** at construction (0 in the Virtual Tank, `app+0x87c == 5`; **1520** with the drop-in flag `+0x230`, in any mode); +1 per calm tick; reset to 0 when he eats a guppy |
| `+0x224` | int | hunger threshold: **1570**, fixed |
| `+0x1c8` | int | bite timer (the Fish eat timer): set to 10 when a prey is close, counts down in the animation; drives the mouth row |

Divisor `+0x180` stays the `Fish::Init` draw (2.0, 1.8 or 1.6); bounds are a guppy's (floor 370, max x 540).

`traced` (`zoo_e_shrapnel_gash_angie`).

## Construction

`Board::AddPet(17, -1, -1)` (`0x00544a90`), **app MTRand** (`app+0x7b0`, `MTRand__NextNoAssert`) in this order:

1. Position: `r % 265 + 105`, `r % 520 + 20` (2 draws).
2. `Fish::Fish` → `Fish::Init` (`0x004de370`): the guppy's 6 draws (facing, divisor, hunger, growth, mode, coin interval).
3. FishTypePet ctor, id-0x11 branch: `+0x224 = 1570`, `+0x220` as in the table above. **No RNG.**
4. Shadow (`0x00544800`): type 0 (every FishTypePet except ids 16 and 20), 1 draw (`% 5`).

So adding Gash costs **9 app draws**, as Itchy.

`traced` (`zoo_e_shrapnel_gash_angie`).

## Update (`0x004f7ad0`), id 17

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`); `0x004f7ad0` itself has no id-0x11 case. The differences are in the slots it calls:

- **Step 3, gate** (slot 82, `0x004e9790`): Gash **hunts** (calls the chase, returns true, so the swim modes are skipped) when
  - an alien (`board+0xb8`) or Bilaterus (`+0xf4`) exists (`0x004da780`), **or**
  - `clock (+0x220) > threshold (+0x224)` (strict) **and** the guppy list `+0xa0` is non-empty (`0x004da7d0`).

  Otherwise it returns false and the swim modes run. On tank 5 (`board+0x3c8 == 5`) the gate isn't called at all, so he never hunts there.
- **Step 6, abilities** (slot 81, only on ticks with no alien and no Bilaterus; returns at once on tank 5 or while paused): `clock += 1`. Nothing else. So the clock **freezes during a wave** and keeps running while he hunts a guppy.
- **Animation:** own branch (below).

The re-roll draws (`r % 10`, then `r % 9 + 1` on 0, every 21 ticks) are unchanged and happen whether he hunts or not.

`traced` (`zoo_e_shrapnel_gash_angie`).

## Target (`0x004e5fc0`, id 0x11)

Best = 10000; distances are computed as for Itchy: `dx = trunc((cx_t) − (x + 40))`, `dy = trunc((cy_t) − (y + 40))`, `d = trunc(f32 sqrt(f32(dx² + dy²)))`, and a candidate wins only if `d < best` (first in list order wins ties).

- **Alien or Bilaterus present:** walk the Bilaterus list `+0xf4` (centre `wx + 40, wy + 40`, every entry a candidate), then the alien list `+0xb8` (centre `wx + 80, wy + 80`; skipped if its byte `+0x204` is set).
- **Otherwise**, only if `clock > threshold` and `+0xa0` is non-empty: walk the guppies `+0xa0` in list order; a guppy is a candidate if its `+0xac < 0` (normal live state), centre `wx + 40, wy + 40`. Any stage qualifies (small to king), and Wadsworth's shelter (`pet-wadsworth.md`) is **not** checked: a sheltered guppy can be eaten (only between waves, when the shelter is off anyway).
- Otherwise no target.

`traced` (`zoo_e_shrapnel_gash_angie`).

## Chase (`0x004e02d0`, id 0x11)

1. Get the target (above).
2. If `mode_timer (+0x1b0) < 5` or there is no target, skip the steering. Otherwise `mode_timer = 0` and steer (so every 5th tick, as Itchy), with `tx, ty` = the target centre used above (for an alien: `+80`, or `+40` if the target's type `+0x8c` is `0x17`; for a guppy: `+40`):
   - **x, with an alien/Bilaterus present:** `x + 40 > tx` → `vx −= 2.5` if `vx > −9`; `x + 40 < tx` → `vx += 2.5` if `vx < 9`.
   - **x, hunting a guppy:** the same with the limits **±8**.
   - **y (both cases):** `y + 40 > ty` → `vy −= 1.5` if `vy > −4`; `y + 40 < ty` → `vy += 1.5` if `vy < 4`.
   - Speed bucket `+0x1bc += 1` while below 5.
3. If there is a target, **attack** (every tick, steering or not). With no target the chase returns false, but the gate has already returned true: the swim modes are still skipped that tick (he coasts on his current velocity).

So his top horizontal speed is 9/div against aliens (Itchy: 10/div) and 8/div against guppies.

`traced` (`zoo_e_shrapnel_gash_angie`).

## Attack (`0x004fe740`, id 0x11)

"Near" below means `|(x + 40) − cx| < 30` and `|(y + 40) − cy| < 30`; "contact" means the same with **20** (doubles, strict; `cx, cy` the target centres above).

`traced` (`zoo_e_shrapnel_gash_angie`).

### With an alien or Bilaterus present

1. **Bilaterus** (`+0xf4`, list order). For each segment:
   - contact → **health −= 3.0** on the segment's owner (`seg+0x158`, double at `+0x1d8`), sample `0x130` (`PUNCH`) with gap 9 (`0x00538340(9)`). If the health is then ≤ 0, run the Bilaterus death `0x004fb4c0` on `seg+0x158` (an `0x00544130` effect, `EXPLODE` `0x11d` + `0x11e`; not specified further here) and go to step 2. Otherwise **continue with the next segment**: several segments can be hit in one tick.
   - else near and bite timer `+0x1c8 == 0` → `+0x1c8 = 10`, go to step 2.
2. **Aliens** (`+0xb8`, list order). For each:
   - contact and `+0x204 == 0` → **damage** (below), **stop** (one alien per tick);
   - else near and `+0x1c8 == 0` → `+0x1c8 = 10`, **stop**;
   - else next.

**Damage** (`0x004fb5b0`, id-0x11 half): alien health `+0x1f0 −= 3.0` (**0.5** for kinds 4 and 5); **if health ≤ 0, the alien dies at once** through `0x004f9c70(1)` (`alien-sylvester.md#43-death-0x004f9c701-kind-1`: diamond coin, debris on the app stream, `0x11d`/`0x11e`, DeadAlien); then sample `0x130` with **gap 9**. (Itchy: −1.0 / −0.25, gap 11, and the alien dies in its own next update.) Since aliens update before the `+0xb4` pets, the dying alien has already had its update this tick.

So a 130-HP kind-3 alien that he keeps in contact dies in 44 ticks (1.2 s).

`traced` (`zoo_e_shrapnel_gash_angie`).

### Without aliens (guppy snack)

Only if `clock > threshold` and `+0xa0` is non-empty. Walk the guppies `+0xa0` in list order, skipping any with `+0xac ≥ 0`:

- **contact** → eat it:
  1. the guppy's slot 87 with `true` (`0x004df0b0`): it is removed from the widget manager, the app registry and `+0xa0` (`0x00541d90`), its shadow is deleted, and `board+0x488` (fish lost) `+= 1`. **No `DeadFish`**, no `DIE` sound: the same removal as an alien eating a guppy (`carnivore.md`, "Removing the guppy").
  2. chomp sound `0x00538560(0)`: if sample `0x113` is off its 3-tick cooldown, **1 app draw `r % 3`** (0 → `CHOMP2` `0x114`, else `CHOMP` `0x113`); no draw while throttled.
  3. `clock = 0`. Stop.
- else near and `+0x1c8 == 0` → `+0x1c8 = 10`, stop.
- else next.

The guppy eaten is the first one **in list order** that he touches, not necessarily the target he was steering toward. Because guppies update before the `+0xb4` pets, the eaten guppy has had its update this tick. Nothing is paid or refunded.

RNG in the attack: the chomp draw only (plus whatever an alien death draws).

`static` (the snack in `zoo_e_01` is in a chunk that can't be replayed).

## Animation (`0x004dfc50`), id 17

Turning is Itchy's (turn ±20 on a vx sign change, cel `9 − turn/2` / `turn/2 + 9`, the bite timer `+0x1c8` cleared when the turn reaches 0). When not turning:

- **bite timer `+0x1c8 > 0`:** decrement it; cel = the new value (9 down to 0). The swim cycle doesn't advance.
- otherwise the default: swim cycle `+0x1b8 += 1` (speed bucket ≤ 1) or 2, reset to 0 above 19; cel = cycle / 2.

Facing follows vx (guppy rule). No RNG.

`traced` (`zoo_e_shrapnel_gash_angie`).

## Drawing (`0x004df810`)

`IMAGE_GASH` (`0x005e8bfc`, `resources.xml`: 10 × 3 cels of 80 × 80), source `(cel · 80, row · 80)`:

| when (first match) | row | mirror |
|---|---|---|
| turning (`+0x1c4 ≠ 0`) | 1 | iff turn > 0 |
| bite timer `+0x1c8 > 0` | 2 (the bite) | guppy rule |
| otherwise | 0 | guppy rule |

There are no separate alien rows. A type-0 shadow follows him.

`static`.

## Open questions

- The guppy snack still needs a replayable capture (≥ 3121 calm ticks in one capture chunk, or a chunk with its own RNG header).
- The Bilaterus side (`+0xf4` segment → `+0x158` owner, `0x004fb4c0`) isn't specified anywhere yet; only its cost to the RNG matters for a replay and that is not read here.
- Whether eating a guppy can end the level (empty tank) is the board's normal empty-tank check, not Gash's code.
