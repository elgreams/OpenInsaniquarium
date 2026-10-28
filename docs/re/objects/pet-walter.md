# Walter the Penguin (`Sexy::FishTypePet`, pet id 23)

Walter swims like a guppy whether or not aliens are in, and never chases anything. He is the one pet that is **always clickable**: a left click on him throws a **punch**. A **boxing glove** shoots out on the side he faces, and every guppy, carnivore-type fish or fish pet it touches is **knocked sideways** (a 50-tick push). A knocked guppy or coin-dropping pet also gets its coin clock moved to **5 ticks before its next coin** (10 for Nostradamus), so punching makes coins come sooner. Aliens, coins and the non-fish pets are not hit. After **5 punches** he is tired for **360 ticks** (10 s): he sinks, shows `ZZZ`, and clicks on him go through to the tank. A pet that Presto turned into is turned **back into Presto** when the glove knocks it (`pet-presto.md#update-0x004f7ad0-id-19`).

In-game text: "WALTER masks his tremendous insecurities with sheer bravado and scrappiness. On the outside, this happy-go-lucky penguin seems to have not a care in the world. On the inside, he has a great big boxing glove which he uses to punch people." He is a bonus pet, bought with shells (`screens/bonus-screen.md`).

Same class, update and fields as Itchy (`pet-itchy.md`). Only the id-23 differences are listed here. All offsets are raw `this`-relative (Ghidra's `FishTypePet_data.offset_0xN` = raw `+0x22c + N`, `Fish_data.field_0xN` = raw `+0x154 + N`, `Widget_data.offset_0x28` = raw `+0x7c`, `Widget_data.field_0x1c/0x24` = raw `+0x70/+0x78`, `BoxingGlove_data.offset_0xN` = raw `+0x154 + N`).

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`: id-0x17 branch at `0x004ef50f` (mouse-visible, insets)
- Update (slot 22) @ `0x004f7ad0`: glove update/delete at `0x004f7b84`, tired countdown at `0x004f7bbb`, mode-0 sink at `0x004f7db4`, re-roll gate at `0x004f8120`
- Gate (slot 82) @ `0x004e9790` and abilities (slot 81) @ `0x004e5900`: no id-23 branch
- Mouse-down (slot 54) @ `0x004fbe20` → `0x004fb680`, id-0x17 block at `0x004fbc40`
- Glove: `Sexy::BoxingGlove` (vtable `0x00598034`, object size `0x168`), ctor @ `0x004ef7c0`; update (slot 22) @ `0x004e9a60`; hit test `0x004e0ed0`; draw (slot 26) @ `0x004e1000`; deleted by Walter's update or his dtor (`0x004ef750`)
- Animation (slot 89) @ `0x004dfc50`, id-0x17 branch (and a turn-trigger test at the top); draw body `0x004f8610` case 0x17 → `0x004dfa00` with `IMAGE_WALTER`
- Presto hand-over of `+0x260`/`+0x264`: slot 73 @ `0x004f89a0` at `0x004f8a56`

Confidence: everything below is `static` (read from the code; nothing captured yet).

Capture: `walter_01` (1-2 with Walter and Presto; Presto changed into Itchy, then Walter clicked 8 times: 6 punches, guppies and the Itchy knocked, the Itchy turning back into Presto, tired after the 5th punch and awake 360 ticks later, clicks during a punch falling through), replayed exactly from the game's own demo. `traced` where a section says so, else `static`.

## Fields used by id 23

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), with:

| Offset | Type | Meaning for Walter | Initial |
|---|---|---|---|
| `+0x55` | byte | widget "mouse visible": **1** (every other pet takes byte `0x005e8f14`, 0) | 1 |
| `+0x7c` | byte | widget hand-cursor flag (probably SexyApp's `mDoFinger`, as on Amp): 1; 0 while tired, back to 1 when the tired timer ends | 1 |
| `+0x70`, `+0x78` | int | mouse insets top / bottom: **10** / **15** (left/right `+0x6c`/`+0x74` keep the Widget default; not checked) | 10, 15 |
| `+0x1c8` | int | **punch timer** (the guppy eat timer): set to **40** by a punch, −1 per tick in the animation; drives the punch row and holds the glove for its first 5 ticks | 0 |
| `+0x24c` | ptr | **the glove** (Ghidra `offset_0x20`), or null | null |
| `+0x260` | int | **punch count** (Ghidra `offset_0x34`): +1 per punch outside the Virtual Tank; at 5 he tires and it goes back to 0 | 0 |
| `+0x264` | int | **tired timer** (Ghidra `offset_0x38`): 360 (global `0x005df034`) when he tires, −1 per tick (Itchy's step 2); reaching 0 sets `+0x7c = 1` | 0 |

Divisor `+0x180` stays `Fish::Init`'s draw; bounds are a guppy's (floor 370, min x 10, ceiling 95, max x 540); widget 80 × 80. The board's pet counter for id 23 is `board+0x144 + 4·23 = board+0x1a0`; no reader was looked for.

Correction to the bonus-pet plan (`bonus-pets-plan.md`): it is the tired timer `+0x264`, not the Presto recharge `+0x238`, whose end sets the widget byte `+0x7c`. `+0x238` behaves as for any pet.

`traced` by `walter_punches` (`walter_01`) (`+0x1c8`, the glove's presence, `+0x260`, `+0x264`).

## Construction

`Board::AddPet(23, -1, -1)` (`0x00544a90`), app MTRand, in order:

1. Position: `x = r % 265 + 105`, `y = r % 520 + 20` (2 draws; none when AddPet gets a position, as for Presto's change).
2. `Fish::Init` (`0x004de370`): the guppy's 6 draws (facing, divisor, hunger, growth, mode, coin interval).
3. Ctor id-0x17 branch: `+0x55 = 1`, `+0x7c = 1`, `+0x70 = 10`, `+0x78 = 15`. **No draw.** The shared ctor part zeroes `+0x1c8`, `+0x24c`, `+0x260`, `+0x264`.
4. Shadow (`0x00544800`): type code `0x15` with id ≠ 16/20 → **type-0 shadow**, whose ctor makes **1 draw** (`% 5`).

So adding Walter costs **9 app draws**, as Itchy; made by Presto (`pet-presto.md#the-change`), 7. A Walter made by Presto (or Presto made from Walter) takes the old pet's `+0x260` and `+0x264` (`0x004f8a56`: copied only between ids 19 and 23, either way), so changing through Presto neither resets the punch count nor wakes a tired Walter.

`traced` by `walter_punches` (`walter_01`). The Walter↔Presto hand-over is `static`.

## Update (`0x004f7ad0`), id 23

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 1a, the glove** (right after `Fish::PreUpdate` and the id-13 block): if `+0x24c` is set, call the glove's update (its slot 22, [below](#the-boxing-glove)); then if the glove's life `+0x15c` < 1, delete it (its slot 0, the deleting dtor) and clear `+0x24c`. Any FishTypePet runs this, but only Walter ever has a glove.
- **Step 2:** the tired timer `+0x264` −1 while non-zero; when it reaches 0 (id 23 only), `+0x7c = 1`. (`+0x23c` −1 while non-zero, unused.)
- **Step 3, gate:** no id-23 branch: returns false, so the **guppy swim modes always run**, aliens or not. Walter never chases.
- **Step 4, mode 0:** vy = **1.0** while tired (`+0x264 ≥ 1`), else 0.5 as Itchy. The 5th punch sets mode 0, so a tired Walter sinks twice as fast and lies on the floor.
- **Step 5, re-roll:** every 21 ticks `r % 10` as Itchy; on 0 the second draw `r % 9 + 1` (new mode) is made **only when not tired**. So while tired: one draw every 21 ticks, never two, and the mode stays 0.
- **Step 6, slot 81:** no id-23 branch; nothing happens.
- **Step 7:** the push timer as for any pet. Walter can be pushed by another Walter's glove (not by his own).
- **Animation:** the id-23 branch (below).

App-stream RNG: the re-roll only. The glove's hits draw from the **global** stream (`Sexy::Rand`, `0x0040f9e0`), one draw per object hit.

`traced` by `walter_punches` (`walter_01`) (tired sinking, the single re-roll draw while tired).

## Clicks (slot 54 `0x004fbe20` → `0x004fb680`)

Walter is mouse-visible from construction, so clicks inside his widget (80 × 80, minus the mouse insets: 10 px off the top, 15 off the bottom, so y from wy + 10 to wy + 65; via the inset-rect method `0x0046e840`; that the widget manager's hit test uses it is assumed from SexyApp, not checked) reach his mouse-down instead of whatever is under him. Slot 54 calls `0x004fb680(x, y, clicks)`; if it returns false and the re-entry guard byte `0x005e89f9` is clear, the click is passed to the board's mouse-down at the same tank position (food, coin pick-up, etc.).

- **Right button** (`clicks < 0`): the shared first block (`pet-presto.md#right-click-0x004fb680-first-block`): unflagged → tank right-click at that spot (`0x0053bfb0`: Presto lookup, else glass tap); Presto-flagged → recharge check / PetDialog. Consumed either way.
- **Left button**, id 23: the punch is allowed only when **all** of these are 0: punch timer `+0x1c8`, glove `+0x24c`, tired timer `+0x264`, turn `+0x1c4`. Otherwise returns false: the click goes to the tank as if he weren't there (a food drop, for instance).
- **Direction** (vx `+0x168`, facing `+0x188`), first match:
  1. vx < 0 → glove **left** (direction byte 0);
  2. `trunc(vx) == 0` and facing < 0 → **left**;
  3. vx > 0 → **right** (1);
  4. facing > 0 (vx is 0 here) → **right**;
  5. otherwise (vx = 0 and facing = 0, not reachable with ±1 facing) → no glove; the click is still consumed with no effect.

  Note rule 2 before 3: with 0 < vx < 1 and facing still negative (he has started to turn right but facing hasn't followed), the glove goes left.
- **The punch** (when a glove was made), in order:
  1. Allocate a `0x168` `BoxingGlove(owner = Walter, direction)` and store it in `+0x24c`. It is **not** added to any board list, the master set or the widget manager: Walter owns, updates and draws it.
  2. Outside the Virtual Tank (`app+0x87c ≠ 5`): `+0x260 += 1`; if it reaches **5**: tired timer `+0x264 = 360`, `+0x260 = 0`, `+0x7c = 0`, mode `+0x1ac = 0`, and his widget slot 38 with 0 (`0x0046e370`, a widget-manager cursor call; not followed). The 5th punch still punches.
  3. Cel `+0x1c0 = 0`, punch timer `+0x1c8 = 40`.
  4. Sample **`0x136` (`SFX`)**, gap 3.
  5. `Board::RebuildZOrder` (`0x0053aa40`; it has no id-23 or glove case, so the order itself doesn't change for him).
  6. Return true (consumed).

No RNG in the click. The click handler doesn't look at aliens: he can punch during a wave.

`traced` by `walter_punches` (`walter_01`) for the gate, direction and the punch; the mouse insets are as built (clicks at wy + 35).

## The boxing glove

**Ctor** (`0x004ef7c0`, `GameObject` base ctor `0x004e9cb0` first, which draws nothing):

| Offset | Meaning | Initial |
|---|---|---|
| `+0x30`, `+0x34` | widget x, y | **−200, −200** (off-screen until its first positioning) |
| `+0x38`, `+0x3c` | widget w, h | 160 × 80 |
| `+0x55` | mouse visible | 0 |
| `+0x158` | owner (Walter) | |
| `+0x15c` | **life**, −1 per active tick | **30** |
| `+0x160` | byte direction: 0 left, 1 right | from the click |
| `+0x164` | punch-sound cooldown | 0 |

No RNG.

**Update** (`0x004e9a60`), called from Walter's step 1a. Each tick:

1. Return if there's no board or it's paused.
2. If the owner is set: if the owner's punch timer `+0x1c8` > **35**, return (nothing else happens: the glove waits, unmoved, for the first 5 ticks of the punch). Otherwise put the widget at `x = owner.wx − 60` (left) or `owner.wx − 20` (right), `y = owner.wy`.
3. `life −= 1`. If `life ≤ 0`, return (Walter deletes it right after).
4. Sound cooldown `+0x164` −1 while > 0.
5. Only while `life > 12`: the **reach** `e` = 0 for life 26..29, 30 for 21..25, 60 for 13..20. The **fist point** is `px = gx + 80 + e` (right) or `gx + 80 − e` (left), `py = gy + 40` (glove widget `gx, gy`). Relative to Walter that is `wx + 60 + e` / `wx + 20 − e`, `wy + 40`.
6. Walk the board's **master set `+0x104`** (a `std::set<GameObject*>`: pointer-address order, `alien-sylvester.md`), skipping the owner and objects whose byte `+0xf4` is 0 (`GameObject` sets it to 1; meaning not followed). By type code `+0x8c`:

   | Type | Hit? | Strength `k` |
   |---|---|---|
   | `0x00` guppy, `0x05` Oscar, `0x06` Ultra, `0x07` Gekko, `0x22`–`0x24` (SylvesterFish, BallFish, BiFish) | yes | 5 |
   | `0x15` FishTypePet id 3 (Prego) | **no** | |
   | `0x15` FishTypePet ids 6 (Vert), 11 (Shrapnel), 16 (Amp) | yes | 5 |
   | `0x15` FishTypePet id 21 (Nostradamus) | yes | 10 |
   | `0x15` any other id (Itchy, Presto, Stanley, another Walter, ...) | yes | −1 (no clock change) |
   | everything else: aliens `0x16`, Bilaterus `0x17`, OtherTypePets `0x14`, Penta `0x08`, Grubber `0x09`, Breeder `0x0a`, coins, food, missiles, shadows, shots | no | |

   For each candidate, the **hit test** `0x004e0ed0(s, k)` with `s` = +1 (right) or −1 (left):
   1. Skip if the object's push timer `+0x20c` > 0 (already knocked), or if it is a FishTypePet with the Presto flag `+0x230` and its raw `+0x28` < 30. `+0x28` is the widget's update counter (+1 per update, `WidgetContainer::Update` `0x0046c4f0`, called from `Fish::PreUpdate`), so a pet Presto made fewer than 30 updates ago can't be punched.
   2. Object centre `cx = wx + w/2`, `cy = wy + h/2` (ints, C division). Hit iff `px − 40 < cx < px + 40` and `py − 40 < cy < py + 40` (strict).
   3. On a hit, in order:
      - if `k > 0`: **coin clock `+0x220 = +0x224 − k`**: the object's clock is put `k` ticks short of its interval, so its next coin / bomb / charge / potion comes after `k` more clock ticks (sooner or later than it would have, depending on where the clock was). For a guppy that means a coin 5 ticks later if its stage allows coins; for **Amp** (clock 300 → threshold 3000+, `pet-amp.md`) a punch brings him to **5 calm ticks from fully charged**; Vert's coin and Shrapnel's bomb 5 calm ticks away, Nostradamus's potion 10;
      - **push timer `+0x20c = 50`**, misc timer `+0x210 = 180`;
      - if `+0x11c ≠ −1`: `0x005156f0(board+0x90 queue, +0x11c)`, which removes the object's queued entries from the board's sound queue (the emote of `Fish::PreUpdate`; not followed);
      - `0x004d6cc0`: if byte `+0xfc` is set, `+0x100` = 1 when it was 0, else capped at 10 (the z-order "hidden" pair, `board.md`; not followed);
      - **`grand()`** (global stream, 1 draw): even → **push `+0x218 = s · 15.0`**, odd → **`s · 10.0`**.
7. If anything was hit this tick and the sound cooldown is 0: sample **`0x130` (`PUNCH`)**, gap 3 (`0x00538340(3)`), and cooldown = 5.

**What a hit does to the target** (its own update, push-timer step, `fish-guppy.md#fields`): while `+0x20c > 0`: `+0x20c −= 1`, `push *= 0.9`, `x += push`. So a 15 push moves it about 134 px over 50 ticks (10: about 90 px), clamped by its bounds as usual. No damage, no death. A guppy or carnivore hit is not otherwise disturbed (its swim mode keeps running). `+0x210` (180) is the guppy's misc countdown; what reads it was not followed. A **Presto-flagged pet** (other than Presto himself) whose push timer falls below 41, i.e. on the 10th tick after the hit, **reverts to Presto** (`pet-presto.md`, slot 73 with 19: GROW, AddPet at its widget spot, 7 or so app draws). Presto himself (id 19) is just pushed.

**Timeline of one punch** (tick 0 = the tick of the click; the click runs in input handling before the object updates, so Walter's tick-0 update already sees the glove and the punch timer 40):

| Tick | Walter `+0x1c8` after his animation | Glove |
|---|---|---|
| 0–4 | 39..35 | waits (timer was 40..36 > 35 when it looked): no move, no countdown, no hits; still at (−200, −200) |
| 5–8 | 34..31 | life 29..26, reach 0 |
| 9–13 | 30..26 | life 25..21, reach 30 |
| 14–21 | 25..18 | life 20..13, reach 60 |
| 22–33 | 17..6 | life 12..1, follows him, no hits |
| 34 | 5 | life 0: deleted by Walter's update |
| 39 | 0 | |
| 40 | | next punch allowed (if not turning or tired) |

The glove positions itself from Walter's widget position, which his update only refreshes at its end (step 13), so it trails his true position by one tick.

So a punch has **17 hit ticks**, each object is hit at most once per glove (the 50-tick push outlasts it), and punches can come at most every 40 ticks. If Walter is removed while the glove lives (Presto's change, or any pet removal), his dtor (`0x004ef750`) deletes the glove with him.

The hits happen inside Walter's update (the `+0xb4` pass, `sim/board.md#tick-order`): an object updated earlier in the tick (guppies, aliens) starts moving next tick; a FishTypePet later in `+0xb4` moves in the same tick.

`traced` by `walter_punches` (`walter_01`) for the 5-tick wait, reach, hit window, the 15/10 push from the global stream, guppy coin clocks, and the Presto revert on the 10th tick (the reverting pet's update returns at step 7, before it moves). Pets other than Itchy/Presto in reach are `static`.

## Animation (`0x004dfc50`), id 23

1. **Turn trigger:** as Itchy's (facing > 0 and vx < 0 → turn 20; facing < 0 and vx > 0 → turn −20), except that **while the glove exists** (`+0x24c` set) Walter doesn't turn: **vx is set to 0** instead.
2. Turn steps toward 0; reaching 0 clears `+0x1c8` (Itchy's rule; a turn can't be in progress when a punch starts, since the click needs turn 0, and step 1 prevents a new turn while the glove lives).
3. While turning: cel = `9 − turn/2` / `turn/2 + 9` (Itchy).
4. Not turning, punch timer `+0x1c8 ≥ 1`: `t = +0x1c8 − 1` (stored). t 30..39 → cel = `9 − (t + 1 − 21)/2` (0 rising to 4); t 10..29 → **cel 4** (held); t 0..9 → cel `t / 2` (4 falling to 0). The swim cycle doesn't advance.
5. Not turning, `+0x1c8 = 0`: swim cycle `+0x1b8` += 1 (speed bucket `+0x1bc` < 2) or 2, 0 when it exceeds 39; cel = `cycle / 2` for cycle < 20, else `|cycle/2 − 19|`: a ping-pong 0..9..0 over 40 steps (resources.xml: `anim="pingpong"`).
6. Facing follows vx (guppy rule).

No RNG.

`traced` by `walter_punches` (`walter_01`).

## Drawing (`0x004f8610` → `0x004dfa00`)

Order within Walter's draw:

1. **The glove first** (behind him), if `+0x24c` is set: translate to the glove's widget position, call its draw (`0x004e1000`), translate back.
2. Presto-flag colour set-up (shared).
3. **Body** `0x004dfa00`:
   - **`IMAGE_ZZZ`** (`0x005e8b44`) at (30, −15), or (50, −15) when mirrored, while `0 < +0x264 < 335` (i.e. from the 26th tick of being tired).
   - `IMAGE_WALTER` (global `0x005e8e50`; `resources.xml` `walter`, 10 × 3 cels of 80 × 80), src `(cel · 80, row · 80)`: row 1 while turning (mirrored iff turn > 0); else row 2 while `+0x1c8 > 0` (punch), else row 0; rows 0/2 take `Fish::Draw`'s guppy mirror choice. No alien rows.
4. Shared tail: Presto puff and label when flagged, drop-in flash.

Type-0 shadow.

**Glove draw** (`0x004e1000`), with `d = |life − 15|` (0 at full stretch, 15 at start):

- Arm: `a = 9 − d`; skipped if `a < 1`; drawn with cel `max(a, 6)` (so cel 6 for d 3..8, then 7, 8, 9 as d → 0): `IMAGE_WALTER` row 2 src `(cel · 80, 160, 80, 80)` at (−3, 2) unmirrored (left) or (83, 2) mirrored (right).
- Fist: `IMAGE_WALTER` row 2 **cel 5** src `(400, 160, 80, 80)` at (`6d − 20`, 0) unmirrored (left) or (`100 − 6d`, 0) mirrored (right).

Relative to Walter (left glove at `wx − 60`): the fist goes from `wx + 10` (d 15) out to `wx − 80` (d 0) and back; right glove (`wx − 20`): from `wx − 10` out to `wx + 80`. During the 5 waiting ticks the glove widget is still at (−200, −200), so nothing of it is visible on screen.

`static`.

## Deviations

- **Hit order is heap order.** The glove walks the master set in pointer-address order, and each hit draws one global `grand()` that decides 15 or 10. With two or more objects hit in the same tick, which object gets which draw depends on heap addresses: an implementation artifact. The sim walks guppies, then carnivores, then the fish pets in `+0xb4`. Replays try the reverse order when the capture's pushes say so (`ArtifactPlan::glove_reversed`, replay-only; `walter_01` has one such tick, a guppy and a pet hit together).

`traced` (`walter_punches`).

## Open questions

- Everything is static. First to capture: the click gate and direction, the 5-tick wait, the hit window and reach, the 15/10 push from the global stream, the coin-clock jump on guppies, tiring after 5 punches (sink, ZZZ, clicks falling through), the Presto revert.
- The raw `+0x28` age test (Presto pets younger than 30 updates can't be punched): `+0x28` is the base widget update counter; not checked whether anything else writes it.
- Whether Ultra, Gekko and types `0x22`–`0x24` run the `+0x20c` push step in their own updates (the guppy and Oscar do, via `Fish::Update`); if not, a hit only changes their clock.
- Readers of the misc timer `+0x210` (set to 180 by a hit): maybe a dazed look or a block on eating; not followed.
- The byte `+0xf4` gating hits, and the `+0x11c` / board `+0x90` queue removal (emotes).
- Widget slot 38 call on tiring (`0x0046e370`) and whether `+0x7c` is the hand cursor.
- The mouse-inset hit rect (left/right insets, and whether the widget manager uses `0x0046e840`).
- Types `0x22`–`0x24` (the special fish) and Ultra/Gekko: what their `+0x220/+0x224` are, so what the `k = 5` clock change does to them.
- The `+0x230 == 0 && id == 0x15 && +0x240 > 0` test in `0x004fb680` (near line 184 of Ghidra's output) belongs to **Amp's zap** (it picks unflagged Nostradamus noses), not to the glove.
- Board pet counter `board+0x1a0` (id 23): readers not looked for.

`static`.

## Recording

A pet zoo on tank 1-2 with **Walter and Presto** (Walter's shells unlock by the CE profile edit, `screens/bonus-screen.md`), in one capture chunk keeping the RNG header and the **global** MT state (every hit draws `grand()`). A few guppies so there's something to punch. No timer pushes are needed: every part is click-driven.

- **Clicks on Walter:** punches facing left and right, some with a guppy in reach (push and the coin 5 ticks later; best with a guppy already old enough to drop coins), some with nothing in reach; a click during a punch and one during a turn (both must fall through to the tank, dropping food).
- **Tiring:** the 5th punch, then 360 ticks of sinking with ZZZ; click him while tired (falls through). The punch counter is only checked in Adventure-style play, not the Virtual Tank.
- **Presto:** punch Presto himself (pushed, stays Presto). Then change Presto into a fish pet (e.g. Itchy), wait at least 30 ticks, punch it: it should revert to Presto on the 10th tick after the hit. Optionally change Presto into Walter and back to check the `+0x260`/`+0x264` hand-over (do it after 2–3 punches).
- Keep one hit per tick where possible (one guppy in reach at a time), so the address-order deviation doesn't come up; one deliberate double hit shows it.

Vert, Shrapnel, Amp (k = 5) and Nostradamus (k = 10) in reach would show the clock jump on pets; optional, in a later zoo.

`static` (a plan).
