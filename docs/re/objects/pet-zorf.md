# Zorf the sea horse (`Sexy::FishTypePet`, pet id 4)

Zorf swims slowly in the upper part of the tank. While the tank is calm, he spits a food pellet whenever a guppy or breeder is getting hungry, at most once every 75 ticks (2.1 s). The pellet flies out sideways in the direction he swims, then sinks. It is always level-1 food, needs no money, and doesn't count against the player's pellet limit. He is the pet hatched after tank 1-5 and first appears on 2-1.

Same class, update and fields as Itchy (`pet-itchy.md`); only the id-4 differences are listed here. Raw `this`-relative offsets.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`, id-4 branch
- Abilities (slot 81) @ `0x004e5900`, id-4 branch `0x004e5b8e`
- Animation (slot 89) @ `0x004dfc50`; draw body `0x004f8610` → `0x004df980`
- Food made through `Board::AddFood` @ `0x00543280` → `Food::Food` @ `0x004ef840`

Confidence per section. `zoo_b_zorf_meryl_wadsworth` (`tank1_1.rs`) replays 1-2 with Zorf, Meryl and Wadsworth from the game's own demo (`zoo_b_01`, 10,400 ticks, 51 pellets) and compares the pets, guppies, food, coins, sounds and the app RNG every tick. Sections marked `static` (drawing, the Virtual Tank, Brinkley, Challenge) aren't covered.

## Fields used by id 4

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), plus:

| Offset | Type | Meaning |
|---|---|---|
| `+0x220` | int | feed counter (the guppy coin timer). +1 per calm tick; −10 right after a pellet |
| `+0x224` | int | feed interval, **65** |
| `+0x240` | int | "check count", only counted while Brinkley (pet 20) is in the tank |
| `+0x180` | f64 | divisor, **3.0** (a guppy's is 1.6–2.0) |
| `+0x190` | int | floor, **270** (a guppy's is 370) |

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Construction

`Board::AddPet(4, -1, -1)` (`0x00544a90`): the same **9 app draws** as Itchy (2 position, 6 `Fish::Init`, 1 shadow). The id-4 ctor branch then sets, with no RNG:

- feed interval `+0x224` = **65**;
- divisor `+0x180` = **3.0**, replacing the `[2.0, 1.8, 1.6]` value `Fish::Init` drew (the draw still happens);
- floor `+0x190` = **270**.

The feed counter `+0x220` starts at 0. The Virtual Tank doesn't change any of this for id 4.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Update (`0x004f7ad0`), id 4

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the behaviour gate (slot 82, `0x004e9790`) has no id-4 branch and returns false, so **the swim modes always run**, aliens or not (as Prego).
- **Step 6:** the feed ability (below). Like every slot-81 ability, it is only called when there are no aliens (`+0xb8`) and no Bilaterus (`+0xf4`), and it returns at once on tank 5 (`board+0x3c8 == 5`) or while the board is paused. So the counter **freezes during a wave**.
- **Steps 8 and 13:** every velocity is divided by 3.0, so Zorf drifts and swims at about half a guppy's speed.
- **Step 10, floor:** y is clamped to 270 (not 370). The one exception is a drop-in Zorf (byte `+0x230` set): below the floor he isn't clamped, his swim mode is set to 2 (swim up) instead. Never in Adventure, where `+0x230` is 0.
- **Animation:** see below.

The re-roll draws (`r % 10`, then `r % 9 + 1` on 0, every 21 ticks) are the same as Itchy's. Zorf updates in the `+0xb4` slot, after the lower pet ids (Itchy, Prego).

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Feeding (slot 81, `0x004e5900`, id 4)

Each call (calm ticks only, see above):

1. `counter (+0x220) += 1`. If `counter < 65`, stop.
2. **Hungry check.** Walk the guppy list (`board+0xa0`) in order, then the breeder list (`board+0xc0`) in order. `hungry` = some fish there has **hunger `+0x9c` < 300** and its `+0xf0` == 0. (`+0xf0` is a `GameObject` field that is 0 for every guppy and breeder we know of; open question.) The walk stops at the first match. Carnivores, ultravores and other fish aren't checked. No RNG.
3. **Brinkley gate.** `board+0x194` is the board's count of pet id 20 (Brinkley; the per-id pet counters are `board+0x144 + 4·id`, `board.md#object-lists`):
   - **No Brinkley** (the normal case): if `hungry` is false, stop. **The counter is not reset**, so from the 65th tick on, the check runs again on every calm tick until some fish is hungry, and the pellet comes on that tick.
   - **Brinkley present:** `+0x240 += 1`. With `m = 2` (4 in the Virtual Tank, `app+0x87c == 5`): if `+0x240 % m ≠ 0` and `hungry` is false, set `counter = 0` and stop. Otherwise drop. So with Brinkley, every m-th check drops a pellet even when no fish is hungry.
4. **Drop:** `counter = −10`, then `AddFood(wx + 15, wy + 10, motion, flag 0, delay 0, level −1)` (`0x00543280`), where `wx, wy` = Zorf's widget position and **motion = 1 if vx < 0, else 2** (vx = 0 counts as 2).

Timing: the counter restarts at −10, so the next check is **75 ticks** after a drop. Without Brinkley that is the minimum spacing; with the tank staying hungry, Zorf drops one pellet every 75 ticks (2.1 s). With Brinkley and no hungry fish: drop, 75 ticks, reset, 65 ticks, drop: one pellet per 140 ticks.

**RNG:** the drop costs **one app draw**, inside Zorf's update: the food ctor's `(r & 1) + 3` animation speed (`food.md#dropping-food-player-click`). Nothing else in this ability draws.

**Sound:** none. `AddFood` plays `SOUND_DROPFOOD` only for motion-0 food.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Zorf's pellet

What `AddFood` (`0x00543280`) and `Food::Food` (`0x004ef840`) make of Zorf's arguments:

- **No count limit, no refund.** The `max_food` check only runs for motion-0 food. And because the limit counts `extra` = pellets with motion ≠ 0, Zorf's pellets **never block the player's own drops**.
- **Level forced to 1**, whatever the bought food quality (any motion ≠ 0 does this; the level argument is −1, so nothing overrides it). Eating it is the level-1 case of `food.md#eat-slot-77-0x004f1ba0-basic-pellet-outside-virtual-tank` (+700 hunger, cap 1000, growth +2), even on a level where the player's own pellets are level 0. (Level 2 food bought in the shop doesn't apply to Zorf's either.)
- Kind `+0x194` = 0 (an ordinary pellet), edible delay `+0x198` = **0**, flag `+0x190` = 0, motion `+0x18c` = 1 or 2.
- Start: x = `wx + 15`, y = `wy + 10` (the food widget's top-left; 40×40). **vx = −3.0 for motion 1, +3.0 for motion 2**; vy = −2.0.
- Appended to the food list `board+0xac`, z-order rebuilt. Food sits before fish in the z-order, but objects made during the object pass still update in that tick (`board.md`, the Prego newborn is the traced case), so the pellet most likely gets one `Food::Update` in its birth tick (`guess` until traced).

**Its motion** (`Food::Update` `0x004f8aa0`, the motion ≠ 0 path), each tick when not fading:

1. `y += 1.5` (the normal fall).
2. If `vy < 0`: `vy += 0.05`, then `y += vy`. So it first rises (net −0.5 px/tick at first) and slows to the plain 1.5 px/tick fall after about 40 ticks.
3. Motion 2: if `vx > 0`: `vx −= 0.05`, `x += vx`. Motion 1: if `vx < 0`: `vx += 0.05`, `x += vx`. A sideways glide of about 90 px over 60 ticks.
4. Clamp x to `[20, 550]`.
5. Then the usual floor rule (`y > 410` → snap, fade) and widget update.

The 0.05 steps are doubles; the last step can leave a tiny non-zero vx/vy (0.05 isn't exact), which only matters for which tick the sign test fails. Compare against a capture.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Animation (`0x004dfc50`), id 4

Turning is Itchy's (turn ±20, cel `9 − turn/2` or `turn/2 + 9`). When not turning, id 4 has its own branch:

- The swim cycle `+0x1b8` advances by **1 every tick** (the speed bucket is ignored), wraps to 0 above 19, and cel `+0x1c0` = cycle / 2.
- If the feed counter is exactly **−1** (the 9th tick after a drop), the cycle is then set to 10, so the swim resumes at cel 5 after the spit.

No RNG. `traced` (`zoo_b_zorf_meryl_wadsworth`).

## Drawing (`0x004df980`)

`IMAGE_ZORF`, 80×80 cels at `(cel·80, row·80)`:

| when | row | cel | mirror |
|---|---|---|---|
| turning (`+0x1c4 ≠ 0`) | 1 | swim cel | iff turn > 0 |
| counter `+0x220` < 0 (the 10 ticks from a drop) | 2 | `counter + 10` (0 … 9) | guppy rule |
| otherwise | 0 | swim cel | guppy rule |

The guppy rule is `Fish::Draw`'s mirror choice (by vx, facing when vx is 0; `fish-guppy.md#animation-and-drawing`). A type-0 shadow follows him like a guppy's. There are no alien rows: Zorf looks the same during a wave.

`static`.

## Open questions

- What sets `+0xf0` on a fish (step 2 of feeding skips such fish)? Probably a Virtual Tank or special-fish state.
- Whether the new pellet updates in its birth tick (likely, by the board rule) and the exact tick the vx/vy decay stops: trace against `demo_test_01`.
- Brinkley (pet 20) is the only thing that reads `board+0x194` here; his own spec will say whether he is ever in an Adventure tank with Zorf.
