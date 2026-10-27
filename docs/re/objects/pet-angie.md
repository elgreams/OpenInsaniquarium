# Angie the angelfish (`Sexy::FishTypePet`, pet id 18)

Angie swims like a guppy until a fish dies. While any corpse (`DeadFish`) is in the tank she stops swimming and homes in on the nearest **fresh** one (life > 95, i.e. at most about 30 ticks old); when her centre touches it she starts its revival, and **10 ticks later** the corpse turns back into a live fish of the same kind (guppy of the same stage, Oscar, Ultravore, beetlemuncher, starcatcher, Guppycruncher, breeder) at its spot, with the `HEAL` sound. The new fish is built from scratch (fresh hunger, RNG draws as any new fish). Fish eaten by aliens or carnivores leave no corpse and can't be revived. She has a pulsing white halo.

Same class, update and fields as Itchy (`pet-itchy.md`). Only the id-18 differences are listed here. All offsets are raw `this`-relative.

- Behaviour gate (slot 82) @ `0x004e9790`, id-0x12 test (first in the function)
- Chase (slot 84) @ `0x004e02d0`, id-0x12 branch; target (slot 85) @ `0x004e5fc0`, id-0x12 branch at `0x004e69c3`; attack (slot 86) @ `0x004fe740`, id-0x12 branch at `0x004fee60`
- Revival trigger `0x004d5630` (on the corpse); `DeadFish::Update` @ `0x004f64e0` (the revival itself)
- Re-creation: guppy `0x00546fc0` (→ 4-argument `Fish::Fish` @ `0x004eef90`), Oscar `0x00544d80`, Ultravore `0x00544fe0`, Gekko `0x005451c0`, Penta `0x00545360`, Grubber `0x005454f0`, Breeder `0x00547270`
- Animation (slot 89) @ `0x004dfc50`, id-0x12 branch; draw body `0x004f8610` case 0x12 → `0x004df590`

Confidence per section. `zoo_e_shrapnel_gash_angie` (`tank1_1.rs`) replays 1-2 with Shrapnel, Gash and Angie from the game's own demo (`zoo_e_01`, its first ~8,600 ticks: Shrapnel's bombs kill guppies and Angie revives them, 4 revivals) and compares the pets, guppies, corpses, sounds and the app RNG every tick. Drawing and the non-guppy revivals stay `static`.

## Fields used by id 18

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), plus:

| Offset | Type | Meaning |
|---|---|---|
| `+0x258` | double | halo phase, 0.0 at construction, ramps −1.0..1.0 by 0.01 per tick in the animation |

On the corpse (`DeadFish`, `fish-guppy.md#dead-fish`): `+0x190` turn/revival counter (100 at birth), `+0x1a0` life (125 at birth), `+0x1a4` kind.

Divisor and bounds are `Fish::Init`'s (guppy floor 370, max x 540).

`traced` (`zoo_e_shrapnel_gash_angie`).

## Construction

`Board::AddPet(18, -1, -1)` (`0x00544a90`), app MTRand: 2 position draws (`% 265 + 105`, `% 520 + 20`), `Fish::Init`'s 6 (facing, divisor, hunger, growth, mode, coin interval), no id-18 ctor branch, then a type-0 shadow (1 draw, `% 5`). **9 app draws**, as Itchy.

`traced` (`zoo_e_shrapnel_gash_angie`).

## Update (`0x004f7ad0`), id 18

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3, gate** (slot 82): if the dead-fish list `board+0x98` is **non-empty**, she runs the chase (below) and the gate returns true, so the swim modes are skipped. This test comes before every other gate test and doesn't look at aliens: with a corpse in the tank she ignores waves. A corpse she can't use (life ≤ 95) still holds her out of the swim modes; with no valid target she just coasts on her current velocity (the vx-bucket drift, clamps and edge steer still run). On tank 5 the gate isn't called.
- **Step 6:** slot 81 has no id-18 branch.
- **Animation:** own branch (below).

The re-roll draws (`r % 10`, then `r % 9 + 1` on 0, every 21 ticks) are unchanged.

`traced` (`zoo_e_shrapnel_gash_angie`).

## Target (`0x004e5fc0`, id 0x12)

Best = 10000. Walk `+0x98` in list order; a corpse is a candidate if its life `+0x1a0 > 95`. Distance as Itchy's: `dx = trunc((d.wx + 40) − (x + 40))`, `dy = trunc((d.wy + 40) − (y + 40))`, `dist = trunc(f32 sqrt(f32(dx² + dy²)))` (the +40 is used even for a 160-px Ultravore corpse); it wins if `dist < best`.

Corpse life drops by 1 every tick while life > 105, and below that only while the corpse is on the floor (y > 370; 300 for kind 6, 365 for kinds 8/9). So a corpse is a valid target for its first 29 ticks, and longer if it is still sinking.

`traced` (`zoo_e_shrapnel_gash_angie`).

## Chase (`0x004e02d0`, id 0x12)

1. Target (above). If `mode_timer (+0x1b0) < 5` or no target, skip the steering; otherwise `mode_timer = 0` and steer (every 5th tick).
2. Steering, with `c = 80` for a kind-6 corpse (Ultravore, 160 px) and 40 otherwise, `tx = d.wx + c`, `ty = d.wy + c`, `px = x + 40`, `py = y + 40` (doubles). A finer approach than Itchy's, first matching rule only:
   - x: `px > tx + 4` → `vx −= 1.0`; `px < tx − 4` → `vx += 1.0`; `px > tx + 2` → `vx −= 0.1`; `px < tx − 2` → `vx += 0.1`; `px > tx` → `vx −= 0.05`; `px < tx` → `vx += 0.05`. Each decrease only while `vx > −3`, each increase only while `vx < 3`.
   - y: `py > ty + 3` → `vy −= 0.6`; `py < ty − 3` → `vy += 1.0`; `py > ty` → `vy −= 0.3`; `py < ty` → `vy += 0.5`. Decreases only while `vy > −2`, increases only while `vy < 3`.
   - Speed bucket `+0x1bc += 1` while below 5.
3. If there is a target, **attack** (every tick).

`traced` (`zoo_e_shrapnel_gash_angie`).

## Attack (`0x004fe740`, id 0x12): starting a revival

Walk `+0x98` in list order. A corpse is touched when `|(x + 40) − (d.wx + c)| < r` and `|(y + 40) − (d.wy + c)| < r` (strict, doubles), with `c = 80, r = 60` for kind 6 and `c = 40, r = 30` otherwise, **and** its life > 95. On the first touched corpse:

- `0x004d5630`: if its counter `+0x190` is still **100** (untouched), set it to **10**. (If it is already counting down, nothing changes.)
- Stop.

No RNG, no sound here.

`traced` (`zoo_e_shrapnel_gash_angie`).

## The revival (`DeadFish::Update` `0x004f64e0`)

The corpse's update (`fish-guppy.md#dead-fish`) has a branch for a counter in 0..10:

1. **Cel:** while `0 ≤ counter ≤ 10`, `counter −= 1` and `cel = counter` (the death animation plays backwards, 9 down to 0) instead of the normal life-based cel.
2. The usual fade (life < 105) and the life < 1 removal come next, as for any corpse.
3. **If `counter < 1`:** sample **`0x125` (`HEAL`)**, gap 3; remove the corpse (its slot 82); then re-create the fish at the corpse's **widget** position `(wx, wy)` with its facing byte `+0x158` (1 = facing right):

| kind `+0x1a4` | fish | call |
|---|---|---|
| 0..4 (guppy stages) | guppy, stage = kind | `0x00546fc0(wx, wy, kind, facing)` |
| 5 | Oscar | `0x00544d80(wx, wy, facing)` |
| 6 | Ultravore | `0x00544fe0(wx, wy, facing)` |
| 7 | Gekko (beetlemuncher) | `0x005451c0(wx, wy, facing)` |
| 8 | Penta (starcatcher) | `0x00545360(wx)` (x only) |
| 9 | Grubber (Guppycruncher) | `0x005454f0(wx)` (x only) |
| 10..12 | Breeder, sub-kind = kind − 10 | `0x00547270(wx, wy, kind − 10, facing)` |

The corpse's update returns right after.

**Timing.** Corpses sit before the pets in the update order (`sim/board.md#tick-order`), so if Angie touches a corpse on tick t, its next updates take the counter 10 → 9 (t + 1) … → 0 (t + 10), and the fish comes back on **tick t + 10**. The new fish joins the end of the widget list and **updates in that same tick** (`sim/board.md`), with its own per-tick draws.

**The revived guppy** (`0x00546fc0`): `Fish::Fish(x, y, stage, facing)` (`0x004eef90`) → `Fish::Init(x, y)`: **6 app draws** (facing, divisor, hunger `% 200 + 400`, growth `% 3 + 4`, mode, coin interval), so it comes back with full fresh hunger and new timers; then stage `+0x1a0 = kind`, and **vx = +1.0 if facing right, else −1.0** (also the facing scale `+0x188`), overriding the draw. It is added to `+0xa0`, the "most guppies" record `board+0x48c` is raised if exceeded, and it gets a **type-0 shadow (1 draw)**. So a guppy revival costs **7 app draws** before its first update. The other kinds' ctors weren't read (their draw counts are open).

RNG in Angie's own code: none (only the re-roll).

`traced` (`zoo_e_shrapnel_gash_angie`).

## Animation (`0x004dfc50`), id 18

Turning is Itchy's (turn ±20, cel `9 − turn/2` / `turn/2 + 9`). When not turning:

- Swim cycle `+0x1b8 += 1` (speed bucket ≤ 1) or 2, reset to 0 above 19; cel = cycle / 2.
- Halo phase `+0x258 += 0.01`; once it is ≥ 1.0, it is set to −1.0.

Facing follows vx (guppy rule). No RNG.

`traced` (`zoo_e_shrapnel_gash_angie`).

## Drawing (`0x004df590`)

`IMAGE_ANGIE` (`0x005e8e8c`, `resources.xml`: 10 × 2 cels of 80 × 80), source `(cel · 80, row · 80)`:

| when | row | mirror |
|---|---|---|
| turning (`+0x1c4 ≠ 0`) | 1 | iff turn > 0 |
| otherwise | 0 | guppy rule |

Then, in additive mode with colourised images, `IMAGE_HALO` (`0x005e8c7c`, 10 × 2) with the **same source rect and mirror**, colour (255, 255, 255, `trunc(|+0x258| · 255)`): a halo that fades out and back in over 200 ticks (5.6 s). A type-0 shadow follows her.

`static`.

## Open questions

- Everything is static. Capture a starving guppy with Angie in the tank: the touch tick, the 10-tick countdown, the HEAL stamp and the 7 draws.
- Whether the corpse's removal (slot 82) deletes the shadow it inherited from the dead fish, or the shadow lingers (the new fish gets its own).
- Draw counts of the Oscar/Ultravore/Gekko/Penta/Grubber/Breeder ctors on revival.
