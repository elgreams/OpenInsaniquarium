# Alien kind 5: the Destructor, and its missiles (`Sexy::Alien` kind 5, `Sexy::Missle` kind 0)

Status: `draft (static)`. Nothing here has been captured yet. Every claim comes from reading the binary, and every section is `static` until a 2-4 recording replays (see "Recording" at the end). Addresses are inner-PE VAs. Offsets are raw `this`-relative, the same as in `alien-sylvester.md` (alien) and `pet-stanley.md#the-ball-missle-kind-2` (missile).

Kind 5 is the tank 2-4 alien. Its warning line is "ALIEN SIGNATURE TYPE-D DETECTED", and its sprite sheet is `IMAGE_DESTRUCTOR` (§7.1), so this spec calls it **the Destructor**. It is the same `Alien` class as Sylvester, with the same update (`0x004fd560`), ctor (`0x004ecfa0`), hit (`0x004fa6f0`) and death (`0x004f9c70`). **This spec only describes what differs for kind 5.** Anything not mentioned here behaves as in `alien-sylvester.md`. Notation (`rand(n)` = app stream `r % n`, `grand()` = global stream, `ix, iy` = widget position, tick) is the same as there.

In short: the Destructor **walks along the bottom** of the tank (its y is fixed at 280) and **never chases or eats fish**. Every 151 to 200 ticks it **fires a volley of up to three homing missiles**, each at a different fish, picking the **farthest** fish first. A missile that reaches its fish **kills it** (a normal death with a corpse) and explodes. The player can **shoot missiles down by clicking them** once they are 15 ticks old, and Stanley shoots them down too. The laser does `2 · laser + 2` per hit (6 at the base laser, so 25 hits for its 150 HP), and knockback only moves it sideways. It dies like Sylvester (diamond, debris, a corpse that settles on the floor). Its missiles outlive it and keep the wave from ending until they are gone.

Confidence: `static` for the whole file (per section below).

## 1. Wave timing

Differences from `alien-sylvester.md` §1 (and `alien-kind4.md` §1, which covers 2-5):

- **Level table** (`0x00541200`, tank 2 branch, `board+0x3cc` switch, case 4): **2-4 sets kind `+0x2bc = 5`** and `+0x340 = 7500`. No RNG. It doesn't write `+0x2c0`, so the countdown keeps the level-start value **3000** (`0x00541a20`). **2-5** sets `kind = (rand() & 1) | 4`, kind 4 or 5 (1 app draw at level setup), and re-rolls after every spawn (2 or 3 app draws; `alien-kind4.md` §1).
- **Re-roll after a spawn:** none on 2-4 (only level 5 re-rolls on tank 2). 2-4 keeps kind 5 and draws nothing.
- **Warning line** (countdown 275): "ALIEN SIGNATURE TYPE-D DETECTED" (`sim/board.md#the-alien-warning-line`). Colours, timer, sound `0x106`, music and the **4 app position draws** are the same as kind 1. A.y is drawn as usual but the ctor then forces y = 280 (§2).
- **Hint dialog:** none. The countdown-276 dialogs at `0x00548407`..`0x005485f6` are for 1-1/1-2 and 2-3 only (`0x00537bb0(2, 3)` is level 2-3).
- **Music:** the same as kind 1.
- **Countdown freeze:** the wave block is skipped while `+0xb8`, `+0xf4` **or `+0xdc`** is non-empty (`alien-sylvester.md#per-tick-countdown`, step 2). The Destructor's missiles live on `+0xdc` (§8), so **the next countdown doesn't start until the last missile is gone**, even after the alien has died.

`static`.

## 2. Spawn (`Board::AddAlien` @ `0x00545620`)

The same steps and the same **3 app draws** as kind 1 (ctor 2, shadow 1). The only difference is the arrival sample: for kind 5 the code at `0x0054589e` loads global `0x005e8bc8` (sound-table slot `0x5e0eac`, sample **`0x127` `SOUND_INTERFER`**) and plays it directly through the app's sound manager (vtable `+0xd8`), unthrottled, like `ROAR` for kinds 1–3. (The slot-to-sample mapping is linear: `0x5e0e9c` = `0x123` GUFFAW, `0x5e0edc` = `0x133` ROAR, so `0x5e0eac` = `0x127`. Kind 6 uses `0x134` ROAR2, kind 7 `0x135` ROAR3.)

`static`.

### Constructor (`0x004ecfa0`), kind 5

- **Divisor `+0x1b8` = 1.2** (`0x596eb8`), **health `+0x1f0` = max `+0x1f8` = 150.0** (`0x5967e8`).
- **y forced to 280** (`0x597550`): outside mode 5 (`app+0x87c ≠ 5`, i.e. not the Virtual Tank) the ctor sets the widget y `+0x34 = 0x118` and the double y `+0x168 = 280.0`, overriding the spawn A.y. (Kind 6 shares this branch.)
- Shooting fields (shared defaults): fire timer `+0x1a4 = 0`, fire interval `+0x1a8 = 75`, fire animation `+0x1ac = 0`.
- Everything else is the same as kind 1: 160×160, eat delay `+0x1a0 = 100`, `+0x200 = 0`, warp `+0x158 = 15`, `+0x1d4 = 40`, `+0x1d8 = 20`, vy 0.
- RNG: the same 2 app draws in the same order (`rand()` direction → vx ±3.0, then `rand(10)` wander mode).
- Virtual Tank only (`app+0x880`): health = `grand() % 100 + 150`, `+0x1a8 = 144`. Not reached in Adventure.

`static`.

## 3. `Alien::Update` (`0x004fd560`), kind 5

The top-level steps of `alien-sylvester.md` §3, with these differences:

1. (Virtual Tank only, `app+0x87c == 5`, kinds 5/6: while y < 280, `vy += 0.4 · div`; at y ≥ 280, y = 280 and vy = 0. A fall to the floor. Not reached in Adventure, where the ctor already put it at 280.)
2. Warp-in: unchanged.
3. **Chase or wander** (`0x004f03d0`): for kinds 5 and 6 the prey branch is skipped, so `NearestPrey`, steering and the eat test **never run**. It always returns "wandering" (§3.1). **The Destructor never eats a fish.** No chomp sound, no gore, and `board+0x488` (fish lost) only grows through its missiles' kills.
4. Clamp `x ∈ [−10, 490]`, `y ∈ [85, 290]`: unchanged (y stays 280).
5. Move: `x += vx / 1.2 + lunge`, `y += vy / 1.2`. vy is 0 for good in Adventure (nothing in kind 5's path writes it; the laser knockback only sets vx, §4.2). So y stays exactly 280.0.
6. Timers: unchanged (`+0x200`, `+0x1a0`).
7. **Firing** (§3.2), kinds 5 only.
8. Widget resize, **animation** (§3.3), death check: unchanged order. Health ≤ 0 → `0x004f9c70(1)` (§4.3).

`static`.

### 3.1 Wander, kind 5

The kind-1 wander (`alien-sylvester.md` §3.3) with three changes:

- **Targets:** wander mode `+0x1d0` 0 or 2 → target vx `+0x180 = −1.0` (`0x5c3d80`); 1 or 3 → **+1.0**; 4..9 → no change. Modes 2/3 don't set a target vy, and **vy isn't steered at all** (the "vy toward target vy" step is in the non-5/6 branch).
- vx moves toward target vx by 0.1 with the same two sequential ifs (so the same ±0.1 dither). With div 1.2 the cruise speed is 1/1.2 ≈ 0.83 px/tick. The target starts at 0 (ctor), so until a mode 0–3 is rolled the Destructor slows from its ±3.0 entry speed to a standstill.
- **Re-roll gate:** `+0x1d4 += 1`, then the re-roll timer `+0x1d8` only advances while the fire animation `+0x1ac == 0`. When it exceeds 20: reset, **`rand(10)`**, and if 0 **`rand() & 3`** (app). So the 10-tick firing animation postpones the next re-roll by 10 ticks.

There is no wall bounce. It presses against x = −10 or 490 until the mode changes.

`static`.

### 3.2 Firing (`0x004fdb06`..`0x004fdd2f`)

Each tick that reaches step 7, if the kind is 5 **and "has prey"** (`0x004d4580`: guppies, Oscars, Starcatchers or some pets exist, or a boss):

1. `+0x1a4 += 1`. If `+0x1a4 ≤ +0x1a8`, stop.
2. `fired = false`. The launch points depend on the facing (`vx ≥ 0`, double compare against 0):

   | | Missile 1 | Missile 2 | Missile 3 |
   |---|---|---|---|
   | vx ≥ 0 | `(ix + 90, iy − 35)` | `(ix + 105, iy − 30)` | `(ix + 120, iy − 25)` |
   | vx < 0 | `(ix − 10, iy − 35)` | `(ix − 25, iy − 30)` | `(ix − 40, iy − 25)` |

   These are the missile's top-left (its widget is 80×80). `ix, iy` are the widget position after this tick's move but before the resize, i.e. last tick's truncated position.
3. `t = PickTarget()` (§6.1). If there is one:
   1. `AddMissile(p1, t, kind 0)` (`0x00544130`, §5), `fired = true`;
   2. sample **`0x128` `SOUND_MISSLE`**, gap 3 (once per volley);
   3. unless the Virtual Tank (`app+0x880`): `t = PickTarget()`; if found, `AddMissile(p2, t, 0)`; then `t = PickTarget()`; if found, `AddMissile(p3, t, 0)`.

   Each `AddMissile` sets the target's `+0x98`, and `PickTarget` skips objects whose `+0x98` is set, so **the three missiles go to three different fish**, and a fish that already has a missile on it from an earlier volley can't be picked. If the first pick finds nothing, nothing is fired and no sound plays. If the second finds nothing, the third isn't tried.
4. Always (fired or not): `+0x1a4 = 0`, **interval `+0x1a8 = rand(50) + 150`** (app; Virtual Tank: `+ 360`).
5. If `fired`: **fire animation `+0x1ac = 10`** (§3.3).

**Rhythm:** the timer counts only on ticks that get past the warp-in freeze and have prey. The first volley comes on the **76th** such tick (interval 75 from the ctor; about 82 ticks after the spawn counting the 6 frozen ticks). After that, each volley comes `interval + 1` = **151 to 200** counted ticks after the previous one. With no guppies (or other prey) the timer stops; it doesn't reset.

**Within-tick order:** the alien's update runs (in z-order after the guppies), creates up to 3 missiles, and draws the interval. Each new missile is brought to the front of the widget list by `AddMissile`, so by `sim/board.md#tick-order` (objects created during the object pass update in it) it **probably runs its first update later in the same tick**, after every other object, and its launch timer reads 14 on the first snapshot (see Open questions). Corpses made in an update are known to behave this way (`sim/board.md`); missiles are not checked yet.

`static`.

### 3.3 Animation (`0x004daa00`), kind 5

- Turn detection and the turn counter (±10 → 0): unchanged.
- **Fire animation** `+0x1ac`: if > 0, decrement it; if it reaches 0, reset the animation counter `+0x1b0 = 0`. While it is still > 0 after the decrement, **cel = `+0x1ac`**: the 10 set in §3.2 shows as cels 9, 8, …, 1 on the next 9 draws (the decrement happens in the same update that set it). This takes priority over turning.
- Otherwise, while turning: the kind-1 turn cels (`9 − turn` / `turn + 10`, row 1).
- Otherwise: **always the slow loop**: `+0x1b0 += 1`, wrap at 20, `cel = +0x1b0 / 2`. Kinds 5, 7 and 0x15 skip the |vx| > 1.6 "stroke" branch, so **the animation never moves the Destructor** (no 0.25/0.75/0.5 pushes, even at the ±3.0 entry speed or after knockback).
- Last-vx update and the attached-ball counter (`+0x98`, Stanley): unchanged.

`static`.

## 4. Clicks and damage

### 4.1 Input routing

Kind 5 takes the **normal laser path** of the Board mouse-down (`alien-sylvester.md` §4.1): click y > 40 → `Alien::Hit` on each alien in list order (stop at the first that dies) → **walk the missiles `+0xdc` with `0x004d82a0`** (stop at the first destroyed, §6.4) → click shot `(x − 40, y − 40)` → `+0x4ed = 1` → shot sound (2 app draws). A click can hit the alien **and** destroy one missile.

**After the alien has died** while missiles are still in flight (`+0xb8` and `+0xf4` empty, no boss, `+0xdc` non-empty), the branch at `0x00543afd` still takes the shot path (`0x00543b9a`): click y > 40 → missile walk → click shot → `+0x4ed = 1`, `+0x3c4` → shot sound (2 app draws). **No food can be dropped until the last missile is gone**, and every click costs the two shot-sound draws.

**Coin under a missile:** `Coin` mouse-down re-dispatches a click inside any `+0xdc` widget to the board handler (`alien-sylvester.md` §4.1), so clicking a coin that overlaps a missile is also a shot (and the coin is collected).

`static`.

### 4.2 `Alien::Hit(cx, cy)` (`0x004fa6f0`), kind 5

1. Hit box and the 10-tick cooldown: unchanged (`x < cx < x + 160`, `y < cy < y + 160`, strict; no effect while `+0x200 > 0`).
2. **Damage: `health −= 2 · laser + 2`** (`0x004fa97c`, kinds 5 and 6; kind 1 does `3 · laser`). Laser = `board+0x3e4`. At the base laser 2 that is **6, the same as kind 1**, so **25 hits** kill a fresh Destructor. Each upgrade adds 2 instead of 3 (laser 12: 26 per hit, 6 hits).
3. `EffectShot(cx − 40, cy − 40, 2)` and sample `0x126` `HIT` (gap 3): unchanged.
4. **Knockback, kinds 5/6** (relative to the alien's double x; s = div = 1.2):
   - if the fire animation `+0x1ac ≠ 0`: **no knockback** at all;
   - `cx < x + 60`: **vx = +3.0 · s = 3.6** (pushed right);
   - `cx > x + 100`: **vx = −3.6** (pushed left);
   - otherwise (the middle 40 px, both edges inclusive): none.

   **vy is never changed**, whatever the click height. The move divides by 1.2 again, so a push is 3 px/tick, decaying by 0.1 per tick toward the wander target. There is no stroke animation (§3.3).
5. Death or the 10-tick cooldown: unchanged.

`static`.

### 4.3 Pets and other damage sources

| Source | Normal damage | Kind 5 |
|---|---|---|
| Itchy bite (`0x004fb5b0`, pet 2) | 1.0 | **0.25** (`0x596818`, shared with kind 4) |
| Gash bite (`0x004fb5b0`, pet 0x11) | 3.0 | **0.5** (`0x5c3c20`, shared with kind 4) |
| Rufus (`0x004e7c30`, `0x004e7f8e`) | 2.0 | **0.25** |
| Stanley's ball (`pet-stanley.md`) | 15 | 15 (no kind check; 10 balls kill) |

From `alien-kind4.md` §4.2; not re-checked here. All of them kill through `0x004f9c70(1)` (§4.4).

`static`.

### 4.4 Death (`0x004f9c70(1)`), kind 5

Kind 5 takes the **generic branch** (only kinds 4 and 0x14 have their own), so it is identical to kind 1 (`alien-sylvester.md` §4.3):

1. Release the attached missile `+0x98`, which is only ever Stanley's ball (the Destructor's own missiles hang off their targets, not off the alien). **Its own missiles are not removed**: they keep flying, can still kill fish, can still be shot, and keep the end-of-wave work waiting (§8).
2. Diamond `AddCoin(ix + 25, iy + 25, kind 4)` (no boss, normal game).
3. Remove the alien and its shadow; `0x00539c90` (only acts if no aliens, Bilaterus **or missiles** remain: lockout `+0x2c8 = 36`, tank music, guppy byte `+0x55`).
4. Debris (app, kind-1 counts): `rand() & 1` × 2 bubbles, `rand(3)` × 2 sparks of size 3–5, `rand() & 1` × 2 sparks of size 6–7; sounds `0x11d`, `0x11e`.
5. **`AddDeadAlien(trunc x, trunc y, cel, 5, vx > 0)`**, with the 10 corpse bursts of `DeadAlien::Update` (`alien-sylvester.md` §4.4).

**The corpse lies on the floor.** In `DeadAlien::Update` (`0x004f6050`) kinds 5 and 6 clamp y to **≤ 280** before the move (`0x597550`; the second bound 310 can't trigger), then `y += vy`. So the corpse starts at 280, and each tick is reset to 280 and moved down by its current vy (≤ 2.0): it hovers at 280 + vy instead of sinking out of the tank. Its draw uses image `0xc0` (`IMAGE_DESTRUCTOR`, §7.1).

`static`.

## 5. The missile: `Sexy::Missle` kind 0

`AddMissile(x, y, target, kind)` (`0x00544130`) is described in `pet-stanley.md#the-ball-missle-kind-2`: allocate `0x1b8` bytes, ctor `0x004eae40`, `Board::AddGameObject` → list **`+0xdc`** (kinds other than 3–5), add to the widget manager, bring it to the front (then `board+0x100` to the front), and **`target+0x98 = missile`**. Type code `0x1e`, vtable `0x00596c1c`.

`static`.

### 5.1 Constructor (`0x004eae40`), kind 0

Differences from the kind-2 ball table in `pet-stanley.md`:

| Offset | Meaning | Kind 0 |
|---|---|---|
| `+0x158`, `+0x160` | x, y (doubles); widget = trunc | the launch point (§3.2) |
| `+0x168`, `+0x170` | vx, vy | 0, 0 |
| `+0x17c` | launch timer | 15 |
| `+0x188` | speed divisor | **0.8** (`0x5c3d18`; the ball's is 0.4) |
| `+0x190` | byte (kind 1: "redirected"; kind 2: "bounced") | 0, never set for kind 0 |
| `+0x194` | cel | 1 (overwritten by the first update, §7.2) |
| `+0x198` | (kinds 3–5 only) | 1 |
| `+0x19c` | kind | 0 |
| `+0x1a0` | row: **`grand() % 4`** | drawn, unused by kind 0's own draw; it colours Stanley's shoot-down ball |
| `+0x1a4` | target | the fish |
| `+0x1a8`, `+0x1ac` | shoot-down progress / duration | 0, 10 |
| widget w/h | | **80 × 80** |

RNG: **1 global draw** (`Sexy::Rand`, the row). The two app draws `r % 6` (a random initial vx and vy from a 6-value table) run only when `0x004d80c0` is true, i.e. **kinds 3, 4, 5** (the `+0xe0` missiles). **Kind 0 doesn't take them.** The ctor clears the widget byte at `+0x55` as for the ball, so the widget manager doesn't route clicks to it; clicks reach it only through the board walk (§6.4).

`static`.

### 5.2 `Missle::Update` (`0x004ffb30`), kind 0

Each tick, return if there is no board or it is paused; then the `GameObject` pre-update `0x004f22c0`, and:

1. **Shot down by Stanley:** while `+0x1a8 > 0`: `+0x1a8 += 1`; when it reaches `+0x1ac`, remove the missile (§6.3) and return (`pet-stanley.md#shooting-down-missiles-0x004e55d0`).
2. **Steer and hit** (`0x004ff4e0`):
   1. If the target is null **or the target's `+0x98` is 0**: remove the missile (§6.3, with the explosion) and return. (Defensive: a target that leaves the tank removes its missile first, §6.2.)
   2. Steer toward the target (§6.1 table). Kind 0 always steers; there is no bounced/redirected state.
   3. Hit test (`0x004fc070`, §6.2). On a hit the missile is gone; return.
3. **Clamp** (kind 0 only, before the move): `x ∈ [10, 550]` (`0x5c3cd8`, `0x598f78`), `y ∈ [95, 370]` (`0x596728`, `0x596ed0`). There is **no out-of-bounds removal** for kind 0 (that rule is for kinds 1-after-redirect, 2 and 3–5) and **no lifetime**: a kind-0 missile lives until it hits, is shot down, or its target goes away.
4. **Move:** `x += vx / 0.8`, `y += vy / 0.8`; launch timer `+0x17c −= 1` if > 0; widget resize to `(trunc x, trunc y)`.
5. **Smoke trail** (kind 0 only): app draw **`r % 3`**; if 0: **`a = r % 5`**, then **`b = r % 5`**, and `EffectShot(ix − 20 · trunc(vx) + 17 + a, iy − 16 · trunc(vy) + 17 + b, 8)` (`0x005436f0`, effect kind 8 = `IMAGE_SMOKEMISSILE`, `alien-sylvester.md` §7.3). `ix, iy` are the just-resized widget position; `trunc` is toward zero. The shot is only created below the 20-shot cap of `+0xd8`, and its ctor costs 1 or 2 more app draws (heap-dependent, `alien-sylvester.md#deviations`).
6. **Heading cel** (`0x004e2170`, §7.2).
7. Tail call through vtable `+0x44` (common widget post-update, no gameplay).

Speed: vx and vy each step by 0.2 up to about ±1.8–2.0 (§6.1), so through the 0.8 divisor a missile flies at up to **2.25–2.5 px/tick per axis**, faster than the Destructor walks and than a guppy swims at cruise.

`static`.

## 6. Targets, steering, hits, explosions, clicks

### 6.1 Target choice (`0x004e8df0`) and steering (`0x004ff4e0`)

**PickTarget** (`0x004e8df0`, `this` = the alien; the pushed argument 1 or 0 is ignored, `RET 4` with no read of it). It clears the alien's byte `+0x190`, then walks the board's **master set `+0x104`** (a `std::set`, so **pointer-address order**, as `NearestPrey`). An object is a candidate if:

- `+0xac < 0` and `+0x120 < 1` (the same filter as `NearestPrey`), **and `+0x98 == 0`** (no missile already on it);
- by type `+0x8c`: `0` guppy unless protected (`0x00500ff0`: Wadsworth's shelter count `0x005e89d0` ≠ 0 and the guppy's stage `+0x1a0 < 2`; `pet-wadsworth.md`); `5..10` (Oscar, Ultravore, Gekko, Starcatcher, Grubber, Breeder) always; `0x14/0x15` pets only with a boss; anything else no.

Score: `d² = (acx − (o.ix + o.w/2))² + (acy − (o.iy + o.h/2))²` (ints), with `(acx, acy)` the **alien's widget centre** `(ix + 80, iy + 80)`. It keeps the candidate with the **largest** d² (`best < d²`, best starting at 0; ties keep the earlier one in address order). So **each missile goes to the farthest eligible fish** from the Destructor, then the next farthest, and so on. No RNG.

**Steering** (`0x004ff4e0`), kind 0, every tick. `c = 40` (the missile's centre offset), `p = x + c`, `q = y + c`, `(tx, ty)` = the target's widget position (ints converted to double), first matching rule:

| X condition | vx change |
|---|---|
| `p > tx + 80` | −0.2 if vx > −1.8 |
| `p < tx` | +0.2 if vx < 1.8 |
| `p > tx + 60` | −0.2 if vx > −1.4 |
| `p < tx + 20` | +0.2 if vx < 1.4 |
| `p < tx + 40` | +0.2 if vx < 0.6 |
| `p == tx + 40` | none |
| else (`tx + 40 < p ≤ tx + 60`) | −0.2 if vx > −0.6 |

| Y condition | vy change |
|---|---|
| `q > ty + 80` | −0.2 if vy > −1.8 |
| `q < ty` | +0.2 if vy < 1.8 |
| `q > ty + 60` | −0.2 if vy > −1.4 |
| `q < ty + 20` | +0.2 if vy < 1.4 |
| `q < ty + 40` | **+0.1 if vy < 0.2** |
| `q == ty + 40` | none |
| else (`ty + 40 < q ≤ ty + 60`) | **−0.1 if vy > −0.2** |

(Doubles. Constants 0.2 `0x596580`, 0.1 `0x5c3cd0`, ±1.8 `0x596780/0x597570`, ±1.4 `0x5991d8/0x5991e0`, ±0.6 `0x5965c8/0x5991d0`, −0.2 `0x5991c8`.) The aim point puts the missile's centre on `(tx + 40, ty + 40)`, the centre of an 80×80 fish.

**Ultravore targets** (type 6, 160×160) use the wide bands of the ball (`pet-stanley.md`): `tx + 200`, `tx − 40`, `tx + 110`, `tx + 50`, `tx + 80` (and the same for y), still with `c = 40`, so the centre goes to `tx + 120`.

`static`.

### 6.2 Hit test (`0x004fc070`, kind-0 path at `0x004fd44d`) and what a hit does

With `p = x + 40`, `q = y + 40` (doubles) and the target's widget `(tx, ty)`:

- normal target: hit if `tx + 10 < p < tx + 70` and `ty + 10 < q < ty + 70` (strict);
- Ultravore (type 6): `tx + 8 < p < tx + 140` and `ty + 4 < q < ty + 140`.

Only the missile's own target is tested; other fish in the way are ignored. **There is no Wadsworth-shelter check on this path** (the check at `0x004fc8b6` that `pet-wadsworth.md` cites belongs to the kind-1 path): a guppy that was targeted before Wadsworth started sheltering is still hit. Shelter only stops new targeting (§6.1).

On a hit, the missile calls its own vtable `+0x134` (`0x004f93a0`, Ghidra's `vfunction78`):

1. Clear the target's `+0x98`.
2. **Kill the target** by its type, with argument 1 (the "real death" flag):
   - 0 guppy, 5 Oscar, 6 Ultravore, 7 Gekko: `Fish::Die` (vtable `+0x160`, `0x004d5f90` for the base class): sample `0x117` `DIE` (gap 3), a **`DeadFish` corpse**, then removal (list, widget manager, master set, shadow), `board+0x488 += 1`. No RNG (`fish-guppy.md#dead-fish`, `carnivore.md`).
   - 8 Starcatcher: `0x004f3c60(1)`; 9 Grubber: `0x004f2890(1)`; 10 Breeder: `0x004dd5d0(1)` (their own deaths, no RNG; `pet-shrapnel.md` lists the same routines).
   - `0x14/0x15` pets (boss only): their slot 76 (`0x004e0e70`), removal.
3. Remove the missile (§6.3): explosion and debris.

So **a missile is an instant kill**, whatever the fish's size or hunger. Coins the fish would have dropped are lost; the corpse is the usual `DeadFish`.

`static`.

### 6.3 Removal and explosion (slot 76, `0x004d80e0`), kind 0

1. If it has a target: clear the target's `+0x98`, clear `+0x1a4`.
2. Leave the widget manager, the master set and the list (`0x00541d90`).
3. `0x00539c90` (end-of-wave check; it only acts when no aliens, Bilaterus or missiles remain).
4. Sample **`0x11d` `EXPLODE`**, gap 3 (kind 1 also plays `0x11f` first).
5. Debris, app stream: **`n = r % 3 + 2`**, then n times **`size = r % 3 + 3`**, **`y = wy + r % 30 − 10`**, **`x = wx + r % 30 − 10`**, `EffectShot(x, y, size)` (+1–2 ctor draws each, below the cap).

Every way a kind-0 missile ends goes through here: hitting its fish (§6.2), a click (§6.4), Stanley (§5.2 step 1), its target leaving (the fish's removal hook, `Fish` slot 87 `0x004df0b0`, releases its `+0x98` missile; this covers starving, being eaten or sold), or a missing target (§5.2 step 2).

`static`.

### 6.4 Clicks on missiles (`0x004d82a0`), kind 0

Called by the board mouse-down for each missile in list order until one returns true (§4.1). For kind 0, with `p = x + 40`, `q = y + 40`:

- if `p − 30 < cx < p + 30` and `q − 30 < cy < q + 30` (a 60×60 box around the centre, strict) **and the launch timer `+0x17c < 1`**: remove the missile (§6.3: EXPLODE, debris draws), then `EffectShot(cx − 40, cy − 40, 2)` (the hit spark, 1–2 more draws), return true;
- otherwise return false. A click inside the box during the first 15 ticks after launch does nothing to the missile (and the walk goes on).

**Yes, the player can shoot missiles down, one per click**, once they are 15 ticks old. The click still makes its click shot and the two shot-sound draws afterwards. Kind 1 (tank 3) is different: a click redirects it instead (`0x004d82a0` kind-1 branch, out of scope).

`static`.

## 7. Drawing and sound

### 7.1 Alien (`0x004db210` → `Alien::DrawSprite` @ `0x004dae20`), kind 5

- Sheet **`IMAGE_DESTRUCTOR`** (global `0x005e8ec8`, written at `0x00508855`; image id `0xc0`; manifest `destructor`, 10 cols × 2 rows, 160×160 cels).
- Mirror by vx as kind 1. Row 0 = swim and the fire animation, row 1 whenever the turn counter `+0x1b4` is non-zero (as kind 1; this also applies during the fire animation, which then shows its cel on row 1). Column = cel `+0x1c0` (§3.3).
- **Hit flash** (white additive, alpha `25 · +0x200`) as kind 1.
- **Fire flash:** when there is no hit flash and `0 < +0x1ac < 10`, the sprite is drawn a second time additively in white with alpha `k · 255 / 5`, `k = +0x1ac` (or `10 − k` when > 5). The draws see `+0x1ac` = 9 … 1, so the flash rises to full white at 5 and fades again: a 9-frame pulse with each volley.
- Warp-in scaling, shadow, health bar (Blip): as kind 1.
- **Dead alien:** `DeadAlien` draw (`0x004dddb0`) maps kind 5 to image `0xc0`, the same sheet, row 0.

`static`.

### 7.2 Missile, kind 0 (`Missle` slot 27 @ `0x004e2120` → `0x004e1d00`)

- First `0x004d6ae0` (calls `0x00538060` while global byte `0x005e89cd` is set; not followed).
- **`IMAGE_MISSILE`** (global `0x005e8a2c`, written at `0x0050a45b`; image id `0x9e`; manifest `missile`, 16 columns), source rect `(cel · 80, 0, 80, 80)`, normal blend, at the widget.
- **Heading cel** (`0x004e2170`, kind 0, after every move): cel 0 = down, 4 = left, 8 = up, 12 = right.
  - `|vy| < 0.8` (`−0.8 < vy < 0.8`): cel **12** if vx > 0, else **4**.
  - otherwise, by vx (same for both): vx > 2.5 → **12**; vx < −2.5 → **4**;
    - moving up (vy ≤ −0.8): vx ∈ (2, 2.5] → 11, (1.5, 2] → 10, (1, 1.5] → 9, [−1, 1] → **8**, [−1.5, −1) → 7, [−2, −1.5) → 6, [−2.5, −2) → 5;
    - moving down (vy ≥ 0.8): (2, 2.5] → 13, (1.5, 2] → 14, (1, 1.5] → 15, [−1, 1] → **0**, [−1.5, −1) → 1, [−2, −1.5) → 2, [−2.5, −2) → 3.
  
  (Constants 0.8 `0x5c3d18`, −0.8 `0x5967c8`, ±2.5 `0x5966a0/0x5967c0`, ±2.0 `0x5c3cc8/0x5965d0`, ±1.5, ±1.0.) A fresh missile (|vy| < 0.8 for its first few ticks) shows cel 12 or 4 by the sign of vx.
- **Shoot-down ball:** while `+0x1a8 > 0`, Stanley's ball is drawn from `IMAGE_BALLS` row `+0x1a0` (`pet-stanley.md#shooting-down-missiles-0x004e55d0`).
- **Smoke:** the trail of effect kind 8 (§5.2 step 5).
- **Targeted fish:** a fish with `+0x98` set (a missile on it) steps its counter `+0x90` (0..19, `0x004d7020`, from its update) and draws **`IMAGE_CROSSHAIR`** (global `0x005e8e1c`, id `0x8a`, 5 cels) additively over itself at (0, 0), cel `+0x90 / 4` (`0x004d7040`, from `Fish::DrawBody` and the other fish draws). This is the reticle that shows which fish a missile is after.

`static`.

### 7.3 Sounds

| Event | Sample | Notes |
|---|---|---|
| Spawn | `0x127` `SOUND_INTERFER` | direct, unthrottled (kinds 1–3: `ROAR`) |
| Volley fired | `0x128` `SOUND_MISSLE` | gap 3, once per volley |
| Laser hit | `0x126` `HIT` | gap 3 |
| Click | `0x144` `ZAP` (+`0x145`, +`0x132`) | the usual shot sound, 2 app draws |
| Missile explodes (any cause) | `0x11d` `EXPLODE` | gap 3 |
| Fish killed by a missile | `0x117` `DIE` | gap 3, from the fish's death |
| Stanley shoots | `0x122` `GROW` | `pet-stanley.md` |
| Death | `0x11d`, `0x11e` | gap 3 each; then the dead-alien rumble as kind 1 |

`static`.

## 8. Effects on the board and other objects

- **Missiles on `+0xdc`** have the effects listed in `pet-stanley.md#effects-on-other-objects-and-the-board` for any `+0xdc` object: the **wave countdown is frozen** while one exists; **clicking a guppy drops no pellet** (`food.md`, step 1); **Wadsworth shelters** (`pet-wadsworth.md`, threat test `0x004e3ea0`), which also makes stage-0/1 guppies untargetable (§6.1).
- **End of wave:** `0x00539c90` runs on the alien's death and on every missile removal, and only acts when aliens, Bilaterus and missiles are all gone. So after the kill, the food lockout (36), the tank music and the guppy byte `+0x55` wait for the last missile to explode, and clicks stay shots (§4.1) until then.
- **Guppies:** no hunger loss, no coins while the alien is in (the usual alien rule, `alien-sylvester.md` §5). That rule tests `+0xb8`/`+0xf4` only (`0x004da780`), not missiles, so hunger and coins resume as soon as the alien dies even if missiles remain (`static`, from the cited function).
- **Stanley** (pet 22) shoots kind-0 missiles at the lowest priority (`pet-stanley.md#shooting-down-missiles-0x004e55d0`).
- **Shared with kind 1 (tank 3, alien kind 6):** the Missle class, ctor, update, removal and the shoot-down by Stanley. Kind 1 differs in its draw (additive glow, `0x005e8a44`), its click (redirect instead of destroy), its extra `0x11f` on removal, its out-of-bounds removal and no clamp once redirected, and its hit path (which has the Wadsworth check). Alien kind 6's firing (`0x004fdd34`) uses `PickTarget(0)`/`(1)`, sample `0x143`, and launch points `+0x46`/`+0x0a`. Out of scope here.

`static`.

## 9. RNG summary (2-4)

| When | Stream | Draws, in order |
|---|---|---|
| Spawn | app | ctor `rand()`, `rand(10)`; shadow `rand(5)` (as kind 1) |
| Alien tick, wander re-roll (timer > 20, fire animation 0) | app | `rand(10)`; if 0: `rand() & 3` |
| Alien tick, volley | global, then app | per missile created: ctor `grand() % 4` (1 to 3 draws); then always `rand(50)` (interval) |
| Each kind-0 missile, each update (including its creation tick) | app | `r % 3`; if 0: `r % 5`, `r % 5`, then the effect-shot ctor (1–2, if under the cap) |
| Missile explodes (hit, click, Stanley, lost target) | app | `r % 3` count, then per piece `r % 3`, `r % 30` (y), `r % 30` (x), shot ctor |
| Player click | app | per alien hit: spark ctor (in `Hit`), kill debris if lethal; then per missile destroyed (at most one): explosion debris, then the `(cx−40, cy−40, 2)` spark ctor; then the shot sound `rand() & 1`, `rand(10)` |
| Stanley shoots a missile | global | `grand() % 100` (cooldown) |
| Alien dies | app + global | kind-1 debris (`alien-sylvester.md` §4.3); bubbles draw from the global stream; then the corpse's 10 bursts |
| 2-5 only | app | level setup `rand() & 1`; the re-roll after each spawn (`alien-kind4.md` §1) |

**Order within a tick:** `Board::Update` (ambient bubbles, countdown; frozen while missiles exist) → objects in z-order: guppies (their hunger etc.) → **the Destructor** (wander, move, firing: ctors' global draws then the interval draw) → FishTypePets (Stanley's shoot-down `grand()`) → **missiles** in `+0xdc` order (each: shoot-down step, steer, hit → kill and explosion, clamp, move, smoke) → newly fired missiles (created this tick, at the end of the widget list) → shots, coins. Player clicks run in the input phase before the update.

`static`.

## Deviations

`static`. None of our own yet. The heap-dependent effect-shot start delay (`alien-sylvester.md#deviations`) applies to every smoke puff, every explosion piece and every click spark here. With a smoke draw on about a third of all missile ticks, the harness will need to re-sync or model the stale-field draw often on 2-4.

## Open questions

- Where exactly the new missiles land in the update order on the fire tick: `AddMissile` brings each to the front and then `board+0x100` to the front. If the widget manager's walk has already passed the end, they might not update until the next tick. Needs the first 2-4 capture (the launch timer reads 14 or 15 on the snapshot).
- Whether the missing Wadsworth check on the kind-0 hit is real in play (a sheltering guppy that was targeted just before the shelter started): rare, needs a zoo with Wadsworth on 2-4.
- `0x004d6ae0`/`0x00538060` at the start of the missile draw, gated by `0x005e89cd`: purpose unknown.
- Whether `Fish::Die(1)` for an Oscar or a Breeder drops anything extra (not checked beyond "no RNG").
- The A.y drawn at the warning is unused for kind 5 (y forced to 280). Confirm that the reticle (`board+0x178`) isn't involved on 2-4.

## Recording

A 2-4 recording (the game's `-record` demo plus a CE capture, `dynamic-analysis.md` routine) must exercise:

1. The warning (TYPE-D), the spawn (INTERFER, warp, 3 draws, y = 280), the wander along the floor.
2. **Volleys:** at least three, with enough guppies that all three missiles fire; the farthest-fish choice, the MISSLE sound, the fire animation and flash, the crosshairs on the targeted fish, the smoke trail.
3. **Missiles hitting fish:** let at least one missile reach a guppy (DIE, DeadFish, EXPLODE, debris draws).
4. **Missiles shot by clicks:** one click inside the box during the first 15 ticks (no effect), then a successful shot; one click that hits the alien and a missile together; clicks after the alien is dead while a missile is still flying (no food).
5. **The kill by the laser** (25 hits at the base laser, or fewer after an upgrade; knockback only sideways, none during the fire animation): the diamond, the corpse settling at y 280, and the end-of-wave work waiting for the last missile.
6. Optionally, a guppy-free stretch (the firing timer stops).
7. **A second capture with Stanley** chosen as a pet: Stanley shooting a kind-0 missile down (GROW, the ball drawn, the missile removed after the computed duration), plus his balls hitting the Destructor (15 HP).

`static` (a plan).
