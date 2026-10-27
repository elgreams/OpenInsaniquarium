# Rhubarb the hermit crab (`Sexy::OtherTypePet`, pet id 14)

Rhubarb walks along the tank floor like Rufus, but he never touches aliens. Whenever a fish swims down near the floor close to him, he snaps his claws, and every fish still inside his claw box at the moment of the snap is flung upward (vy −20 or −30, one app draw per fish). In-game text: "RHUBARB snaps his claws at ..." (`0x005a2fe0`). He is the pet hatched after tank 3-5 (`screens/hatch-screen.md#pet-award-table-adventure-first-play-through`).

Same class, fields and update skeleton as Stinky (`pet-stinky.md`), Clyde (`pet-clyde.md`) and Rufus (`pet-rufus.md`, the other crab). Only the id-14 differences from Rufus are listed. All offsets are raw `this`-relative.

- Ctor `OtherTypePet::OtherTypePet` @ `0x004eb5c0`
- Update (slot 22) @ `0x004f2e80`, id-14 branch (`0x004f31be`..`0x004f3284`)
- Gate `0x004efe80` (id 14) → chase `0x004ebc80` → target `0x004e7270` (id-14 branch at `0x004e74c2`), snap `0x004e7c30` (id-14 branch)
- Animation `0x004eb7c0` (id-14 path)
- Draw (slot 26) `0x004f37c0` → `0x004f3590` case 14

Confidence per section. `zoo_d_blip_rhubarb_nimbus` (`tank1_1.rs`) replays 1-2 with Blip, Rhubarb and Nimbus from the game's own demo (`zoo_d_02`, about 5,900 ticks, two waves: 53 snaps flinging guppies) and compares the pets, guppies, coins, food, sounds and the app RNG every tick. Drawing, the Virtual Tank and fish lists not on tank 1 stay `static`.

## Fields used by id 14

As Rufus's table (`pet-rufus.md#fields-used-by-id-7`), with these differences:

| Offset | Meaning for Rhubarb | Initial |
|---|---|---|
| `+0x160` | y: **forced to 355.0** (`0x00596ec8`) unless the drop-in flag is set | 355 |
| `+0x188` | speed divisor: the default | **2.0** |
| `+0x190` | wander mode, 0..8 after the first reroll (the ctor's `% 10` can give 9, which keeps the target unchanged) | `Next() % 10` |
| `+0x1a0` | walk animation counter (0..39) | 0 |
| `+0x1b4` | **snap counter**: 20 when a snap starts, −1 per tick to 0 | 0 |

`traced` (`zoo_d_blip_rhubarb_nimbus`).

## Construction

`Board::AddPet(14, -1, -1, 0, 0)` (`0x00544a90`; ids 0, 1, 5, 7, 14 build an `OtherTypePet`) → ctor → shadow. App MTRand draws, in order:

1. AddPet: `x = Next() % 265 + 105`.
2. AddPet: `y = Next() % 520 + 20` (discarded: the ctor forces y = 355).
3. Ctor: `mode = Next() % 10`.
4. Ctor: `+0x1b8 = Next() % 250 + 250` (unused).
5. Shadow ctor: `y = 410 + Next() % 5`. Shadow **type 1** (`0x00544800`: every OtherTypePet except ids 1 and 5), as Rufus.

**5 app draws.** Divisor 2.0 (`0x005c3cc8`). No other id-14 ctor code.

`traced` (`zoo_d_blip_rhubarb_nimbus`).

## Update (`0x004f2e80`), id 14

Same skeleton as Rufus's update (`pet-rufus.md#update-0x004f2e80-id-7`). Differences:

1. Return if no board / paused; `PreUpdate` (`0x004f22c0`). Drop-in: `vy += 0.1` while y < 380 (ids 0, 1, 7, 14).
2. **Tank 5 (Virtual Tank, `board+0x3c8 == 5`): the whole step is skipped**: no gate, no target vx, **no vx drift**. So in the Virtual Tank Rhubarb's vx never changes and he stands where he was placed (open question below).
3. Otherwise:
   - Call the gate `0x004efe80`. For id 14 it **always** runs the chase `0x004ebc80` ([Snap](#snap)), regardless of aliens; its return value is ignored by the update.
   - **Then, every tick (whether or not a fish was found), set target vx `+0x180` from the mode:**

     | mode | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | ≥ 9 |
     |---|---|---|---|---|---|---|---|---|---|---|
     | target vx | 0 | −0.5 | +0.5 | −1.0 | +1.0 | **+1.5** | **−1.5** | −2.5 | +2.5 | unchanged |

     (Rufus has 0 for modes 5 and 6.) Constants: `0x00596760` −0.5, `0x005c3c20` 0.5, `0x005c3d80` −1.0, 1.0 inline, `0x005c3d58` 1.5, `0x00596778` −1.5, `0x005967c0` −2.5, `0x005966a0` 2.5.
   - Drift: if target < vx, `vx −= 0.1`; then if vx < target, `vx += 0.1` (no snap), as Rufus.

   Unlike Rufus, the wander target is refreshed even while chasing, so the chase's ±1.8 kicks are pulled back toward the wander speed at 0.1/tick.
4. Counters and reroll: exactly as Rufus (`+0x198 > 20`, or `x ≤ 10` with target ≤ 0, or `x ≥ 540` with target ≥ 0 → `+0x198 = 0`, app `Next() % 10`; if 0, app `mode = Next() % 9`). The reroll runs on tank 5 too. The shell routine `0x004e2a20` does nothing for id 14.
5. **Clamp:** x ≤ **540** (`0x00599048`, the bound for every OtherTypePet other than 0, 5, 7), x ≥ 10. y > **355** → y = 355 and vy = 0. y < 95 → 95.
6. Edge modes, as Stinky/Rufus: x > 535 and vx > 0.1 → mode 1; x < 15 and vx < −0.1 → mode 2.
7. **Animation** `0x004eb7c0`, id 14 ([Animation](#animation)); this is also where the snap counter counts down.
8. Move: `x += vx / 2`, `y += vy / 2` (÷ 10 with the slow flag `+0x1b0`, never set here).
9. Widget position = `trunc(x), trunc(y)`.

RNG per tick, in order: the snap's launch draws in step 3 (one per flung fish, see below), then the reroll in step 4 (`% 10`, then `% 9` w.p. 1/10). All app MTRand.

`traced` (`zoo_d_blip_rhubarb_nimbus`).

## Snap

The chase `0x004ebc80` (id 14), every non-tank-5 tick:

1. **Target** (`0x004e7270`, id 14). Threshold `T` = **250** if any alien (`board+0xb8`) or Bilaterus (`board+0xf4`) exists, else **260** (`0x004da780`). Best = 100 000 000, strict `<` (first minimum wins). Scan, in this order, each list in list order:
   guppies `board+0xa0`, carnivores `board+0xa4`, Gekkos `board+0xcc`, breeders `board+0xc0`, Ultravores `board+0xd4`.
   For each fish: `dx = trunc((f.wx + 40) − (x + 40))`, `dy = trunc((f.wy + 40) − (y + 40))` (Ultravore: `+80` instead of `+40` on the fish side), `d = dx² + dy²`; a fish qualifies only if its widget y `f.wy > T` (Ultravore: `f.wy > T − 40`). Aliens are not considered at all.
2. **Steering**, only when `+0x194 > 4` and a target exists; then `+0x194 = 0`. With `cx = x + 40` and `tx = target.wx + 40` (+40 for every kind, including the 160-px Ultravore):
   - `cx > tx` and `vx > −5`: `vx −= 1.8`;
   - else `cx < tx` and `vx < 5`: `vx += 1.8`.

   If there is no target the counter is not reset and keeps counting.
3. **Snap / fling** (`0x004e7c30`, id 14), every tick a target exists. Threshold `S` = **260** with an alien or Bilaterus present, else **270**. With `cx = x + 40` (double), walk the same five lists in the same order; for each fish, two checks in this order:
   - **Fling**: if `f.wx < cx < f.wx + 80` (Ultravore `+160`) and `f.wy > S` (Ultravore `S − 40`) and the snap counter `+0x1b4 == 5`: set the fish's vy-damp timer to **50** (`+0x208`; for breeders `+0x1d8`), then **one app draw** `r`: vy `+0x170` = **−20.0** if `r` is even (`0x00596810`), **−30.0** if odd (`0x00596768`).
   - **Start**: if `f.wx − 10 < cx < f.wx + 90` (Ultravore `+170`) and `f.wy > S` (Ultravore `S − 40`) and `+0x1b4 == 0`: `+0x1b4 = 20`.

   Bounds are strict, fish x/y are the integer widget position. Nothing else is changed: no damage, no sound here, Rhubarb's own vx is not touched.

**Timeline of one snap.** Tick t: a fish low enough is in the wide box → counter 20, and the animation in the same tick takes it to 19. The fling check sees the counter after the previous tick's animation, so it sees 5 on tick **t + 15**: every fish then inside the narrow box and below `S` is flung (not only the one that started the snap, and not only the target). The click sound plays on tick t + 9 (counter 10). The counter is 0 again after tick t + 19, so a new snap can start on tick t + 20.

Note the two thresholds differ: the target needs `wy > 260` (250), the snap `wy > 270` (260). A fish with `260 < wy ≤ 270` is chased but never snapped at.

`traced` (`zoo_d_blip_rhubarb_nimbus`).

## Effects on other objects

Applied in `0x004e7c30` (id-14 branch, from `0x004e7e5a`):

- **Guppies** (`+0xa0`), **carnivores** (`+0xa4`), **Gekkos** (`+0xcc`), **Ultravores** (`+0xd4`): vy `+0x170` = −20 or −30 and vy-damp timer `+0x208` = 50. With the guppy's damping (`fish-guppy.md`: `vy *= 0.9` per tick while the timer is positive) a −30 kick decays like a splash-in in reverse.
- **Breeders** (`+0xc0`): vy `+0x170` = −20 / −30, and **`+0x1d8`** = 50 (the breeder's own layout; assumed to be its damp timer, unconfirmed).
- **Aliens, Bilaterus, coins, food:** none. Rhubarb does no damage.

`traced` (`zoo_d_blip_rhubarb_nimbus`).

## Animation

`0x004eb7c0`, id 14. No RNG. Walk counter `a` = `+0x1a0`, cel = `+0x1a4`.

1. **Snapping** (`+0x1b4 > 0`): `+0x1b4 −= 1`; cel = new value / 2 (9 → 0). When the new value is 10, play `SOUND_BUTTONCLICK` (global `0x005e8c40`, set next to the `"SOUND_BUTTONCLICK"` lookup in `0x00506ea0`) through the app's sound slot (app vtable `+0xd8`), the same call the shop buttons use. No throttle in this call. `a` is left alone.
2. Otherwise walk (no `% 40` pre-wrap, unlike Rufus):
   - vx ≥ 1 → `a −= 2`;
   - vx ≤ −1 → `a = (a + 2) % 40`;
   - 0 < vx < 1 → `a −= 1`;
   - −1 < vx ≤ 0 → `a = (a + 1) % 40`;
   - negative `a` gets `+40`; cel = a / 4.

   This is Rufus's walk with the direction reversed (his cycle runs forward when he walks left). There is no idle animation: standing still (vx = 0) still advances the cycle by 1.
3. Missile overlay counter (`+0x98`) as every pet.

`traced` (`zoo_d_blip_rhubarb_nimbus`).

## Clicks

As Clyde (`pet-clyde.md#clicks`): a left click goes through to the board; a right click is handled locally (`0x004f3a00`).

`static`.

## Drawing

Draw (slot 26 @ `0x004f37c0`) calls `0x004f3590(g, mirror = 0)` for id 14: **never mirrored**. Case 14 draws `IMAGE_RHUBARB` (global `0x005e8a98`, name at `0x0059c640`; `resources.xml`: 10 × 2 cels, framedelay 4) at (0, 0), source `(cel · 80, row · 80, 80, 80)`:

- row 0 while `+0x1b4 < 1` (walking);
- row 1 while snapping (`+0x1b4 ≥ 1`).

Then the shared tail (drop-in flash, missile overlay, drop-in timer). Shadow: type 1.

`static`.

## Summary / open questions

- **5 app draws** on AddPet (x, discarded y, mode, `+0x1b8`, shadow). Per tick: one parity draw per flung fish (during the update's step 3, list order guppy → carnivore → Gekko → breeder → Ultravore), then the wander reroll.
- Effects: vy −20/−30 and damp timer 50 on fish low in the tank (widget y > 270, or > 260 during an attack) that are in his claw box 15 ticks after a snap starts. No damage to anything.
- Open: whether the Virtual Tank really leaves him motionless (step 2 skips the drift on tank 5; check against a VT run); breeder `+0x1d8` meaning; whether a flung guppy shows the splash bubbles (`Fish::Update`'s bubble branch needs vy > 0, so presumably not); the sound id behind `0x005e8c40` (assumed `SOUND_BUTTONCLICK` from its single writer) and whether the app's `+0xd8` slot throttles repeats.
- First capture should be tank 3-5 or later with Rhubarb and guppies near the floor (an empty food supply makes guppies sink).
