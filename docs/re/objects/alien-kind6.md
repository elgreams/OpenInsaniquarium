# Alien kind 6 and kind-1 missiles (`Sexy::Alien` kind 6, `Sexy::Missle` kind 1)

Status: `draft (static)`. Nothing here has been captured yet. Every claim comes from the existing kind-5 spec (`alien-kind5.md`), the Stanley spec (`pet-stanley.md#open-questions`), and static analysis. Addresses are inner-PE VAs. Offsets are raw `this`-relative.

Kind 6 is the new tank 3 alien. It shares the `Alien` class and most behaviour with the Destructor (kind 5, `alien-kind5.md`); **this spec only describes what differs for kind 6.** Anything not mentioned here behaves as in `alien-kind5.md`, which in turn defers to `alien-sylvester.md`. Notation is the same.

In short: kind 6 is a **slow floor walker** (divisor 3.5, y = 280) that fires **kind-1 missiles** at fish. Kind-1 missiles differ from the Destructor's kind-0 missiles: a click **redirects** them (changes their heading) instead of destroying them. Once redirected, the missile flies straight, unclamped, until it leaves the tank. Stanley shoots both kinds down the same way.

Confidence: `static` for the whole file (per section below).

## 1. Wave timing

`static`

Kind 6 first appears on tank 3. The per-level table at `0x00541200` (tank 3 branch) assigns kind 6 to one or more levels. The wave countdown, re-roll formulas and pair-kind assignments for tank 3 are not yet read from the binary (`tank3-plan.md` step 1).

The countdown freeze rule is the same as kind 5: the next countdown doesn't start until the last missile (kind 1, on `+0xdc`) is gone (`alien-kind5.md` §1).

## 2. Spawn (`Board::AddAlien` @ `0x00545620`)

`static`

The same steps and the same **3 app draws** as kind 1 (ctor 2, shadow 1). The arrival sample differs: for kind 6 the code at `0x0054589e` loads `0x134` **ROAR2** (`alien-kind5.md` §2, sound table `0x5e0eb0`).

### Constructor (`0x004ecfa0`), kind 6

`static`

- **Divisor `+0x1b8` = 3.5**, **health `+0x1f0` = max `+0x1f8` = 220.0** (`alien-sylvester.md` §2 table).
- **y forced to 280**: the same branch as kind 5 (`0x597550`), outside mode 5 (`alien-kind5.md` §2).
- Shooting fields (shared defaults): fire timer `+0x1a4 = 0`, fire interval `+0x1a8 = 75`, fire animation `+0x1ac = 0`.
- Everything else as kind 1: 160×160, eat delay 100, warp 15, `+0x1d4 = 40`, `+0x1d8 = 20`.
- RNG: the same 2 app draws (`rand()` direction, `rand(10)` wander mode).
- Virtual Tank only (`app+0x880`): health = `grand() % 100 + 150`, `+0x1a8 = 144`. Not reached in Adventure.

## 3. `Alien::Update` (`0x004fd560`), kind 6

`static`

The same top-level steps as kind 5 (`alien-kind5.md` §3), with these differences:

1. Virtual Tank fall to the floor: shared with kind 5 (y < 280 → `vy += 0.4 · div`). Not reached in Adventure.
2. Warp-in: unchanged.
3. **Chase or wander**: for kinds 5 and 6 the prey branch is skipped (`alien-kind5.md` §3). **Kind 6 never eats a fish.** It always wanders (§3.1).
4. Clamp: unchanged.
5. **Move:** `x += vx / 3.5 + lunge`, `y += vy / 3.5`. With divisor 3.5, the cruise speed is 1/3.5 ≈ 0.29 px/tick (much slower than the Destructor's 0.83). The initial ±3.0 entry speed gives 3/3.5 ≈ 0.86 + the 3.0 lunge.
6. Timers: unchanged.
7. **Firing** (§3.2).
8. Widget resize, animation (§3.3), death check.

### 3.1 Wander, kind 6

`static`

Shared with kind 5 (`alien-kind5.md` §3.1): targets ±1.0 (modes 0/2 → −1.0, modes 1/3 → +1.0), vy is not steered, vx dithers ±0.1 toward its target. Re-roll timer pauses during the fire animation. The cruise speed through div 3.5 is ≈ 0.29 px/tick.

### 3.2 Firing (`0x004fdd34`)

`static`

Each tick that reaches step 7, if the kind is 6 **and prey exists** (the same `0x004d4580` check as kind 5):

1. `+0x1a4 += 1`. If `+0x1a4 ≤ +0x1a8`, stop.
2. `PickTarget()` (`0x004e8df0`): the same farthest-fish algorithm as kind 5 (`alien-kind5.md` §6.1). If no target, skip. Otherwise:
   1. The launch point depends on facing (`vx ≥ 0`):

      | Facing | Launch (x, y) |
      |---|---|
      | vx ≥ 0 (right) | `(ix + 70, iy − 24)` |
      | vx < 0 (left) | `(ix + 10, iy − 24)` |

      Source: `pet-stanley.md#open-questions` (`0x004fddac`, `(wx + 10, wy − 24)`) and the mirror offset `0x46` = 70 from `tank3-plan.md`. The y-offset −24 is shared.
   2. `AddMissile(point, target, kind 1)` (`0x00544130`, §5).
   3. `PickTarget()` again; if found, `AddMissile(point, target, kind 1)` from the same position.
   4. Sample **`0x143`** (`SOUND_UNLEASH`?), gap 3.
3. Always: `+0x1a4 = 0`, **interval `+0x1a8 = rand(50) + 150`** (app; same formula as kind 5 — **needs Ghidra verification**).
4. If fired: **fire animation `+0x1ac = 10`** (§3.3).

**Volley size:** up to **2** missiles (vs. kind 5's 3), based on the two `PickTarget` calls observed in the firing code (`tank3-plan.md`). Each missile goes to a different fish. The first `PickTarget` takes argument 0, the second argument 1; the argument is ignored (`alien-kind5.md` §6.1).

### 3.3 Animation (`0x004daa00`), kind 6

`static`

Shared with kind 5 (`alien-kind5.md` §3.3): fire animation `+0x1ac` cels 9..1, then always the slow loop (kinds 5, 6, 7 and 0x15 skip the stroke branch). Turn detection unchanged. The slow-loop cruise is 0.29 px/tick at divisor 3.5.

## 4. Clicks and damage

### 4.1 Input routing

`static`

Same as kind 5 (`alien-kind5.md` §4.1), except that the missile walk (`0x004d82a0`) now encounters kind-1 missiles instead of kind 0. For kind 1, a click **redirects** the missile (§6.4) instead of destroying it. The click-to-redirect still counts as "handled" for the walk (stops at the first redirected missile).

After the alien has died while kind-1 missiles are still in flight, clicks stay shots (no food drops).

### 4.2 `Alien::Hit(cx, cy)`, kind 6

`static`

Shared with kind 5 (`alien-kind5.md` §4.2): **damage `2 · laser + 2`**, sideways-only knockback (none during the fire animation). At laser 2, 6 per hit → **37 hits** to kill 220 HP.

### 4.3 Pets and other damage sources

`static`

Same reduced-damage table as kind 5 (`alien-kind5.md` §4.3): Itchy 0.25, Gash 0.5, Rufus 0.25, Stanley's ball 15.

### 4.4 Death (`0x004f9c70(1)`), kind 6

`static`

Same as kind 5 (`alien-kind5.md` §4.4): diamond, kind-1 debris, `AddDeadAlien` with the floor clamp (y ≤ 280). The dead alien uses `IMAGE_KIND6` (§7.1). **Its own missiles are not removed**: they keep flying, can still kill fish, can still be shot down or redirected, and keep the end-of-wave check waiting.

## 5. The missile: `Sexy::Missle` kind 1

`static`

`AddMissile(x, y, target, kind 1)` (`0x00544130`) is the same path as kind 0 and kind 2 (`pet-stanley.md`): allocate, ctor, add to list `+0xdc`, bring to front, set `target+0x98 = missile`.

### 5.1 Constructor (`0x004eae40`), kind 1

`static`

Same as kind 0 (`alien-kind5.md` §5.1) except:

| Offset | Meaning | Kind 1 |
|---|---|---|
| `+0x190` | redirected byte | 0 (set to 1 on click redirect, §6.4) |
| `+0x19c` | kind | **1** |

RNG: **1 global draw** (`grand() % 4`, the row), same as kind 0. Widget 80 × 80.

### 5.2 `Missle::Update` (`0x004ffb30`), kind 1

`static`

Same steps as kind 0 (`alien-kind5.md` §5.2) with these differences:

1. **Shot down by Stanley**: unchanged (kinds 0 and 1 both eligible, `pet-stanley.md`; the `m.kind > 1` check in Stanley's code excludes only kind 2+).
2. **Steer and hit**: while not redirected (`+0x190 == 0`), same as kind 0 (steer toward target, hit test). Once redirected: no steering (the missile flies straight at its current velocity).
3. **Hit test** (`0x004fc070`, kind-1 path at `0x004fc8b6`): same box as kind 0, but **with a Wadsworth shelter check** (`alien-kind5.md` §6.2). If the target guppy is sheltered (Wadsworth active and stage < 2), the missile does not hit.
4. **Clamp**: while not redirected, same as kind 0 (`x ∈ [10, 550]`, `y ∈ [95, 370]`). **Once redirected, no clamping.**
5. **Out-of-bounds removal**: once redirected, if `x > 580 || x < −20 || y > 380 || y < 45` (the same bounds as the ball, `alien-kind5.md` §5.2 note), the missile is removed (§6.3).
6. **Move**: `x += vx / 0.8`, `y += vy / 0.8`; launch timer; widget resize. Unchanged.
7. **Smoke trail**: same as kind 0 (app `r % 3`; on 0: `r % 5`, `r % 5`, `EffectShot` kind 8).
8. **Heading cel**: same `0x004e2170` function as kind 0.

### Speed and behaviour summary

Before redirect: identical to kind 0. Steers toward its fish, clamped to the tank, up to ~2.25 px/tick.

After redirect: flies straight at its current velocity (up to ~2.5 px/tick), no clamping, exits the tank in a few seconds. The player can aim it away from their fish by choosing where to click on the missile.

## 6. Targets, steering, hits, explosions, clicks

### 6.1 Target choice and steering

`static`

Same as kind 5 (`alien-kind5.md` §6.1): `PickTarget` picks the farthest eligible fish; steering is the same table. No change for kind 6 or kind 1.

### 6.2 Hit test, kind 1

`static`

Same box as kind 0 (alien-kind5.md §6.2), but with the **Wadsworth shelter check** at `0x004fc8b6` (`pet-wadsworth.md`): if `0x005e89d0 ≠ 0` (Wadsworth is sheltering) and the target guppy's stage `+0x1a0 < 2`, the hit is blocked. The missile keeps flying past the fish. (Kind 0 has no such check: a sheltered guppy that was targeted before Wadsworth started is still hit by kind 0.)

On a hit (not sheltered), the same as kind 0: clear `+0x98`, kill the target fish (DIE, corpse), remove the missile (§6.3).

### 6.3 Removal and explosion, kind 1

`static`

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

`static`

- Sheet: the kind-6 image global (not yet identified; likely `IMAGE_KIND6` at an `0x005e8e..` slot). 10 × 2 layout, 160 × 160 cels, same as the Destructor's `IMAGE_DESTRUCTOR`.
- Mirror, row, cel: same rules as kind 5. Fire flash (additive white pulse when `0 < +0x1ac < 10`) and hit flash shared.
- Dead alien: `DeadAlien` draw maps kind 6 to the same sheet.

### 7.2 Missile, kind 1

`static`

Same as kind 0 (`alien-kind5.md` §7.2): `IMAGE_MISSILE`, heading cel, crosshair on the targeted fish. **Additionally**, kind 1 is drawn with an **additive glow** (`alien-kind5.md` §8): the missile image is drawn a second time additively, using the image at global `0x005e8a44`. This makes kind-1 missiles visually brighter (easier to spot and click).

### 7.3 Sounds

`static`

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

`static`

Same as kind 5 (`alien-kind5.md` §8): kind-1 missiles on `+0xdc` freeze the wave countdown, keep clicks as shots while the alien is dead, trigger Wadsworth's shelter, and keep the end-of-wave check waiting. Hunger and coins resume as soon as the alien dies (the `+0xb8` test, not `+0xdc`).

## 9. RNG summary

`static`

Same structure as kind 5 (`alien-kind5.md` §9), with:
- Spawn: 3 app draws (ctor 2, shadow 1). No global draws in Adventure.
- Per volley: 1 global draw per missile created (ctor `grand() % 4`), then `rand(50)` (interval).
- Each kind-1 missile update: same as kind 0 (smoke `r % 3`, effect-shot ctor).
- Kind-1 missile removal: `r % 3 + 2` sparks, each `r % 3`, `r % 30`, `r % 30`, shot ctor.
- Kind-1 redirect (click): no extra RNG (the shot-sound draws are from the board, not the missile).

## Deviations

`static`

Same as kind 5: PickTarget ties by heap address (sim uses list order, replay can override). Effect-shot start delay (heap-dependent, sim never draws it).

## Open questions

- **Volley size**: the tank3-plan says `PickTarget(0)`/`(1)` — two calls, so up to 2 missiles. Confirm via Ghidra at `0x004fdd34`.
- **Launch positions**: pet-stanley.md gives `(wx + 10, wy − 24)` for one call. The facing-right mirror at `(wx + 70, wy − 24)` comes from offset `0x46` in `tank3-plan.md`. Are both missiles from the same position? Confirm.
- **Firing interval**: assumed `rand(50) + 150` (same as kind 5). Confirm.
- **Redirect mechanic**: assumed velocity reversal (`vx = −vx, vy = −vy`). The actual `0x004d82a0` kind-1 branch may use the click position to set the direction. Confirm.
- **Redirect re-clickable**: assumed only one redirect per missile. Confirm.
- **Hit with Wadsworth**: assumed shelter blocks the hit (the missile keeps flying). Confirm whether the missile is removed instead.
- **Kind-6 image global**: which `IMAGE_*` constant. Likely at an `0x005e8e..` slot.
- **Sound `0x143`**: assumed as the volley sound from `tank3-plan.md`. May be UNLEASH (used by Shrapnel's bomb too).
- **Sound `0x11f`**: played before EXPLODE on kind-1 removal. Identify.

## Recording

A tank 3 level with kind-6 waves (from the wave table, once read). The capture must exercise:

1. The kind-6 spawn (ROAR2, warp, y = 280).
2. A volley of kind-1 missiles: the targets, the launch position(s), the sound.
3. Clicking a missile (redirect): the velocity change, the redirected flag, the missile leaving the tank.
4. A missile hitting a fish (DIE, corpse, the `0x11f` + EXPLODE).
5. Stanley shooting a kind-1 missile down.
6. The laser kill (37 hits at base laser), the diamond, the floor corpse.
7. Clicks after the alien dies while missiles fly.

`static` (a plan).
