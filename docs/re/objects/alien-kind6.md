# Alien kind 6 and kind-1 missiles (`Sexy::Alien` kind 6, `Sexy::Missle` kind 1)

Status: `partial (traced)`. Core behaviour (spawn, update, firing, animation, missile update, death) traced via `tank_3_4`. Drawing, sound and Wadsworth hit check sections `confirmed`. Addresses are inner-PE VAs. Offsets are raw `this`-relative.

Kind 6 is the new tank 3 alien. It shares the `Alien` class and most behaviour with the Destructor (kind 5, `alien-kind5.md`); **this spec only describes what differs for kind 6.** Anything not mentioned here behaves as in `alien-kind5.md`, which in turn defers to `alien-sylvester.md`. Notation is the same.

In short: kind 6 is a **slow floor walker** (divisor 3.5, y = 280) that fires **kind-1 missiles** at fish. Kind-1 missiles differ from the Destructor's kind-0 missiles: a click **redirects** them (changes their heading) instead of destroying them. Once redirected, the missile flies straight, unclamped, until it leaves the tank. Stanley shoots both kinds down the same way.

Confidence: most sections `traced` via `tank_3_4`; Wadsworth hit check (§6.2) `traced` via `tank_3_4_wadsworth`; drawing/sound sections `confirmed` from Ghidra decompilation (`Alien::DrawSprite` @ `0x004dae20`, `Alien::Update` @ `0x004fd560`); click redirect (§6.4) remains `static`.

## 1. Wave timing

`traced` (`tank_3_4`)

Kind 6 first appears on tank 3. The per-level table at `0x00541200` (tank 3 branch) assigns kind 6 to one or more levels. The wave countdown, re-roll formulas and pair-kind assignments for tank 3 are not yet read from the binary (`tank3-plan.md` step 1).

The countdown freeze rule is the same as kind 5: the next countdown doesn't start until the last missile (kind 1, on `+0xdc`) is gone (`alien-kind5.md` §1).

## 2. Spawn (`Board::AddAlien` @ `0x00545620`)

`traced` (`tank_3_4`)

The same steps and the same **3 app draws** as kind 1 (ctor 2, shadow 1). The arrival sample differs: for kind 6 the code at `0x0054589e` loads `0x134` **ROAR2** (`alien-kind5.md` §2, sound table `0x5e0eb0`).

### Constructor (`0x004ecfa0`), kind 6

`traced` (`tank_3_4`)

- **Divisor `+0x1b8` = 3.5**, **health `+0x1f0` = max `+0x1f8` = 220.0** (`alien-sylvester.md` §2 table).
- **y forced to 280**: the same branch as kind 5 (`0x597550`), outside mode 5 (`alien-kind5.md` §2).
- Shooting fields (shared defaults): fire timer `+0x1a4 = 0`, fire interval `+0x1a8 = 75`, fire animation `+0x1ac = 0`.
- Everything else as kind 1: 160×160, eat delay 100, warp 15, `+0x1d4 = 40`, `+0x1d8 = 20`.
- RNG: the same 2 app draws (`rand()` direction, `rand(10)` wander mode).
- Virtual Tank only (`app+0x880`): health = `grand() % 100 + 150`, `+0x1a8 = 144`. Not reached in Adventure.

## 3. `Alien::Update` (`0x004fd560`), kind 6

`traced` (`tank_3_4`)

The same top-level steps as kind 5 (`alien-kind5.md` §3), with these differences:

1. Virtual Tank fall to the floor: shared with kind 5 (y < 280 → `vy += 0.4 · div`). Not reached in Adventure.
2. Warp-in: unchanged.
3. **Chase or wander**: for kinds 5 and 6 the prey branch is skipped (`alien-kind5.md` §3). **Kind 6 never eats a fish.** It always wanders (§3.1).
4. Clamp: unchanged.
5. **vx manipulation** (before the move, `0x004fd560` lines 172–205): keyed on `cel % 5` when `fire_anim == 0` (normal swim), or clamped to `[-0.1, 0.1]` when `fire_anim > 0` (slow loop / pre-fire):
      - `fire_anim == 0`: `cel % 5 ∈ {1,2}` → accelerate (vx ± 0.55 away from 0); `cel % 5 ∈ {0,4}` → decelerate (if |vx| > 0.9, vx ∓ 0.8); `cel % 5 == 3` → no change.
      - `fire_anim > 0`: clamp vx to `[-0.1, 0.1]`.
6. **Move:** `x += vx / 3.5 + lunge`, `y += vy / 3.5`. With divisor 3.5, the cruise speed varies with the cel-keyed manipulation above.
7. Timers: unchanged.
8. **Firing** (§3.2).
9. Widget resize, animation (§3.3), death check.

### 3.1 Wander, kind 6

`traced` (`tank_3_4`)

Shared with kind 5 (`alien-kind5.md` §3.1): targets ±1.0 (modes 0/2 → −1.0, modes 1/3 → +1.0), vy is not steered, vx dithers ±0.1 toward its target. Re-roll timer pauses during the fire animation. The cruise speed through div 3.5 is ≈ 0.29 px/tick.

### 3.2 Firing (`0x004fdd34`)

`traced` (`tank_3_4`)

Kind 6 uses a **pre-fire** animation: the fire_anim starts 15 ticks before the interval expires. Each tick that reaches step 8, if the kind is 6 **and prey exists** (the same `0x004d4580` check as kind 5):

1. `+0x1a4 += 1`. If `+0x1a4 ≤ +0x1a8`:
   - At `+0x1a4 == +0x1a8 − 15`: if `turn == 0` and a target exists, set `fire_anim = 40` (start the pre-fire ping-pong animation, §3.3). If turning, subtract 2 from `fire_timer` to postpone.
   - Otherwise, stop.
2. When `+0x1a4 > +0x1a8`: `PickTarget()` (`0x004e8df0`), the same farthest-fish algorithm as kind 5 (`alien-kind5.md` §6.1). If no target, skip. Otherwise:
   1. The launch points depend on facing (`vx ≥ 0`):

      | Facing | Missile 1 (x, y) | Missile 2 (x, y) |
      |---|---|---|
      | vx ≥ 0 (right) | `(ix + 70, iy − 24)` | `(ix + 78, iy − 16)` |
      | vx < 0 (left) | `(ix + 10, iy − 24)` | `(ix + 2, iy − 16)` |

      `confirmed` from decompilation at `0x004fdd34`: the offsets are `0x46`/`0x18` and `0x4e`/`0x10` (right), `0x0a`/`0x18` and `0x02`/`0x10` (left). The second missile fires from a slightly offset position, not from the same point.
   2. `AddMissile(point1, target, kind 1)` (`0x00544130`, §5).
   3. `PickTarget()` again; if found, `AddMissile(point2, target, kind 1)` from the second position.
   4. Sample **`0x143`** (`SOUND_UNLEASH`?), gap 3.
3. Always: `+0x1a4 = 0`, **interval `+0x1a8 = rand(50) + 150`** (app; same formula as kind 5).
4. If fired: **fire animation `+0x1ac = 10`** (the post-volley snap, distinct from the pre-fire that used 40).

**Volley size:** up to **2** missiles (vs. kind 5's 3), based on the two `PickTarget` calls observed in the firing code (`tank3-plan.md`). Each missile goes to a different fish. The first `PickTarget` takes argument 0, the second argument 1; the argument is ignored (`alien-kind5.md` §6.1).

### 3.3 Animation (`0x004daa00`), kind 6

`traced` (`tank_3_4`)

Three animation modes, after decrementing `fire_anim` if > 0:

1. **Fire animation** (`fire_anim > 0`): kind 6 uses a **ping-pong** over 40 ticks: cel = `9 − (fire_anim − 20) / 2` when `fire_anim > 20`, else `(fire_anim − 1) / 2`. This cycles 0→9→0. The pre-fire (40 ticks) and post-volley (10 ticks) both use this formula.
2. **Turn** (`fire_anim == 0, turn ≠ 0`): cel = `9 − |turn| / 2` (0..9 symmetric, 18-tick turn).
3. **Normal swim** (`fire_anim == 0, turn == 0`): anim increments by 1 (or 2 if `|vx| > 8.0`), wraps at 80; cel = `anim / 8` (10 cels, 0..9). The original spec's "stroke" cycle was wrong; it's a simple counter.

## 4. Clicks and damage

### 4.1 Input routing

`traced` (`tank_3_4`)

Same as kind 5 (`alien-kind5.md` §4.1), except that the missile walk (`0x004d82a0`) now encounters kind-1 missiles instead of kind 0. For kind 1, a click **redirects** the missile (§6.4) instead of destroying it. The click-to-redirect still counts as "handled" for the walk (stops at the first redirected missile).

After the alien has died while kind-1 missiles are still in flight, clicks stay shots (no food drops).

### 4.2 `Alien::Hit(cx, cy)`, kind 6

`traced` (`tank_3_4`)

Shared with kind 5 (`alien-kind5.md` §4.2): **damage `2 · laser + 2`**, sideways-only knockback (none during the fire animation). At laser 2, 6 per hit → **37 hits** to kill 220 HP.

### 4.3 Pets and other damage sources

`static`

Same reduced-damage table as kind 5 (`alien-kind5.md` §4.3): Itchy 0.25, Gash 0.5, Rufus 0.25, Stanley's ball 15.

### 4.4 Death (`0x004f9c70(1)`), kind 6

`traced` (`tank_3_4`)

Same as kind 5 (`alien-kind5.md` §4.4): diamond, kind-1 debris, `AddDeadAlien` with the floor clamp (y ≤ 280). The dead alien uses `IMAGE_KIND6` (§7.1). **Its own missiles are not removed**: they keep flying, can still kill fish, can still be shot down or redirected, and keep the end-of-wave check waiting. `kill_alien()` only removes kind-2 missiles (Stanley's balls) that target the dying alien; kind-0 and kind-1 missiles are retained.

## 5. The missile: `Sexy::Missle` kind 1

`static`

`AddMissile(x, y, target, kind 1)` (`0x00544130`) is the same path as kind 0 and kind 2 (`pet-stanley.md`): allocate, ctor, add to list `+0xdc`, bring to front, set `target+0x98 = missile`.

### 5.1 Constructor (`0x004eae40`), kind 1

`traced` (`tank_3_4`)

Same as kind 0 (`alien-kind5.md` §5.1) except:

| Offset | Meaning | Kind 1 |
|---|---|---|
| `+0x190` | redirected byte | 0 (set to 1 on click redirect, §6.4) |
| `+0x19c` | kind | **1** |

RNG: **1 global draw** (`grand() % 4`, the row), same as kind 0. Widget 80 × 80.

### 5.2 `Missle::Update` (`0x004ffb30`), kind 1

`traced` (`tank_3_4`)

Same steps as kind 0 (`alien-kind5.md` §5.2) with these differences:

1. **Shot down by Stanley**: unchanged (kinds 0 and 1 both eligible, `pet-stanley.md`; the `m.kind > 1` check in Stanley's code excludes only kind 2+).
2. **Steer and hit**: while not redirected (`+0x190 == 0`), same as kind 0 (steer toward target, hit test). Once redirected: no steering (the missile flies straight at its current velocity).
3. **Hit test** (`0x004fc070`, kind-1 path at `0x004fc8b6`): same box as kind 0, but **with a Wadsworth shelter check** (`alien-kind5.md` §6.2). If the target guppy is sheltered (Wadsworth active and stage < 2), the missile does not hit.
4. **Clamp**: while not redirected, same as kind 0 (`x ∈ [10, 550]`, `y ∈ [95, 370]`). **Once redirected, no clamping.**
5. **Out-of-bounds removal**: once redirected, if `x > 580 || x < −20 || y > 380 || y < 45` (the same bounds as the ball, `alien-kind5.md` §5.2 note), the missile is removed (§6.3).
6. **Move**: `x += vx / 0.8`, `y += vy / 0.8`; launch timer; widget resize. Unchanged.
7. **Smoke trail**: kind-1 missiles have **no smoke trail**. The RNG draw counts at launch+0 match the baseline with no extra draws. ~~The original static claim that it was "same as kind 0" was wrong.~~
8. **Heading cel**: same `0x004e2170` function as kind 0.

### Speed and behaviour summary

Before redirect: identical to kind 0. Steers toward its fish, clamped to the tank, up to ~2.25 px/tick.

After redirect: flies straight at its current velocity (up to ~2.5 px/tick), no clamping, exits the tank in a few seconds. The player can aim it away from their fish by choosing where to click on the missile.

## 6. Targets, steering, hits, explosions, clicks

### 6.1 Target choice and steering

`traced` (`tank_3_4`)

Same as kind 5 (`alien-kind5.md` §6.1): `PickTarget` picks the farthest eligible fish; steering is the same table. No change for kind 6 or kind 1.

### 6.2 Hit test, kind 1

`traced` (`tank_3_4_wadsworth`; previously `confirmed` by cross-referencing `alien-kind5.md` §6.2, `pet-wadsworth.md` and the decompiled prey filter)

Same box as kind 0 (alien-kind5.md §6.2), but with the **Wadsworth shelter check** at `0x004fc8b6` (`pet-wadsworth.md`): if `0x005e89d0 ≠ 0` (Wadsworth is sheltering) and the target guppy's stage `+0x1a0 < 2`, the hit is blocked. The missile keeps flying past the fish. (Kind 0 has no such check: a sheltered guppy that was targeted before Wadsworth started is still hit by kind 0.)

The shelter check on the kind-1 path mirrors the prey filter `FUN_00500ff0`, which also gates on fish type `+0x8c == 0` (guppy); for kind-1 missiles this is always true since `PickTarget` only selects guppies. The traced `alien-kind5.md` §6.2 explicitly confirms: "the check at `0x004fc8b6` … belongs to the kind-1 path" and "Kind 1 differs in … its hit path (which has the Wadsworth check)." The traced `pet-wadsworth.md` lists `0x004fc070 (0x004fc8b6)` as a reader of `DAT_005e89d0`.

On a hit (not sheltered), the same as kind 0: clear `+0x98`, kill the target fish (DIE, corpse), remove the missile (§6.3).

### 6.3 Removal and explosion, kind 1

`traced` (`tank_3_4`)

Same as kind 0 (`alien-kind5.md` §6.3): clear the target's `+0x98`, leave the lists, end-of-wave check, EXPLODE (gap 3), debris (2..4 sparks). **Additionally**, kind 1 plays sample **`0x11f`** before the EXPLODE (`alien-kind5.md` §8). The `0x11f` sound is unidentified (possibly a whistle or whoosh on removal).

### 6.4 Clicks on missiles (`0x004d82a0`), kind 1

`static`

The click box is the same as kind 0: 60 × 60 around the centre, launch timer < 1 (15 ticks old). But instead of removing the missile:

1. Set `+0x190 = 1` (**redirected**).
2. Clear the target fish's `+0x98` (no longer homing on it).
3. **Reverse the velocity**: `vx = −vx`, `vy = −vy`. The missile now flies in the opposite direction. (**Needs Ghidra verification**: the redirect might use the click position to set the direction instead of a simple reversal.)
4. Return true (the walk stops).

After redirect, the missile:
- No longer steers toward any target.
- Is no longer clamped to the tank bounds.
- Is removed when it reaches the out-of-bounds zone (§5.2 step 5).
- Can still be shot down by Stanley.
- Can still hit its former target if it circles back (unlikely with a reversal).

**A redirected missile does not explode on click.** Subsequent clicks on the same missile have no effect (the `redirected` flag is already set, and the kind-1 click code only runs when not yet redirected — **needs Ghidra verification**).

## 7. Drawing and sound

### 7.1 Alien, kind 6

`confirmed` (from `Alien::DrawSprite` @ `0x004dae20`)

- **Sheet: `IMAGE_KIND6`** (global `DAT_005e8e58`). **10 columns × 3 rows**, 160 × 160 cels.
- **Row selection**: row 0 = normal swim; row 1 = turning (mirror by turn direction); **row 2 = fire animation** (`+0x1ac > 0`). The fire animation takes priority over turning.
- **Turn counter**: **±20** for kind 6 (not ±10 like other kinds), from `0x004daa00`.
- Mirror by vx as kind 1. Column = cel `+0x1c0` (0..9).
- **Hit flash** (white additive, alpha `25 · +0x200`): same as kind 1.
- **No fire flash**: the additive fire-flash code in `DrawSprite` checks `kind == 5` specifically (`0x004db0aa`). Kind 6 does **not** have the white fire pulse. (The earlier `static` claim was wrong.)
- **Dead alien**: `DeadAlien` draw maps kind 6 to `IMAGE_KIND6` (`DAT_005e8e58`).
- Warp-in scaling, shadow, health bar (Blip): as kind 1.

### 7.2 Missile, kind 1

`static`

Same as kind 0 (`alien-kind5.md` §7.2): `IMAGE_MISSILE`, heading cel, crosshair on the targeted fish. **Additionally**, kind 1 is drawn with an **additive glow** (`alien-kind5.md` §8): the missile image is drawn a second time additively, using the image at global `0x005e8a44`. This makes kind-1 missiles visually brighter (easier to spot and click).

### 7.3 Sounds

`confirmed` (spawn from `AddAlien` @ `0x0054589e`; volley from `Alien::Update` @ `0x004fde47`)

| Event | Sample | Notes |
|---|---|---|
| Spawn | `0x134` ROAR2 | direct, unthrottled (kind 5: `0x127` INTERFER) |
| Volley fired | `0x143` | gap 3, once per volley |
| Laser hit | `0x126` HIT | gap 3 |
| Click (redirect) | `0x144` ZAP (+`0x145`, +`0x132`) | the usual shot sound, 2 app draws |
| Missile removed (any cause) | `0x11f` then `0x11d` EXPLODE | gap 3 each |
| Fish killed by a missile | `0x117` DIE | gap 3 |
| Stanley shoots | `0x122` GROW | `pet-stanley.md` |
| Death | `0x11d`, `0x11e` | the same as kind 1/5 |

## 8. Effects on the board

`traced` (`tank_3_4`)

Same as kind 5 (`alien-kind5.md` §8): kind-1 missiles on `+0xdc` freeze the wave countdown, keep clicks as shots while the alien is dead, trigger Wadsworth's shelter, and keep the end-of-wave check waiting. Hunger and coins resume as soon as the alien dies (the `+0xb8` test, not `+0xdc`).

## 9. RNG summary

`traced` (`tank_3_4`)

Same structure as kind 5 (`alien-kind5.md` §9), with:
- Spawn: 3 app draws (ctor 2, shadow 1). No global draws in Adventure.
- Per volley: 1 global draw per missile created (ctor `grand() % 4`), then `rand(50)` (interval).
- Each kind-1 missile update: **no smoke trail** (unlike kind 0). No per-tick `r % 3` draw.
- Kind-1 missile removal: `r % 3 + 2` sparks, each `r % 3`, `r % 30`, `r % 30`, shot ctor.
- Kind-1 redirect (click): no extra RNG (the shot-sound draws are from the board, not the missile).

## Deviations

`traced` (`tank_3_4`)

Same as kind 5: PickTarget ties by heap address (sim uses list order, replay can override). Effect-shot start delay (heap-dependent, sim never draws it).

**Corrections from `tank_3_4` replay:**
- Kind-1 missiles have no smoke trail (the static claim was wrong).
- Kind-6 animation is a simple anim/8 cycle (not a "stroke" pattern).
- The vx manipulation is keyed on `cel % 5`, not on stroke deltas.
- `fire_anim` uses a ping-pong (0→9→0 over 40 ticks), not a simple countdown.
- The pre-fire animation starts at `interval − 15`, not at the volley itself.
- `kill_alien()` only removes kind-2 missiles (Stanley's balls), not kind-0/kind-1.

**Corrections from Ghidra read (`t3-ghidra-c-combat`):**
- The second missile fires from a different position than the first (§3.2 table).
- Kind 6 does NOT have the fire flash — the `DrawSprite` check is `kind == 5` only.
- The kind-6 sprite sheet is 10 × 3 (row 2 for the fire animation), not 10 × 2.

## Open questions

- ~~**Volley size**: confirmed 2 via `tank_3_4`.~~
- ~~**Launch positions**: confirmed via `tank_3_4`.~~
- ~~**Firing interval**: confirmed `rand(50) + 150` via `tank_3_4`.~~
- **Redirect mechanic**: assumed velocity reversal (`vx = −vx, vy = −vy`). May use click position instead — not yet individually confirmed by a recording that exercises redirect.
- **Redirect re-clickable**: assumed only one redirect per missile. Not yet individually confirmed.
- ~~**Hit with Wadsworth**: `traced` via `tank_3_4_wadsworth` (3-4 with Wadsworth; kind-1 missiles fire at sheltered guppies, hits blocked, large guppies killed). Previously `confirmed` via cross-referencing traced specs and the decompiled prey filter.~~
- ~~**Kind-6 image global**: confirmed `DAT_005e8e58` (`IMAGE_KIND6`) from `DrawSprite`.~~
- ~~**Sound `0x143`**: confirmed as the volley sound from the `Alien::Update` firing code.~~
- **Sound `0x11f`**: played before EXPLODE on kind-1 removal. Identify (also used by kind `0x14` death).

## Recording

`traced` (`tank_3_4`)

`tank_3_4` (`tank3-4_full_01`): full level 3-4 playthrough. Exercised kind-6 spawn, firing (volleys of 2 kind-1 missiles), pre-fire animation, vx manipulation, death with missile retention, laser combat. The replay matches tick-by-tick.

Still not exercised: click-to-redirect, Stanley vs kind-1 missile.

`tank_3_4_wadsworth` (`tank3-4_wadsworth_01`): 3-4 with Stinky, Niko and Wadsworth. Exercised: kind-6 alien firing kind-1 missiles while Wadsworth shelters — shelter blocks hits on stage-0/1 guppies, large guppies killed normally. 205 missile ticks, all with shelter active. The replay matches tick-by-tick.
