# Rufus the fiddler crab (`Sexy::OtherTypePet`, pet id 7)

Rufus walks along the tank floor. When the tank is calm he wanders with a 9-way mode (stop, walk or run left/right) and snaps his claw when standing still. While any alien or Bilaterus is present he runs toward the nearest one along the floor, and every tick his body overlaps an alien he takes **2 HP** off it (0.25 or 0.5 for some kinds) and stops. He cannot leave the floor, so he only hurts aliens that come down low. He is the pet hatched after tank 2-3 (`screens/hatch-screen.md#pet-award-table-adventure-first-play-through`).

Same class, fields and update skeleton as Stinky (`pet-stinky.md`) and Clyde (`pet-clyde.md`). Only the id-7 differences are listed here. All offsets are raw `this`-relative.

- Ctor `OtherTypePet::OtherTypePet` @ `0x004eb5c0`
- Update (slot 22) @ `0x004f2e80`, id-7 branch
- Attack gate `0x004efe80` → chase `0x004ebc80` → target `0x004e7270` (id-7 branch), hit `0x004e7c30` (id-7 branch at the top)
- Animation `0x004eb7c0` (id-7 path)
- Draw (slot 26) `0x004f37c0` → `0x004f3590` case 7
- Sound: `0x00538340(gap)` = `Board::PlaySample(0x130, gap)` (`0x00538230`), the same hit sound as Itchy's stab

Confidence: `static` throughout. No capture with Rufus exists yet.

## Fields used by id 7

As Stinky's table (`pet-stinky.md#fields`), with these differences:

| Offset | Meaning for Rufus | Initial |
|---|---|---|
| `+0x160` | y: **forced to 365.0** unless the drop-in flag is set | 365 |
| `+0x188` | speed divisor: the default | **2.0** |
| `+0x190` | wander mode, **0..8** after the first reroll (the ctor's `% 10` can give 9, which keeps the target unchanged) | `Next() % 10` |
| `+0x1a0` | animation counter (0..59 idle, 0..39 otherwise) | 0 |
| `+0x1b4` | unused by id 7 (no shell) | 0 |

`static`.

## Construction

`Board::AddPet(7, -1, -1, 0, 0)` (`0x00544a90`) → ctor → shadow. App MTRand draws, in order:

1. AddPet: `x = Next() % 265 + 105`.
2. AddPet: `y = Next() % 520 + 20` (discarded: the ctor forces y = 365).
3. Ctor: `mode = Next() % 10`.
4. Ctor: `+0x1b8 = Next() % 250 + 250` (unused).
5. Shadow ctor: `y = 410 + Next() % 5`. The shadow is **type 1** (`0x00544800`: every OtherTypePet except ids 1 and 5), which keeps its ctor y, like Stinky's.

**5 app draws.** Divisor 2.0 (`0x005c3cc8`, the default). No other id-7 ctor code.

`static`.

## Update (`0x004f2e80`), id 7

Same skeleton as Stinky's update. Board `tank` = `board+0x3c8`.

1. Return if there is no board or it is paused. `PreUpdate` (`0x004f22c0`).
2. Drop-in only: `vy += 0.1` while y < 380 (ids 0, 1, 7, 14). At level start the flag is clear.
3. **Attack or wander.** Except on tank 5, call the gate `0x004efe80` (id 7): if an alien (`board+0xb8`) or a Bilaterus (`board+0xf4`) exists, run the chase `0x004ebc80` ([Attack](#attack)) and report "chasing", else "not chasing". There is no coin chase for Rufus.
   - When not chasing (or on tank 5): set target vx `+0x180` from the mode:

     | mode | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | ≥ 9 |
     |---|---|---|---|---|---|---|---|---|---|---|
     | target vx | 0 | −0.5 | +0.5 | −1.0 | +1.0 | 0 | 0 | −2.5 | +2.5 | unchanged |

     So 3 of 9 modes stand still, 2 walk, 2 trot, 2 run.
   - **Every tick (chasing or not):** if target < vx, `vx −= 0.1`; then if vx < target, `vx += 0.1`. No snap, as Clyde (`pet-clyde.md`, step 4). While chasing, the target is the stale value from the last calm tick (or 0 after a hit, see below).
4. Counters and reroll: `+0x194 += 1`, `+0x198 += 1`. Reroll if `+0x198 > 20`, or (`x ≤ 10` and target ≤ 0), or (`x ≥ 540` and target ≥ 0): `+0x198 = 0`, **app `Next() % 10`**; if 0, **app `mode = Next() % 9`** (ids 7 and 14). This runs during attacks too (the mode just isn't used then). `+0x1ac −= 1` if non-zero; `+0x1a8 += 1` while < 20. The shell routine `0x004e2a20` does nothing for id 7.
5. **Clamp:** x ≤ **560** (id 7's own bound, `0x00599040`), x ≥ 10. y > **365** → y = 365 and vy = 0. y < 95 → 95. Rufus has no vertical motion of his own, so outside a drop-in he stays at y = 365.
6. Edge modes, as Stinky: x > 535 and vx > 0.1 → mode 1; x < 15 and vx < −0.1 → mode 2. (Mode 1 is the slow walk left.)
7. **Animation** `0x004eb7c0`, id 7 ([Animation](#animation)).
8. **Move:** `x += vx / 2`, `y += vy / 2` (÷ 10 with the slow flag `+0x1b0`, which only the Virtual Tank sets; id 7 has no call to the sleepy helper in this update, so it stays 0).
9. Widget position = `trunc(x), trunc(y)`.

RNG per tick: only the reroll in step 4 (`% 10`, then `% 9` w.p. 1/10). All app MTRand. The attack and its sound draw nothing.

`static`.

## Attack

`0x004ebc80`, id 7, called every tick while an alien or Bilaterus exists (not on tank 5).

1. **Target** (`0x004e7270`, id 7), best = 100 000 000, strict `<` (first minimum wins):
   - first every Bilaterus in `board+0xf4`, list order: `dx = trunc((b.wx + 40) − (x + 40))`, `dy = trunc((b.wy + 40) − (y + 40))`, `d = dx² + dy²`;
   - then every alien in `board+0xb8`, list order, skipping aliens with byte `+0x204` set: same with `+80` (the 160-px alien widget's centre).
2. **Steering**, only when `+0x194 > 4` and there is a target; then `+0x194 = 0`. With `cx = x + 40` and `tx = target.wx + 80` (`+ 40` for a type `0x17` target, i.e. a Bilaterus segment):
   - `cx > tx` and `vx > −5`: `vx −= 1.8`;
   - else `cx < tx` and `vx < 5`: `vx += 1.8`.

   Horizontal only. With the 0.1/tick drift in between, he reaches ±5.3 at most (−5 < vx before a −1.8 step).
3. **Hit** (`0x004e7c30`, id 7), every tick there is a target (independent of the steering counter). No early return: every overlapping enemy is hit in the same tick.
   - Each Bilaterus in `+0xf4`, list order: if `|(x + 40) − (b.wx + 40)| < 10` and `|(y + 40) − (b.wy + 40)| < 10`: the health of the segment's owner (`[b+0x158] + 0x1d8`, double) `−= 2.0`; `PlaySample(0x130, gap 10)`; vx = 0, target vx = 0.
   - Each alien in `+0xb8`, list order: if `a.wx + 30 < x + 40 < a.wx + 140` and `a.wy + 10 < y + 40 < a.wy + 150` (doubles, strict) and `a+0x204 == 0`: alien health `+0x1f0` −= **0.25 for kind 5, 0.5 for kinds 4 and 6, 2.0 otherwise**; `PlaySample(0x130, gap 10)`; vx = 0, target vx = 0.

   On the floor `y + 40 = 405`, so an alien is in reach only while `255 < a.wy < 395`. The hit zeroes his speed, so he stands under the alien and keeps hitting it every tick it stays in the box; once it drifts out, the next steering tick (≤ 5 ticks later) starts him moving again.

Damage per second at 2 HP/tick (≈ 71 HP/s): Sylvester (50 HP, `alien-sylvester.md`) dies after 25 ticks in the box. Health ≤ 0 is handled by the alien's own update, not here.

`static`.

## Effects on other objects

- **Aliens** (`board+0xb8`): health `+0x1f0` reduced in `0x004e7c30` (the kind-dependent subtraction at `0x004e7f22`..`0x004e7fe8`). Kinds with `+0x204` set are immune and also not targeted.
- **Bilaterus** (`board+0xf4`): health at `[segment+0x158] + 0x1d8` reduced by 2.0 per overlapping segment per tick (`0x004e7d7c`).
- **Coins, food, fish:** none.

`static`.

## Animation

`0x004eb7c0`, id 7. No RNG. Counter `a` = `+0x1a0`, cel = `+0x1a4`. "Still" means `−0.01 ≤ vx ≤ 0.01` and target vx == 0.

1. **Idle claw snap**: still and no alien in `board+0xb8` (Bilaterus doesn't count here): `a = (a + 1) % 60`, cel: a < 3 → a/2 (0, 1); a < 24 → 2; a < 33 → a/2 − 10 (2..6); a < 54 → 7; a < 60 → a/2 − 20 (7..9). Done.
2. Otherwise first `a = a % 40`. Then:
   - **Hitting**: still, no boss (`board+0x110` null), and the target search finds something: `a = (a + 4) % 40`, cel = a/4.
   - **Walking** (everything else): vx ≥ 1 → `a += 2`; 0 < vx < 1 → `a += 1`; −1 < vx ≤ 0 → `a −= 1`; vx ≤ −1 → `a −= 2`; wrap into 0..39; cel = a/4. So the walk cycle plays backwards when he walks left.
3. A missile overlay counter (`+0x98`) steps if attached, as every pet.

No turn counter and no facing update.

`static`.

## Clicks

As Clyde (`pet-clyde.md#clicks`): a left click goes through to the board; a right click is handled locally (`0x004f3a00`).

`static`.

## Drawing

Draw (slot 26 @ `0x004f37c0`) → `0x004f3590(g, mirror = 0)`: **never mirrored** (the walk direction is shown only by the cel order). Case 7 draws `IMAGE_RUFUS` (`0x005e8b60`, `resources.xml`: 10 × 2 cels of 80 × 80) at (0, 0), source `(cel · 80, row · 80, 80, 80)`:

- row 0 while moving (`vx > 0.01`, `vx < −0.01`, or target vx ≠ 0);
- row 1 while still (the idle snap and the hitting animation).

No ZZZ image for id 7. Then the shared tail (drop-in flash, missile overlay, drop-in timer). Shadow: type 1.

`static`.

## Summary / open questions

- **5 app draws** on AddPet (x, discarded y, mode, `+0x1b8`, shadow). Per tick only the wander reroll (`% 10`, then `% 9`); the attack draws nothing.
- 2 HP/tick to aliens whose box holds his centre (0.25 kind 5, 0.5 kinds 4/6), 2 HP/tick per Bilaterus segment within ±10 px; sound `0x130` every 10 ticks at most; stops on each hit.
- Open: which alien kinds 4, 5, 6 are (by name) and whether any reach y > 255 in practice; whether the dithering vx (no snap) shows up as a 0.1 creep when idle (check the idle snap actually triggers, it needs |vx| ≤ 0.01); Bilaterus segment layout (`+0x158` owner, `+0x1d8` health) to be confirmed in its own spec.
- First capture should be tank 2-4 (the first level he can be brought to) with a low-swimming alien.
