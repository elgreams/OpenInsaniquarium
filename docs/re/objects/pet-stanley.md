# Stanley the startlingly small sea serpent (`Sexy::FishTypePet`, pet id 22)

Stanley swims like a guppy whether or not aliens are in, and never chases anything. While an alien is in he **grows into a big sea serpent** (a 10-tick transformation, reversed when the tank is calm), and he has two ball powers:

- **Throwing.** Every 101 ticks he throws a **ball** (a `Missle` of kind 2) at the nearest alien that has no ball on it yet. The ball homes in, hits for **15 HP**, bounces off and falls out of the tank.
- **Shooting down missiles.** When an alien missile (missile kinds 0/1) is in flight, he throws a ball at it: after a delay of `distance / 16` ticks the missile explodes. Then he rests 30, 60 or 90 ticks (global RNG) before he can do it again.

In-game text: "STANLEY's seeming unlimited supply of balls are the scourge of alien missile shooters everywhere. Some players wonder why he bothers throwing balls directly at the aliens, though. STANLEY answers: "I like balls."" He is a bonus pet, bought with shells (`screens/bonus-screen.md`, 30 000).

Same class, update and fields as Itchy (`pet-itchy.md`). Only the id-22 differences are listed here. All offsets are raw `this`-relative (Ghidra's `FishTypePet_data.offset_0xN` = raw `+0x22c + N`, `Fish_data.field_0xN` = raw `+0x154 + N`, `Missle_data.offset_0xN` = raw `+0x154 + N`).

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`: id-0x16 branch (`+0x224 = 100`)
- Update (slot 22) @ `0x004f7ad0`: id-0x16 test at `0x004f8182` (slot 81 every tick)
- Gate (slot 82) @ `0x004e9790`: no id-22 branch
- Abilities (slot 81) @ `0x004e5900`, id-0x16 branch at `0x004e5ed0`: grow counter, throw clock; missile shoot-down `0x004e55d0`; target (slot 85) @ `0x004e5fc0` (id-0x16 branches at the top and the alien `+0x98` test)
- Ball: `Board::AddMissle` `0x00544130` → `Missle::Missle` @ `0x004eae40` (vtable `0x00596c1c`); `Missle::Update` (slot 22) @ `0x004ffb30`; steer `0x004ff4e0`; hit `0x004fc070` (kind-2 half); spin `0x004e2170`; removal (slot 76) @ `0x004d80e0`; draw `0x004e1d00`
- Animation (slot 89) @ `0x004dfc50` (id-0x16 branch after the Nimbus/Brinkley one); draw body `0x004f8610` case 0x16 → `0x004dfac0` with `IMAGE_STANLEY`

Confidence: everything below is `static` (read from the code; nothing captured yet).

## Fields used by id 22

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), with:

| Offset | Type | Meaning for Stanley | Initial |
|---|---|---|---|
| `+0x220` | int | **throw clock** (the guppy coin timer): +1 per tick (aliens or not); 0 after a throw-clock fire and after each missile shot | 0 |
| `+0x224` | int | throw interval: **100** (fires when the clock exceeds it, so every 101 ticks); also in the Virtual Tank | 100 |
| `+0x23c` | int | **shoot-down cooldown**: −1 per tick in the update (Itchy's step 2); set to 30/60/90 after each missile shot, and to 50 by a throw-clock fire when it is 0; missiles are shot only while it is 0 | 0 |
| `+0x260` | int | **grow counter** (Ghidra `offset_0x34`): +1 per tick while an alien or Bilaterus is in (up to 10), −1 per tick otherwise (down to 0); drives the transformation row | 0 |
| `+0x1c8` | int | **throw animation** (the guppy eat timer): set to 10 by a throw or a shot, counts down in the animation (cel = value); cleared when a turn ends | 0 |

Divisor `+0x180` stays `Fish::Init`'s draw; bounds are a guppy's (floor 370, min x 10, ceiling 95, max x 540). The board's per-id pet counter for id 22 is `board+0x144 + 4·22 = board+0x19c` (`board.md#object-lists`); no reader of it was looked for.

`static`.

## Construction

`Board::AddPet(22, -1, -1)` (`0x00544a90`), app MTRand, in order:

1. Position: `x = r % 265 + 105`, `y = r % 520 + 20` (2 draws; none when AddPet gets a position, as for Presto's change).
2. `Fish::Init` (`0x004de370`): the guppy's 6 draws (facing, divisor, hunger, growth, mode, coin interval).
3. Ctor id-0x16 branch: `+0x224 = 100`. No draw (no Virtual Tank variant). `+0x23c = 0`, `+0x260 = 0`.
4. Shadow (`0x00544800`): type code `0x15` with id ≠ 16/20 → **type-0 shadow**, whose ctor makes **1 draw** (`% 5`).

So adding Stanley costs **9 app draws**, as Itchy; made by Presto (`pet-presto.md#the-change`), 7.

`static`.

## Update (`0x004f7ad0`), id 22

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 2:** the shoot-down cooldown `+0x23c` counts down by 1 while non-zero.
- **Step 3, gate:** no id-22 branch: it returns false, so the **guppy swim modes always run**, aliens or not. Stanley never chases or stabs.
- **Step 5:** the 21-tick re-roll as Itchy (`r % 10`, then `r % 9 + 1` on 0).
- **Step 6, slot 81:** for every other pet it runs only on calm ticks; for id 22 the update calls it **every tick** (the id-0x16 test at `0x004f8182`). Slot 81 itself still returns at once on tank 5 (`board+0x3c8 == 5`) and while the board is paused (`board+0x94`). See [Abilities](#abilities-slot-81-0x004e5900-id-22).
- **Animation:** the id-22 branch (below).

App-stream RNG: the re-roll only. The ability draws from the **global** stream (`Sexy::Rand`, `0x0040f9e0`), at most one draw per tick (a shot and a throw can't happen in the same tick, below).

`static`.

## Abilities (slot 81, `0x004e5900`, id 22)

Each tick, in order:

1. **Grow counter:** if an alien (`board+0xb8`) or Bilaterus (`board+0xf4`) is present (`0x004da780`), `+0x260 += 1` while < 10; otherwise `+0x260 −= 1` while > 0.
2. **Throw clock:** `+0x220 += 1`.
3. **Missile shoot-down** (`0x004e55d0`, [below](#shooting-down-missiles-0x004e55d0)). It may reset `+0x220` to 0.
4. If `+0x220 > +0x224` (i.e. it reached 101):
   1. `+0x220 = 0`; if `+0x23c == 0`, `+0x23c = 50` (a throw-clock fire blocks shoot-downs for 50 ticks, even when nothing is thrown).
   2. **Target** (slot 85, `0x004e5fc0`, id 22): best = 10 000, strict `<` (first minimum wins). First the Bilaterus list `+0xf4` in list order, candidates only with **`+0x98 == 0`** (no ball on it), centre offset 40: `dx = trunc((x + 40) − (b.wx + 40))`, `dy = trunc((y + 40) − (b.wy + 40))`. Then the alien list `+0xb8` in list order, candidates with `+0x204 == 0` **and `+0x98 == 0`**, centre offset 80 (as Itchy). `d = trunc(f32 sqrt(f32(dx² + dy²)))`.
   3. No target: nothing more. Otherwise **throw a ball** (`0x00544130`): `Missle(bx, wy, target, kind 2)` with `bx = wx + 40` when vx ≥ 0, `bx = wx` when vx < 0 (his widget position; the ball leaves from his front). Then sample **`0x122` (`GROW`)**, gap 3 (`0x00538230`), and `+0x1c8 = 10`.

So with an alien in, a ball goes every 101 ticks (2.8 s at 28 ms/tick), as long as some alien has no ball on it. With one alien that means a new ball only after the last one hit (or left the tank). A shoot-down restarts the 101-tick clock.

`static`.

## The ball (`Missle`, kind 2)

`0x00544130(x, y, target, kind)`: allocate a `0x1b8` `Missle`, ctor (`0x004eae40`), `Board::AddGameObject(missile, 0)` → for kinds other than 3/4/5 the **missile list `+0xdc`**, added to the widget manager and the z-order; then for kinds other than 3/4/5 **`target+0x98 = missile`** (the alien now "has a ball on it").

Ctor, kind 2 (fields raw):

| Offset | Meaning | Ball's initial value |
|---|---|---|
| `+0x158`, `+0x160` | x, y (doubles); widget = trunc | the throw position |
| `+0x168`, `+0x170` | vx, vy | 0, 0 |
| `+0x17c` | launch timer, −1 per tick while > 0 | 15 |
| `+0x180` | spin counter (0..19) | 0 |
| `+0x188` | speed divisor | **0.4** (0.8 for other kinds) |
| `+0x190` | byte "bounced" | 0 |
| `+0x194` | cel | 1 |
| `+0x19c` | missile kind | 2 |
| `+0x1a0` | row: **`grand() & 3`** (ball colour, 0..3) | drawn |
| `+0x1a4` | target | the alien |
| `+0x1a8`, `+0x1ac` | shoot-down progress / duration (used on kinds 0/1, below) | 0, 10 |
| widget w/h | | **50 × 50** (80 × 80 for other kinds) |

RNG: **1 global draw** (`& 3`). The app-stream direction draws (`r % 6` twice) are only for kinds 3–5. Not mouse-visible.

`Missle::Update` (`0x004ffb30`), kind 2, each tick (return while paused; `Fish::PreUpdate`-style common timers first):

1. **Steer** (`0x004ff4e0`), skipped once bounced, and skipped while neither an alien nor a Bilaterus is in. With `c = 25` (40 when any Bilaterus exists, which also switches to the generic box below), target widget `(tx, ty)`, `p = x + c`, first matching rule (doubles):

   | Condition | vx change |
   |---|---|
   | `p > tx + 200` | −0.2 if vx > −1.8 |
   | `p < tx − 40` | +0.2 if vx < 1.8 |
   | `p > tx + 110` | −0.2 if vx > −1.4 |
   | `p < tx + 50` | +0.2 if vx < 1.4 |
   | `p < tx + 80` | +0.2 if vx < 0.6 |
   | `p == tx + 80` | none |
   | else (`tx + 80 < p ≤ tx + 110`) | −0.2 if vx > −0.6 |

   Then with `q = y + c` and `ty`, the same bands, except the inner two: `q < ty + 80` → **+0.1 if vy < 0.2**; else (`ty + 80 < q ≤ ty + 110`) → **−0.1 if vy > −0.2**. The aim point puts the ball's centre (x + 25) on the alien's centre (tx + 80). With a Bilaterus present the bands are the generic ones (offsets 0/20/40/60/80 around the target, centre 40); not detailed here.
2. **Hit** (`0x004fc070`, kind 2) while not bounced and the target is set: `tx + 20 < x + 25 < tx + 140` and `ty + 20 < y + 25 < ty + 140` (strict; Bilaterus segment: 10..70). On a hit:
   1. bounced `+0x190 = 1`, `target+0x98 = 0`, target = 0;
   2. row < 4 (always): sample **`0x136` (`SFX`)**, gap 3; damage **15**; divisor `+0x188 = 1.5`. (Row ≥ 4 would be `PUNCH` gap 3, damage 55, divisor 0.6; unreachable.)
   3. `vx = −vx`, `vy = −vy`;
   4. alien (type `0x16`) with `+0x204 == 0`: **health `+0x1f0 −= 15`**; if ≤ 0, **die at once** (`0x004f9c70(1)`, `alien-sylvester.md#43-death-0x004f9c701-kind-1`: diamond, debris app draws, all inside the ball's update). Bilaterus: its owner's health `+0x1d8 −= 15`, death `0x004fb4c0(1)` at ≤ 0. Other target types: no damage.

   The ball is not removed by a hit.
3. **Out of bounds:** `x > 580`, `x < −20`, `y > 380` or `y < 45` → removed (slot 76 `0x004d80e0`: clears its target's `+0x98`, leaves the widget manager, registry and list, then `0x00539c90`, the end-of-wave check). No sound or debris for kind 2.
4. **Gravity** once bounced: `vy += 0.2`.
5. **Move:** `x += vx / div`, `y += vy / div`; launch timer −1; widget = trunc.
6. **Spin** (`0x004e2170`): counter `+0x180` steps by 2 (1 once bounced): up when vx < 0 (mod 20), down when vx ≥ 0 (wrapping +20); cel `+0x194 = counter / 2`.

So a thrown ball starts at rest, accelerates by 0.2 per tick up to 1.8 (4.5 px per tick through the 0.4 divisor), slows near the alien, and after the hit flies back the way it came while falling (gravity 0.2/1.5 per tick) until it leaves the tank (usually through the bottom, y > 380). With Sylvester's 50 HP, 4 hits kill him (`alien-sylvester.md`).

If the alien dies some other way while a ball is still on it, the death releases its `+0x98` missile (slot 76): the ball vanishes. A bounced ball has already been cleared from `+0x98`, so it keeps falling.

`static`.

## Shooting down missiles (`0x004e55d0`)

Runs from step 3 of the abilities, every tick. (Its first loop walks the alien list and does nothing.) Only when the cooldown `+0x23c` is 0:

1. **Pick a missile** from `board+0xdc` in list order: candidates have launch timer `+0x17c < 11` (at least 5 ticks in flight), are not already shot (`+0x1a8 < 1`), kind `+0x19c` 0 or 1 (never his own balls, kind 2, or kinds 3–5 on `+0xe0`) and a target `+0x1a4`. Priority: kind 1 with byte `+0x190` set (2) > kind 1 without (1) > kind 0 (0); a lower-priority missile is skipped once a higher one was found. Among equals, the smallest **integer d² from the missile's widget centre to its target's widget centre** wins (strict `<`, start 100 000 000); a candidate that raises the priority always wins. So he shoots the missile that is closest to hitting its fish.
2. If one was found:
   1. **`grand() % 100`** (global stream): < 10 → cooldown **30**; 10..59 → **60**; 60..99 → **90**.
   2. Throw clock `+0x220 = 0`.
   3. Sample **`0x122` (`GROW`)**, gap 3.
   4. On the missile: start point `+0x1b0 = wx + w/2` (his widget centre x), `+0x1b4 = wy + 10`; progress `+0x1a8 = 1`; duration **`+0x1ac = trunc(f32(sqrt(d²)) · 0.0625)`**, d² the integer squared distance from that start point to the missile's widget centre (1 tick per 16 px).
   5. `+0x1c8 = 10` if it was 0 (throw animation).

The cooldown branch for `+0x23c > 0` after the loop (`cooldown −= 2` when a priority-2 missile exists) is dead code: the function only gets there with the cooldown at 0.

**The shot missile** (`Missle::Update`, top): while `+0x1a8 > 0`, `+0x1a8 += 1`; when it reaches `+0x1ac` the missile is **removed** (slot 76): its target's `+0x98` cleared, `0x00539c90`, sample `0x11f` (`EXPLOSION4`) for kind 1, then sample **`0x11d` (`EXPLODE`)** (both gap 3), and debris on the **app** stream: `n = r % 3 + 2`, then n times `kind = r % 3 + 3`, `y = wy + r % 30 − 10`, `x = wx + r % 30 − 10` (in that order) and an effect shot `0x005436f0(x, y, kind)` (which may draw more, `alien-sylvester.md`). Until then the missile **keeps flying and can still hit its fish**: the shot doesn't stop it. A duration of 0 or 1 removes it on its next update.

The ball of a shoot-down is drawn only: the missile's draw (`0x004e1d00`) puts `IMAGE_BALLS` cel `+0x1a8 % 10`, row `+0x1a0` (the missile's own `grand() & 3` row from its ctor) at the integer interpolation `((dur − t)·start + t·centre) / dur` from his start point to the missile's centre, offset by −25. No ball object exists.

`static`.

## Effects on other objects and the board

- **Aliens:** 15 HP per ball, at most one ball in flight per alien (the `+0x98` test). The kill happens inside the ball's update (the missile pass comes after the FishTypePets, `sim/board.md#tick-order`), not on the alien's next tick as with Itchy.
- **Missile list:** balls live on `+0xdc`, so while one is in the tank the **wave countdown is frozen** (`alien-sylvester.md#per-tick-countdown`, step 2), clicking a guppy drops no pellet (`food.md#clicking-on-a-guppy-fish-mouse-down-slot-55-0x004e4c10`, step 1), and Wadsworth shelters (`pet-wadsworth.md`). The alien a ball is on has its `+0x98` set, so its update steps the attached-missile counter (`0x004d7020`) and its draw adds the missile overlay (`0x004d7040`, not followed). When the last alien dies while a bounced ball is still falling, the end-of-wave work (`0x00539c90`: food lockout 36, tank music) waits until the ball leaves the tank.
- **Alien missiles:** shot down as above.
- **Other pets / guppies:** none.
- **Clicks:** not mouse-visible (`+0x55` = byte `0x005e8f14`, 0); no id-22 case in the mouse-down `0x004fb680`. The ball isn't clickable either.

`static`.

## Animation (`0x004dfc50`), id 22

Turning is Itchy's (turn ±20, cel `9 − turn/2` / `turn/2 + 9`; a turn reaching 0 clears `+0x1c8`). When not turning:

1. If the grow counter is **1..9**: cel = counter (the transformation frame); nothing else advances.
2. Else if `+0x1c8 > 0`: `+0x1c8 −= 1`, **cel = `+0x1c8`** (throw animation, cels 9..0).
3. Else the swim cycle: `+0x1b8 += 1` if the speed bucket `+0x1bc < 2`, else `+= 2`; 0 when it exceeds 39; **cel = cycle / 4** (0..9).

Facing follows vx (guppy rule). No RNG.

`static`.

## Drawing (`0x004dfac0`)

`IMAGE_STANLEY` (global `0x005e8d00`; `resources.xml` `stanley`, 10 × 6 cels of 80 × 80, framedelay 2), src `(cel · 80, row · 80)`:

| | no alien | alien or Bilaterus present |
|---|---|---|
| grow counter ≠ 0 (no alien) / < 10 (alien) | row 5 | row 5 |
| throw animation `+0x1c8 ≠ 0` | (row 0) | row 4 |
| swimming | row 0 | row 2 |
| turning | row 1 | row 3 |

First match from the top in each column (with no alien, `+0x1c8` doesn't change the row). Mirroring: rows 1 and 3 mirrored iff turn > 0; the others take `Fish::Draw`'s guppy mirror choice. So rows 0/1 are the small serpent, 2/3/4 the big one, 5 the growing/shrinking frames (cel = counter, 1..9). No ZZZ, no shake. Then the shared tail of `0x004f8610`: Presto puff when made by Presto, missile overlay if `+0x98` is set, drop-in flash. Type-0 shadow.

The ball: `0x004e1d00` kind 2 → `IMAGE_BALLS` (global `0x005e8ee8`; `balls`, 10 × 4 cels) cel `+0x194`, row `+0x1a0`, at its widget.

`static`.

## Open questions

- Everything is static. The grow counter, the 101-tick throw, the ball's steering and bounce, and the 15 damage are the parts to capture first; the global-stream colour draw (`& 3`) needs the global MT state in the capture.
- Who fires the missiles he shoots: `Alien::Update` (`0x004fd560`) makes missile **kind 0** for alien kind 5 (call at `0x004fdb7b`, from `(wx − 10, wy − 35)`) and **kind 1** for alien kind 6 (`0x004fddac`, from `(wx + 10, wy − 24)`), each at a target from `0x004e8df0`. Which named aliens these are, what the kind-1 byte `+0x190` means, and on which tank they first come: not followed.
- The ball's generic steering bands with a Bilaterus present were not written out.
- The board pet counter `board+0x19c` (id 22): readers not looked for.
- Whether alien missiles can target Stanley (his `+0x98`): the missile target pick was not followed.

`static`.

## Recording

A pet zoo on tank 1-2 with Stanley, through one Sylvester wave (one capture chunk, keeping the RNG header and the **global** MT state, since every ball draws `grand() & 3`):

- Before the wave: the grow counter stays 0; the throw clock fires every 101 ticks with no target (sets `+0x23c = 50`, nothing else).
- The wave: the transformation (row 5, 10 ticks), a ball every 101 ticks; Sylvester (50 HP) dies on the 4th hit if nothing else hurts him, so don't click him. The kill comes inside the ball's update, and the end of the wave waits for the last falling ball. After the wave the counter runs back down.
- To save time, push `+0x220` to 95 just after the warning (the replay must adopt that write); the wave itself can be brought forward by pushing the board countdown `board+0x2c0` as in earlier zoos.

The missile shoot-down needs an alien of kind 5 or 6, which fire missiles (not on 1-2): a second session on a later tank, or later in the bonus-pet work. Optional: Wadsworth (pet 9) alongside, to see him shelter while a ball is in flight.

`static` (a plan).
