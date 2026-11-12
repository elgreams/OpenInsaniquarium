# Alien kind 7 (`Sexy::Alien` kind 7)

Status: `traced` (`tank_3_3`). Most sections verified by the `tank_3_3` replay of `tank3-3_full_01`. Remaining `static` items: panic dive (health never dropped below 100 before first natural submerge in the recording), Virtual Tank health, exact knockback constants, and the 3-5 wave draw (covered by `tank_3_5`). Addresses are inner-PE VAs. Offsets are raw `this`-relative.

Kind 7 is a tank 3 alien that alternates between **surfaced** (fast, chases and eats fish) and **submerged** (slow, untargetable by pets, heals from laser hits). It shares the `Alien` class with all other aliens; **this spec only describes what differs for kind 7.** Anything not mentioned here behaves as in `alien-sylvester.md`. Notation is the same.

In short: kind 7 is **very fast when surfaced** (divisor 0.5, cruise ~2.0 px/tick, chase up to ~6 px/tick) and chases fish like Sylvester. Every 301–400 ticks it toggles between surfaced and submerged. **While submerged** it slows down (divisor 2.0), stops chasing, and **heals from laser hits** instead of taking damage ("Stop shooting! Alien regains health!"). Pets (Itchy, Gash, Stanley) skip it while submerged (`+0x204 ≠ 0`). A **panic dive** triggers if health drops below 100 before its first natural submerge.

## 1. Wave timing

`traced` (`tank_3_3`); 3-5 draw `static` (see `tank_3_5`).

Kind 7 first appears on tank 3. Per the level table at `0x00541200` (tank 3 branch): **3-3 sets kind 7**, and **3-5 uses `(r & 1) | 6`** (kind 6 or 7, 1 app draw). The countdown, re-roll formulas and pair-kind assignments are in `tank3-plan.md`.

Kind 7 does not fire missiles, so the countdown freeze rule that applies to kinds 5/6 (`+0xdc` non-empty) does not apply. The countdown freezes only while the alien is alive (`+0xb8` non-empty).

**Warning line** (countdown 275): "ALIEN SIGNATURE TYPE-F DETECTED" (expected from the sequence TYPE-A through TYPE-F; not yet confirmed).

**Pair kind** on 3-5: if 3-5 rolls kind 7, the pair kind is 10 (= 7 + 3: Sylvester plus kind 7). `static` from `alien-sylvester.md#per-tick-countdown`.

## 2. Spawn (`Board::AddAlien` @ `0x00545620`)

`traced` (`tank_3_3`)

The same steps and the same **3 app draws** as kind 1 (ctor 2, shadow 1). The arrival sample: for kind 7, the sound table maps to **`0x135` ROAR3** (`alien-kind5.md` §2, sound table `0x5e0eb0`).

### Constructor (`0x004ecfa0`), kind 7

`traced` (`tank_3_3`); Virtual Tank health `static`.

- **Divisor `+0x1b8` = 0.5** (`0x5c3c20`), **health `+0x1f0` = max `+0x1f8` = 260.0** (`0x5966f0`).
- **No y override**: unlike kinds 5/6, kind 7 does not force y to 280. It spawns at the warning's A.y and swims freely.
- Submerge fields: **timer `+0x194` = 200**, **threshold `+0x198` = 400**, **counter `+0x19c` = 0** (the "has submerged" flag), **state `+0x204` = 0** (surfaced).
- Everything else as kind 1: 160×160, eat delay 100, warp 15, `+0x1d4 = 40`, `+0x1d8 = 20`.
- RNG: the same 2 app draws (`rand()` direction, `rand(10)` wander mode).
- Virtual Tank only (`app+0x880`): health = `grand() % 100 + 150`, `+0x1a8 = 144`. Not reached in Adventure.

## 3. `Alien::Update` (`0x004fd560`), kind 7

`traced` (`tank_3_3`)

The same top-level steps as kind 1 (`alien-sylvester.md` §3), with these differences:

1. Virtual Tank fall: N/A (that branch is kinds 5/6 only).
2. Warp-in: unchanged.
3. **Chase or wander** (`0x004f03d0`): kind 7 **chases and eats fish** (the same `NearestPrey` + steering + eat path as kinds 1–3), **but only when surfaced** (`+0x204 == 0`). When submerged (`+0x204 ≠ 0`), the chase branch is skipped; it always wanders (§3.1).
4. Clamp: the standard `x ∈ [−10, 490]`, `y ∈ [85, 290]`. Kind 7 swims freely (not floor-locked like kinds 5/6).
5. **Submerge toggle** (§3.2).
6. No vx manipulation (that's kind 6 only).
7. **Move:** `x += vx / div + lunge`, `y += vy / div`. With div 0.5 (surfaced), the cruise speed is ~2.0 px/tick per axis; with div 2.0 (submerged), ~0.5 px/tick.
8. Timers: unchanged.
9. No firing (kinds 5/6 only).
10. Widget resize, **animation** (§3.3), death check.

### 3.1 Wander, kind 7

`traced` (`tank_3_3`)

The normal wander path (`alien-sylvester.md` §3.3), **not** the floor-walker wander of kinds 5/6:

- Modes 0/1 set target vx (−1.5 / +1.5); modes 2/3 set target vy (−1.5 / +1.5); modes 4–9 no change.
- Both vx and vy are steered toward their targets by ±0.1 per tick.
- Re-roll: `+0x1d8 += 1` when `+0x1ac == 0`; when > 20, reset and `rand(10)`, if 0 then `rand() & 3`.
- No wall bounce (presses against the clamp).

Cruise speed: surfaced (div 0.5) ≈ 1.5/0.5 = 3.0 px/tick; submerged (div 2.0) ≈ 1.5/2.0 = 0.75 px/tick.

### 3.2 Submerge toggle

`traced` (`tank_3_3`); panic dive `static` (health stayed above 100 before first natural submerge in the recording).

Each tick that reaches step 5 (after wander/chase, after clamp), if kind is 7 and not the Virtual Tank:

1. `+0x194 += 1`. If `+0x194 < +0x198`: stop.
2. When `+0x194 ≥ +0x198`:
   1. **Toggle `+0x204`**: if it was 0 (surfaced), set to 1 (submerged); if non-zero (submerged), set to 0 (surfaced).
   2. If **submerging** (0 → 1): set `+0x19c = 1` (the "has submerged" flag; stays 1 forever).
   3. **`+0x1ac = 10`** (transition animation, §3.3).
   4. **`+0x194 = rand() % 100`** (app; reset the timer to a random 0–99).
   5. If **surfacing** (1 → 0): **divisor `+0x1b8` = 0.5** (`0x5c3c20`), sample **`0x135` ROAR3** (gap 3), and `0x00537f90(−1)` (alien music cue).
   6. If **submerging** (0 → 1): **divisor `+0x1b8` = 2.0** (`0x5c3cc8`). No sound.

**Timing:** the ctor sets `+0x194 = 200` and `+0x198 = 400`. The **first submerge** happens **200 ticks** after spawn. After each transition, the timer resets to `rand() % 100`, so subsequent transitions come every **301–400 ticks** (threshold 400 − random start 0–99).

**Cycle:** surfaced → (200 ticks) → submerged → (301–400 ticks) → surfaced → (301–400) → submerged → …

### 3.3 Animation (`0x004daa00`), kind 7

`traced` (`tank_3_3`)

Three modes:

1. **Transition animation** (`+0x1ac > 0`): `+0x1ac` decrements by 1 each tick. While > 0:
   - **Submerging** (`+0x204 ≠ 0`): **cel = `10 − fire_anim`** (1, 2, …, 9 as fire_anim goes 9 → 1). A 9-tick dive sequence.
   - **Surfacing** (`+0x204 == 0`): **cel = `fire_anim`** (9, 8, …, 1). The dive in reverse.
   - Row 4 (§7.1).

2. **Turn** (`+0x1ac == 0`, turn counter `+0x1b4 ≠ 0`): cel = `9 − |turn|` (same as kinds 1–3, 10-tick turn). The turn counter is ±10, not ±20 (unlike kind 6).

3. **Normal swim** (`+0x1ac == 0`, turn = 0): the **slow loop** — `+0x1b0 += 1`, wrap at 20, **cel = `+0x1b0 / 2`** (10 cels, 0..9). Kinds 5, 7 and `0x15` skip the stroke branch, so the animation **never moves** kind 7 (no 0.25/0.75/0.5 pushes).

## 4. Clicks and damage

### 4.1 Input routing

`traced` (`tank_3_3`)

Same as kind 1 (`alien-sylvester.md` §4.1): laser path, `Alien::Hit` on each alien, shot sound (2 app draws). Kind 7 has no missiles.

### 4.2 `Alien::Hit(cx, cy)` (`0x004fa6f0`), kind 7

`traced` (`tank_3_3`); panic dive `static` (not triggered in the recording — see §3.2); exact knockback constants `static`.

Hit box and 10-tick cooldown: unchanged (160×160, `+0x200 > 0` → no effect).

#### Surfaced (`+0x204 == 0`)

`traced` (`tank_3_3`)

**Damage: `health −= 3 · laser`** (same as kinds 1–3). At laser 2 that is **6 per hit**, so **44 hits** to kill 260 HP. EffectShot and HIT sound (`0x126`, gap 3) as kind 1. Verified: the capture shows −6.0 HP drops every ~10 ticks while surfaced.

**Panic dive** (`0x004fa997`): `static` — after applying damage, if **all** of:
- kind is 7,
- `health < 100.0` (`0x5c3bd0`),
- `+0x19c == 0` (has never submerged before),
- not the Virtual Tank,

then: **force submerge** — `+0x204 = 1`, `+0x19c = 1`, `+0x1ac = 10`, `+0x194 = rand() % 100` (app). The alien dives immediately.

This is a **one-time emergency dive**: once `+0x19c = 1` (either from the first natural submerge or from the panic dive), it never triggers again. At the base laser (6 per hit, 10-tick cooldown), the player can deal at most 20 × 6 = 120 damage in the first 200 ticks, bringing health from 260 to 140 — **not below 100**. The panic dive only matters at higher laser levels.

**9-zone knockback** (`0x004fab6e`): same structure as kinds 1–3 (`alien-sylvester.md` §4.2), with zone boundaries at `x+60 / x+100` and `y+60 / y+100`. The impulse magnitudes are **div-scaled** (`vx = div · k`), and since the move divides by div, the actual displacement per tick is `k` regardless of the current divisor. The constants `k` for kind 7 are **larger** than for kinds 1–3 (approximately 3.5 for diagonals and ±3.0/±2.5 for axes, vs 2.5/3.0 for kinds 1–3). The exact per-zone constants are at `0x597560`, `0x599180`, `0x596588`, `0x5965b0`, `0x596598`; `static`, full verification needs a focused knockback recording.

#### Submerged (`+0x204 ≠ 0`)

`traced` (`tank_3_3`)

**Healing** (`0x004fa7e0`): if no boss (`board+0x110 == 0`), **`health += 3 · laser`**. At laser 2 that is **+6 HP per hit**. No EffectShot, no HIT sound, no hit flash (the draw path skips it when `+0x204 ≠ 0`). Verified: the capture shows 32 heal events of +6.0 each while submerged.

The board displays **"Stop shooting! Alien regains health!"** via the message line (`0x0053e950`; green text, one-time per message slot). The hit **cooldown `+0x200 = 10`** is still set (no rapid healing), and the **9-zone knockback** still runs (the alien is pushed despite gaining health).

### 4.3 Pets and other damage sources

`traced` (`tank_3_3`); Rufus targeting `static`.

**Correction:** kind 7 does **not** use the reduced-damage table of kinds 4/5. The capture shows Itchy dealing **1.0 HP/tick** (46 hits of exactly −1.0 while surfaced), matching the default `STAB` constant. Gash deals **3.0** and Stanley's ball **15** (full values). The reduced table (`Itchy 0.25, Gash 0.5`) applies **only to kinds 4 and 5** (Gus and Destructor), per `0x004fb5b0`.

Pets skip kind 7 when submerged (`+0x204 ≠ 0`): Itchy, Gash and Stanley confirm this (`pet-itchy.md`, `pet-gash.md`, `pet-stanley.md`).

Whether Rufus also skips it is not checked (`static`). Rufus targets the nearest alien; if he checks `+0x204` he'd skip submerged kind 7 too.

### 4.4 Death (`0x004f9c70(1)`), kind 7

`traced` (`tank_3_3`)

The **generic branch** (same as kinds 1–3 and 5–6): diamond, kind-1 debris, `AddDeadAlien`. The corpse uses `IMAGE_KIND7` (§7.1). Unlike kinds 5/6, there is **no y ≤ 280 clamp** on the dead alien: the corpse sinks out of the tank normally.

## 5. Eating

`traced` (`tank_3_3`)

When surfaced, kind 7 chases and eats fish using the same path as kinds 1–3 (`alien-sylvester.md` §3.2): `NearestPrey` from the master set `+0x104`, steering by ±0.1/tick toward the target, eat test with 45/65 half-extents (70/85 for Ultravores), one fish per tick. Eat delay starts at 100 from the ctor.

With divisor 0.5, the chase is extremely fast: steering caps at ±1.8, so the move is up to `1.8 / 0.5 = 3.6 px/tick` per axis. The initial ±3.0 vx from the ctor translates to `3.0 / 0.5 = 6.0 px/tick`, decaying toward the ±1.8 cruise as the alien steers.

When submerged, the chase is completely skipped — the alien wanders harmlessly.

## 6. Effects on the board

`traced` (`tank_3_3`)

Same as kind 1 (`alien-sylvester.md` §5): while alive, guppies don't drop coins, hunger runs, Wadsworth shelters. Kind 7 has no missiles, so `+0xdc` is never populated by it.

The submerge/surface cycle means the alien's presence (`+0xb8` non-empty) blocks the wave countdown and keeps the coin lockout for the entire duration, including submerged phases. Hunger is gated by `+0xb8` too, so guppies can still starve during kind 7's submerge phase.

## 7. Drawing and sound

### 7.1 Alien, kind 7

`traced` (`tank_3_3`)

- **Sheet: `IMAGE_KIND7`** (global `DAT_005e8c4c`, from `Alien::DrawSprite` @ `0x004dae20`). 10 columns, **5 rows**, 160 × 160 cels.
- **Row selection** (from `DrawSprite`):

  | State | Row |
  |---|---|
  | Surfaced, normal swim | 0 |
  | Surfaced, turning | 1 |
  | Submerged, normal | 2 |
  | Submerged, turning | 3 |
  | Transition animation (`+0x1ac > 0`) | 4 |

- **Cel**: from the animation function (§3.3). Column = cel `+0x1c0` (0..9).
- **Mirror** by vx (facing direction), same as kind 1.
- **Hit flash**: white additive (`alpha = 25 · +0x200`), but **skipped when submerged** (`+0x204 ≠ 0`). Even though the cooldown is set after a heal, no flash shows.
- **No fire flash** (that is kind 5 only in `DrawSprite`).
- **Dead alien**: `DeadAlien` draw maps kind 7 to `IMAGE_KIND7`, row 0.
- Warp-in scaling, shadow, health bar (Blip): as kind 1.

### 7.2 Sounds

`traced` (`tank_3_3`)

| Event | Sample | Notes |
|---|---|---|
| Spawn | `0x135` ROAR3 | direct, unthrottled |
| Surface (each cycle) | `0x135` ROAR3 | gap 3; plus alien music cue `0x00537f90(−1)` |
| Submerge | — | no sound |
| Laser hit (surfaced) | `0x126` HIT | gap 3 |
| Laser hit (submerged) | — | no HIT sound; "Stop shooting!" message |
| Click | `0x144` ZAP (+`0x145`, +`0x132`) | the usual shot sound, 2 app draws |
| Eat | `0x113`/`0x114` | gap 3, `rand(3)` pick; same as kind 1 |
| Death | `0x11d`, `0x11e` | the same as kind 1 |

## 8. RNG summary

`traced` (`tank_3_3`)

| When | Stream | Draws |
|---|---|---|
| Spawn | app | ctor `rand()`, `rand(10)`; shadow `rand(5)` |
| Wander re-roll (timer > 20, `+0x1ac == 0`) | app | `rand(10)`; if 0: `rand() & 3` |
| Submerge transition | app | `rand() % 100` (timer reset) |
| Panic dive (in Hit) | app | `rand() % 100` (timer reset) |
| Eat | app | `rand(3)` (sound pick); gore: `rand(3)` count, then per piece `rand(20)` × 2 |
| Player click | app | shot sound: `rand() & 1`, `rand(10)` |
| Death | app | kind-1 debris (`alien-sylvester.md` §4.3) |

**Order within a tick:** same as kinds 1–3. The submerge toggle happens after the wander/chase step and after the clamp, before the move.

## Deviations

`traced` (`tank_3_3`)

- **NearestPrey ties by heap address** (`alien-sylvester.md` §3.1): the sim uses list order.
- **Effect-shot start delay** (heap-dependent): same as kinds 1–3.

## Open questions

- **Warning line text**: "TYPE-F" is expected from the A–F sequence but not confirmed.
- **Knockback constants**: the 9-zone impulse magnitudes are div-scaled; the exact per-zone constants need a focused recording to verify.
- **Rufus vs submerged kind 7**: whether Rufus checks `+0x204` before targeting.
- **Panic dive**: not triggered in the recording (health never dropped below 100 before the first natural submerge). Requires a higher laser level or additional pet damage to exercise.
- **Music cue** (`0x00537f90(−1)`): what it does (likely resumes alien music).

## Recording

`traced` (`tank_3_3`)

Recorded as `tank3-3_full_01` (6001 snapshots, 3 pets: 2 OtherType + 1 FishType). Exercised:

1. Spawn at tick 2031 (div=0.5, hp=260, 3 draws).
2. Surfaced chase: alien ate 2 guppies (3 → 1 between ticks 2031–2236).
3. First submerge at tick 2236 (~205 ticks after spawn): div → 2.0, state → 1, fire_anim=9.
4. Laser healing while submerged: 32 events of +6.0 (= 3 × laser 2), hp reached 306.
5. Surfacing at tick 2558: div → 0.5, state → 0, fire_anim=9, ROAR3.
6. Surfaced laser damage: −6.0 per hit (10-tick cooldown). Itchy: −1.0/tick (full damage, not reduced).
7. Alien killed at tick 2723–2724 (death debris, diamond).
