# Food (`Sexy::Food`, type `0x1c`) and guppy feeding

Status: `draft`. Static analysis of the inner PE, checked against the live capture `tank1-1_feed_01` (4 pellets dropped, 2 eaten, 2 sank and faded). Addresses are inner-PE VAs.

Covers basic pellets dropped by the player. Potions (type 2), the "drop from above" variant (`+0x190`), drifting food (`+0x18c` ≠ 0) and level-3 food are only noted.

## Fields (raw offsets)

| Offset | Type | Meaning |
|---|---|---|
| `+0x158`, `+0x160` | double | x, y (top-left of the 40×40 widget) |
| `+0x168`, `+0x170` | double | vx (−3.0, or 3.0 for motion 2), vy (−2.0). Only used by drifting food |
| `+0x178` | int | animation counter, wraps at `speed·10` |
| `+0x17c` | int | animation speed: **RNG** `(Next() & 1) + 3` at construction |
| `+0x180` | int | fade-out countdown (15 → 0, then removed) |
| `+0x184` | byte | "dropped-from-above has settled" (variant only) |
| `+0x185` | byte | "being carried" — cannot be eaten, glides to (550, 30) |
| `+0x188` | int | food level (0 = basic). Copied from the global food level `0x005e89dc` at drop |
| `+0x18c` | int | motion type (0 = straight down) |
| `+0x190` | byte | dropped-from-above variant |
| `+0x194` | int | kind: 0 = pellet, 2 = potion |
| `+0x198` | int | edible delay; counts down each tick. Player drops set it to 0 (the ctor's default is 20) |
| `+0x30`, `+0x34` | int | widget position = `trunc(x), trunc(y)` |

## Dropping food (player click)

`Board` mouse-down handler (`0x00543920`), normal path (no aliens, no special widgets):

1. The click must be inside the tank: `31 ≤ x ≤ 586` and `61 ≤ y ≤ 399` (unsigned-range checks on `x − 31 ≤ 0x22b`, `y − 61 ≤ 0x152`). Also requires `board+0x2c8 ≤ 0`.
2. Unless free food is active (`board+0x440`), spend the food price `board+0x4ac` (5) via `0x00540b30`. If money is short, the buzzer and message play, and nothing else happens.
3. **`Board::AddFood`** (`0x00543280`) with `(x − 10, y − 10, motion 0, flag 0, delay 0, level −1)`:
   - **Count limit:** count = pellets on the board; `extra` = those with motion ≠ 0 or kind ≠ 0. If `count ≥ max_food + extra` (`max_food` = global `0x005df58c`, 1 at the start of tank 1-1), refund the price (`0x0053c280`: only when there are no aliens and free food is off) and show "You can only drop 1 food pellet at a time for now". No food.
   - Otherwise construct the food (`0x004ef840`): x, y as given; vx −3.0; vy −2.0; anim 0; fade 0; **RNG `(Next() & 1) + 3`** → speed; delay 20, then overwritten with 0 by AddFood. Level = the global food level. Play `SOUND_DROPFOOD`. Append to `board+0xac` and rebuild the z-order.

So a successful drop costs exactly one app-RNG draw (confirmed: one extra draw on each drop tick).

## `Food::Update` (`0x004f8aa0`), basic pellet

1. Return if the board is paused. Run the common `PreUpdate`.
2. `delay −= 1` if non-zero.
3. `anim += 1`; if `anim > speed·10 − 1`, set `anim = 0`.
4. If fading (`fade ≠ 0`): `fade −= 1`, and remove the food when it reaches 0. **Return either way**: a fading pellet doesn't move.
5. Fall: `y += 1.5` (unless "being carried").
6. If `y > 410` (or level 3 and `y > 400`): non-potion food snaps to `y = 410` and starts fading (`fade = 15` if not already set). Potions are removed. Level 3 explodes (RNG; not modelled).
7. Set the widget position to `trunc(x), trunc(y)`.

Food updates **before** guppies (z-order: Food precedes Fish).

## Guppy hunger and feeding

### `Fish::UpdateHunger` (slot 82 @ `0x004f15c0`) — replaces the hunger notes in `fish-guppy.md`

1. **Colour fade step** (`0x004d6c50`): if `fade (+0xa0) > 0`, it moves toward hungry (`+0xa8` set: +1, then reset to 0 after it passes 5) or toward fed (`+0xa8` clear: −1).
2. If there are no aliens: **hunger −= 1** (only while `+0x11c == −1`). When hunger becomes `+0xa4 + 4` (= 304; `+0xa4` is 300), start the "turning hungry" fade: `+0xa8 = 1`, `fade = 1`. Clamp hunger at −1000.
3. Starving check (`0x004d69d0`: `+0xac < 0` and hunger ≤ 0 outside Virtual Tank). A starving fish without the message flag (`+0x228`) dies at once.
4. Message ticks **return "not chasing" without chasing that tick** (all require `+0x228`):
   - hunger == −1: "Fish are hungry! Click to drop food!"
   - hunger == −200 (and `board+0x4c5` clear): "Fish are REALLY hungry!" plus sound
   - hunger == −400: "Fish are about to die of hunger!"
   - hunger ≤ −500: **die** (slot 88).
5. If hunger < 500 and (no aliens, or the first alien isn't type 4): return the result of the **food chase** (slot 84). Otherwise return "not chasing".

When this returns "chasing", `Fish::Update` skips the swim-mode state machine. Everything after it (timers, reroll, damping, clamps, integration) still runs.

### Food chase (slot 84 @ `0x004d5b20`)

1. `food = FindFood()` (slot 85, below).
2. Fish centre `fx = trunc(x + 40)`, `fy = trunc(y + 40)`.
3. If `mode_timer (+0x1b0) < 3`: skip steering.
4. Else if there is no food: return false. Otherwise:
   - `mode_timer = 0`. Target centre `tx = food.wx + 20`, `ty = food.wy + 20` (integers).
   - Pick constants by hunger. For **hunger > 300** (peckish): x steps (1.0, 0.1, 0.05), cap 3.0; y down 0.6 / 0.3, cap −2.0; y up 1.0 / 0.5, cap 3.0. For **hunger ≤ 300** (hungry): x steps (1.3, 0.2, 0.05), cap 4.0; y down 1.0 / 0.5, cap −3.0; y up 1.3 / 0.7, cap 4.0.
   - **x** (first matching row; a step applies only while vx is strictly inside the cap):

     | Condition | Change |
     |---|---|
     | `fx > tx + 8` | `vx −= big` |
     | `fx < tx − 8` | `vx += big` |
     | `fx > tx + 4` | `vx −= mid` |
     | `fx < tx − 4` | `vx += mid` |
     | `fx > tx` | `vx −= 0.05` |
     | `fx < tx` | `vx += 0.05` |

   - **y:** `fy > ty + 6` → `vy −= far_down`; `fy < ty − 6` → `vy += far_up`; otherwise `fy > ty` → `vy −= near_down`, `fy < ty` → `vy += near_up`. Down steps require `vy > down_cap`; up steps require `vy < up_cap`.
   - Speed bucket `+0x1bc`: +1 if < 5.
5. If there is food: run the **eat check** (slot 86).
6. Return "chasing" iff there is food.

### Find food (slot 85 @ `0x004ef050`)

Iterate the food list in order. For each pellet: `dx = trunc((wx + 20) − (x + 40))`, `dy = trunc((wy + 20) − (y + 40))`, `d = dx² + dy²` (int). Keep the **first** pellet with the strictly smallest `d` among those the fish can eat (`0x004d5f20`: not carried, delay 0, and kind 0; potions follow other rules). If the best `d < 10000`, set `+0xf8 = 100` (`+0x104` effects are off for guppies).

### Eat check (slot 86 @ `0x004ef1b0`)

With `cx = x + 40`, `cy = y + 40` (doubles), iterate the food list in order:

- **Bite box:** `wx + 5 < cx < wx + 35` and `wy < cy < wy + 35`, and edible. Then **eat** (slot 77), **remove the pellet**, set `eat_timer (+0x1c8) = 8` if it was 0, and stop iterating.
- Otherwise, if `eat_timer == 0` and the fish is in the **mouth-open box** (`wx − 10 < cx < wx + 50`, `wy − 5 < cy < wy + 40`) and the pellet is edible: `eat_timer = 20`. Keep iterating.

### Eat (slot 77 @ `0x004f1ba0`), basic pellet, outside Virtual Tank

1. `was_hungry_look` = `0x004d6f30` (hunger < 301 and colour fade 0).
2. **Slurp sound** (`0x005384c0`). If sample `0x138` is off cooldown (≥ 3 board ticks since it last played, `0x005381f0`), **RNG `Next() % 5`** picks one of three slurp samples. No draw when it's throttled.
3. `+0xf8 = 0`. (The hunger floor of 300 in `0x004d6a30` doesn't apply to guppies, since `+0xac < 0`.)
4. By food level (outside Virtual Tank):
   - Level 0: `hunger += 500`, plus 200 more with `+0x228`. **Clamp at 800.** `growth (+0x1a4) += 1`.
   - Level 1: +700, cap 1000, growth +2.
   - Level 2: +1100, cap 1400, growth +3 (the fall-through branch at `0x004f2079`).
   - Level 3 (star potion): grows medium/large guppies to stage 3; small ones explode. Not modelled yet.
5. **Grow:** if `growth ≥ growth_needed (+0x1a8)` and stage < 2: stage += 1, growth = 0, pulse `+0x1cc = 10`, sound `SOUND_GROW`, tutorial messages (`0x004f1840`). If `growth ≥ 15·growth_needed`, stage ≥ 2, stage ≠ 4 and `+0x1d0` is clear: stage = 4 (crowned).
6. If `was_hungry_look` and the fish no longer looks hungry: `+0xa8 = 0`, `fade = 5` (fade back to the fed colours).

Checked against the trace: hunger −14 + 700 = 686, and 297 + 700 → clamped to 800. The eat timer goes 20 → … while approaching, then is held because it's non-zero at the bite. The fade runs 5 → 0 after eating and 1 → 5 → 0 at hunger 304.

## Open questions

- The cooldown clock `board+0x28` for sample throttling: assumed to be the board's update counter (`guess`).
- Death (slot 88 @ `0x004d5f90`) spawns a `DeadFish`. Not modelled yet.

## Clicking on a guppy (`Fish` mouse-down, slot 55 @ `0x004e4c10`)

Guppies are widgets above the board, so a click that lands on a guppy's 80×80 rect goes to the guppy, not to the board's handler above. `confirmed` in `tank1-1_pres_01`, tick 4893: a click at (219, 113) on a guppy at widget (208, 94) produced a pellet with delay 25.

With the click local to the widget, `lx = x − wx` and `ly = y − wy`:

1. If any alien (`+0xb8`, `+0xf4`) or missile (`+0xdc`) exists, or the fish's `+0x229` is 0, nothing happens. Every guppy sets `+0x229 = 1` in its ctor, so for guppies this only means "no aliens or missiles".
2. **Tank bounds** use the fish's float position: `30 < lx + fish.x < 587` and `60 < ly + fish.y < 400` (strict). Outside them, nothing happens.
3. If a potion pellet is under the cursor (`0x00539f30`), it is picked up instead. Not in tank 1.
4. `TrySpend(food price, buzz)`. If that fails, nothing more happens.
5. **`AddFood`** at `(trunc(fish.x + lx − 10), trunc(fish.y + ly − 10))`, motion 0, flag 0, level −1, with the **edible delay**:
   - `(70 − ly) / 2` (C division) if `10 ≤ lx ≤ 70` and `10 ≤ ly ≤ 70` (a click on the fish's body, so the pellet starts out of reach of its own mouth);
   - 0 otherwise.
6. Set the auto-feed state: `+0x4ec = 1` and `+0x3c0` = now, the same as the board path.

The board path (`0x00543920`) handles clicks that reach the board: where no guppy, coin or other widget is under the cursor.
