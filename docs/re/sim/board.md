# Board & game-object model

Status: per section. The update order for the object kinds on tank 1 is `traced` (every 1-1/1-2 replay depends on it); the mapping for other kinds is `static`.
Addresses are inner-PE VAs. Offsets are raw `this`-relative.

## Architecture in one paragraph

Every in-tank entity is a `Sexy::GameObject`, which **is a `Sexy::Widget`**. Objects are added as top-level children of the `WidgetManager` (`board+0x10`), not of the Board. The Board keeps **typed vectors** of object pointers for game logic, plus one master list. Each tick, `WidgetManager` walks its child list and calls `UpdateAll` → `Update` on each widget in list order. That makes **update order = widget z-order (back to front)**. The Board controls that order by rebuilding the z-order from its typed vectors in a fixed sequence (`Board::RebuildZOrder` @ `0x0053aa40`).

`traced` for tank-1 objects (update order = z-order is what makes the replays match).

## Class hierarchy (from RTTI)

```
Widget
└─ GameObject                     vtable 0x00596844, 81 slots
   ├─ Fish                        vtable 0x00597d54, 91 slots (adds 10 virtuals)
   │  ├─ Oscar, Ultra, Gekko, SylvesterFish, BallFish, BiFish, FishTypePet
   ├─ Alien, Bilaterus (+BilaterusHead, BilaterusBone)
   ├─ Breeder, Grubber, Penta, Larva
   ├─ Coin, Food, Shot, Missle, Warp, Shadow, BoxingGlove
   ├─ DeadFish, DeadAlien
   └─ OtherTypePet
Board : Widget, ButtonListener    vtable 0x005a8404
```

`static` (RTTI).

## Object type codes (`GameObject+0x8c`) → Board list

Each constructor stores a type code at `+0x8c`. `Board::AddGameObject` (`0x00542ee0`) routes on it with a switch (jump table `0x005431ec`) and pushes the object into one typed vector:

| Code | Class | Board vector | Likely meaning (TODO confirm) |
|---|---|---|---|
| `0x00` | `Fish` | `+0xa0` | Guppy |
| `0x05` | `Oscar` | `+0xa4` | Carnivore |
| `0x06` | `Ultra` | `+0xd4` | Ultravore |
| `0x07` | `Gekko` | `+0xcc` | Beetlemuncher? |
| `0x08` | `Penta` | `+0xd0` | Starcatcher |
| `0x09` | `Grubber` | `+0xc4` | Guppycruncher |
| `0x0a` | `Breeder` | `+0xc0` | Breeder |
| `0x14` | `OtherTypePet` | `+0xb0` | Non-fish pets; also `++board[+0x144 + 4*petId]` (petId < 24) |
| `0x15` | `FishTypePet` | `+0xb4` | Fish-shaped pets; same pet counter (petId at `+0x234`) |
| `0x16` | `Alien` | `+0xbc` if `alien+0x1ec == 0x14`, else `+0xb8` | Aliens; subtype `0x14` is special (boss?) |
| `0x17` | `Bilaterus` | `+0xf4` | Bilaterus alien |
| `0x19` | `Coin` | `+0xa8` / `+0xe8` / `+0xec` by coin subtype `+0x1a0` | Coins, pearls, treasure… |
| `0x1a` | `DeadAlien` | `+0x9c` | Death animation |
| `0x1b` | `DeadFish` | `+0x98` | Death animation |
| `0x1c` | `Food` | `+0xac` | Pellets/potions |
| `0x1d` | `Larva` | `+0xc8` | Beetle (dropped by Guppycruncher) |
| `0x1e` | `Missle` | `+0xe0` or `+0xdc` (by `Missle::FUN_004d80c0`) | Alien missiles |
| `0x1f` | `Shadow` | `+0xe4` | Floor shadows |
| `0x20` | `Shot` | `+0xd8` | Player/pet shots |
| `0x21` | `Warp` | `+0xf0` | Alien warp-in effect |
| `0x22–0x24` | `SylvesterFish`, `BallFish`, `BiFish` | `+0xf8` | TODO identify |

Codes 1–4 and 0x0b–0x13 have no case (unused or non-board objects).

Every added object is also appended to a master container at `board+0x104` (`0x00540ec0`). Unless the type is 0x1e–0x21 (missile/shadow/shot/warp), adding also sets the flag `board+0x2a7 = 1`.

`traced` for guppies, food, coins, pets, aliens, shots, warps, dead aliens, dead fish, shadows (captured from these lists). Other rows `static`.

## Tick order

Per logic tick (28 ms, see `framework/timing.md`):

1. `WidgetManager::UpdateFrame` walks its children in list order. For each, `UpdateAll` runs `Update()` (vtable slot 22, `+0x58`) once per WM tick (it guards on `mLastWMUpdateCount`), then recurses into that widget's own children.
2. **`Board::Update`** (`0x00547610`) runs first among the game widgets (TODO: confirm Board precedes all objects in the WM list). It handles timers, alien-wave countdown, spawns, ambient RNG, mode logic, and finally calls `Board::RebuildZOrder` (`0x0053aa40`).
3. **Each GameObject** then gets its `Update()` in the rebuilt list order.

**Paused board** (dialogs, focus loss): the board clock `+0x28` still advances by one per update, and nothing else runs. When `Board::Update` raises a dialog itself (the 1-2 "DANGER!" hint at countdown 276), the objects don't update on that tick either. Confirmed in `tank1-2_full_02`.

**No fish left** (`0x00548e29`, `0x005392a0`) is checked in `Board::Update` right after the wave countdown, so it also comes before the objects update. When the last guppy dies (eaten or starved) during tick *n*'s object updates, tick *n + 1* still runs the board's own part (ambient draw, wave countdown − 1). Then it raises GAME OVER (dialog 0x11, or 0x10 in the tutorial) and pauses, and the objects don't run. `traced`:
- `tank_1_2_from_demo`: the last guppy eaten at update 7012, the dialog at 7013 with the countdown one lower and one more draw;
- `tank_1_4_from_start`.

Removal during iteration is safe: `WidgetContainer` keeps the iteration cursor in the container (`+0x18/+0x1c`) plus a "removed current" flag (`+0x14`).

`traced` (paused-board behaviour and the dialog-raising tick are covered by the 1-2 replays).

### `Board::RebuildZOrder` (`0x0053aa40`) order, back → front

`RebuildZOrder` first calls `wm->MarkAllDirty()`, then `wm->BringToFront(obj)` (WM vtable slot 11, `+0x2c`) for each object in this sequence. Each call moves the object to the end of the list, so the first one called ends up furthest back and updates first:

```
board+0xfc (BoardOverlay)   ← TODO: verify it's really at the back
+0xe4 Shadow
+0x98 DeadFish
+0x9c DeadAlien
+0xe8 Coin(sub)
+0xf0 Warp
+0xd0 Penta
+0xc4 Grubber
+0xac Food
+0xa0 Fish (guppy)
+0xa4 Oscar
+0xc8 Larva
+0xcc Gekko
+0xd4 Ultra
+0xc0 Breeder
+0xf8 SylvesterFish/BallFish/BiFish   (special handling when +0x1a0 == 4, and board+0x110)
+0xb8 Alien         (pass 1: only objects with +0xfc == 0 or +0x100 != 0)
+0xf4 Bilaterus     (pass 1)
+0xb0 OtherTypePet  (Niko, id 1 without the drop-in flag, goes behind the overlay instead: back-most; pet-niko.md#update-order)
+0xb4 FishTypePet
+0xdc Missle
+0xe0 Missle(alt)
board+0x110         (single widget; also slot 17)
+0xb8 Alien         (pass 2)
+0xf4 Bilaterus     (pass 2)
+0xbc Alien(sub 0x14)
+0xd8 Shot
+0xa8 Coin
+0xec Coin(sub)
board+0x118 …       (single widgets, HUD overlays)
```

**Objects created during the object pass update in it.** A new object joins the end of the widget list, and the widget manager's walk reaches it in the same tick: a guppy's corpse (`fish-guppy.md#dead-fish`) and an alien's corpse made in the alien's update (or by a pet's stab) both show one tick of ageing on their first snapshot. A corpse made by a click (before the walk) also updates that tick. `traced` (`tank_1_4_from_start`, dead alien life 124 on the kill tick).

**Drawing follows the same list.** The Board widget itself (backdrop, `IMAGE_MENUBAR`, the tank label; `Board::Draw`) and the shop buttons and money label are widgets that `RebuildZOrder` never brings forward, so they sit behind every game object. A collected coin gliding to the counter is drawn **over** the menu bar. `confirmed` by `tank13_carnivore_01` frame 1, a gold coin at y 39 over the bar's lower edge; the frame test `tank_1_3_carnivore_frames_match_original` has it under test. The overlays `+0xfc`/`+0x100` are `BoardOverlay` widgets that call back into the Board (`0x00540470`): mode 0 draws pet/alien extras (`0x0053bd90`), mode 1 the screensaver's shell banner (`0x005400f0`). `static`.

Per-element filter in most loops: an object is brought to front unless `obj+0xfc != 0 && obj+0x100 == 0`. This looks like a "hidden / pending removal" check. TODO.

**Determinism:** within each vector, order is insertion order (`std::vector` push_back). Objects of one type always update before the next type in the sequence above, and **this is the RNG call order**. Our sim must keep the same per-type vectors (or an equivalent ordering) and walk them in this sequence.

`traced` for the kinds present on tank 1; the rest of the sequence is `static`.

## Board fields seen so far

| Offset | Meaning |
|---|---|
| `+0x10` | `WidgetManager*` (parent of all game objects) |
| `+0x98…+0xf8` | Typed object vectors, see table (each `std::vector<GameObject*>*`) |
| `+0xfc` | `BoardOverlay*` |
| `+0x104` | Master object container |
| `+0x138`, `+0x13c` | Resource/name strings (ctor) |
| `+0x144[24]` | Per-pet-ID counters |
| `+0x2a7` | "Objects changed" flag |
| `+0x48c` | High-water mark of guppy count |

The `Board::Update` body (timers, wave countdown `+0x234` from 3000, the alien-type selection per tank/level, ambient bubbles) is substantial. Split it out into `sim/board-update.md` when we implement M3.

`confirmed` (CE).

## Live observations (CE read-only capture, 2026-10-21)

From `tools/ce_capture.lua`, tank 1-1, two guppies (`traces/original/tank1-1_live_02.trace.jsonl`, 200 consecutive ticks with none missed):

| Fact | Value | Confidence |
|---|---|---|
| Board pointer | `[app+0x730]` | confirmed |
| Money | `board+0x3f0` (int); food pellet costs **5** (180→175→170) | confirmed |
| App-RNG draws per idle tick | **exactly 1** (the `rand()%1000` ambient-bubbles check in `Board::Update`) | confirmed |
| Periodic extra draws | **+2 every 21 ticks** with 2 guppies (ticks 3296, 3317, 3338, …), probably 1 draw per fish on a 21-tick timer | confirmed (count), guess (cause) |
| Food drop | money −5, `food` list +1, extra RNG draws on the drop tick | confirmed |
| Global RNG (`gMTRand`) | untouched during tank play (`mti` stayed 6) | confirmed |
| Shadows | one `Shadow` object per guppy (`board+0xe4`) | confirmed |

Per-tick app-RNG draw counts are a cheap, strong fidelity check. Our sim must reproduce this exact sequence.

`confirmed` (see the table). The "global RNG untouched" row only holds without aliens (see `alien-sylvester.md#6`).

## Fish spawn (`Board::AddRandomGuppy` @ `0x00546d70`)

As an example of call order and RNG consumption:
1. `x = rand() % 520 + 20`, `y = rand() % 265 + 105` (app MTRand; x drawn first)
2. `new Fish(x, y)` (0x230 bytes)
3. `fish.speed? (+0x1c) = rand() % 5 + 18` (double)
4. `fish+0xb4 = rand() % 10 + 45`; `fish+0xc = <const 0x005966b0>`; `fish+0x30 = 30`
5. `AddGameObject(fish)`, `wm->AddWidget(fish)`, update the guppy high-water mark, `FUN_00544800(fish)` (shadow creation), `RebuildZOrder()`

So these four draws come first, in this order. The full cost with the Fish ctor, shadow and splash is in `fish-guppy.md#spawn` (11 draws, 12 with the purchase splash; `traced`).

`traced` (see `fish-guppy.md#spawn`; note it costs 11–12 draws with the shadow and splash, not 4).

## Shadows

`Shadow` (type 0x1f, list `+0xe4`, drawn first so it sits behind every object). It is created by `0x00544800(owner)` → `0x00544370(shadowType, owner)` when the board holds fewer than 40. The shadow type depends on the owner:

| Owner (type `+0x8c`) | Shadow type |
|---|---|
| guppy 0, 5, 7, 10, 0x1b, 0x23, 0x24 | 0 |
| 6 (Ultravore), alien 0x16 (kind ≠ 0x14), 0x22 (`+0x1a0 ≠ 1`) | 2 |
| 8, 9; pet 0x14 with id ≠ 1, 5 (**Stinky: 1**) | 1 |
| pet 0x14 id 5 | 0 |

Fields (raw): owner `+0x158`, type `+0x15c`, darkness `+0x160`, scale `+0x168` (f64, 1.0). The widget is 80×40.

- **Ctor** `0x004ec4f0`: x −200, **y = `rand(5) + 410`** (the one app draw every spawn pays), darkness = `(owner.iy − 50) / 2`.
- **Update** `0x004f3d00` (unpaused only, no RNG; it runs before its owner moves, so it lags one tick):
  - darkness = `(owner.iy − 50) / 2`;
  - x = owner.ix, plus 40 for type 2;
  - types 0 and 2: `y = 415 − (max(0, 370 − owner.iy) · 30) / 370` (ints). Type 1 keeps its ctor y.

  Confirmed in `tank1-2_full_02`: Stinky's shadow at (ix, 412) with darkness 160, and guppy shadows at y 398..408.
- **Draw** `0x004e31f0`: colour = the tank's shadow colour (`0x00537d30`: tank 1 → (66, 48, 41)), alpha:
  - `clamp(darkness · scale, 0, 150)` for types 0/1;
  - `clamp(2 · darkness · scale, 0, 255)` for type 2.

  Then:
  - Type 1: `IMAGE_SHADOW` at (0, 0).
  - Otherwise, on the 3D renderer: `f = (shadow.iy − owner.iy) / 100` (as f32), clamped to [0.9, 1.3], **+0.4 for type 2**, then `g = f − 1`. Draw the image stretched to `(−dx, −dy, 80 + 2dx, 40 + 2dy)` with `dx = trunc(80·g)`, `dy = trunc(40·g)`: fish high in the tank cast larger, fainter shadows. Without 3D: unstretched.
  - Skipped when the owner is hidden (`+0xfc`) and of type 8/9.

Owners only ever lose their shadow when removed. Shadows are cosmetic apart from the ctor draw.

`traced` for the ctor draw; drawing `confirmed` by the frame diff (guppies and Stinky).

## The global RNG stream in normal play

`Sexy::Rand` (global `gMTRand`) has 74 call sites. On tank 1-1 with guppies, food and coins only, every global draw comes from the **cosmetic bubble systems**:

- `board+0x11c`: the ambient bubble emitter (`0x005110e0`, run from `Board::Update`). Each tick it advances its bubbles (one `Rand() & 1` wobble per bubble, `0x004ffec0`) and removes those above the surface. With probability `emit_pct %` (`Rand() % 100 < +0x28`) it spawns a bubble at a random x in its range (`0x00510fe0`). New bubbles draw 5–6 more (`0x004ffdc0`: size `& 3`, re-rolled once on 3; y `400 + %6`; x `150 + %22`; speed from a 6-entry table). A second list at `+0x4c` (stream bubbles, `0x00511010` / `0x004fff40`) draws colour and position.
- `board+0x140`: the side bubble streams (`0x00511680`), refilled to their quota with `Rand() % 480` positions (`0x005114a0` → `0x00511580`).
- `Board::Update` also draws in level-start/background selection (`0x00547776`) and the surface-splash burst (`0x005478f0`: `5 + %5` splashes, each `%41` and `%10`).

None of these feed back into gameplay. Gameplay sites on the global stream are only: guppy startle exit (`Fish::Update`, mode 123), aliens, pets, missiles, and Virtual Tank fish init. So the sim can treat the global stream as independent of the app stream. The replay harness compares only the app stream's `mti`, and the global stream is resynchronised from each capture's header. **Modelling the bubble systems exactly** (float positions, list order) is needed only before the first global-stream gameplay draw that follows bubble activity, i.e. alien levels and startles. It's tracked as an open item for M3's Sylvester work.

`static`: the bubble systems aren't modelled and the global `mti` isn't compared.

## Backdrop

`Board::DrawBackdrop` @ `0x00538e50`, called first by `Board::Draw` @ `0x0053f5f0`. Normal game, 3D renderer on (`[app+0x36c]+0x38`, which is the case in our captures). `confirmed`: a model built from these rules matches a screenshot of the original within 2 levels per channel on average.

1. **Screen shake:** while `+0x2b4` is in 1..19 and the board isn't paused, the whole backdrop is drawn offset by `(rand() % 5 − 2, rand() % 5 − 2)` using the C `rand()` (not MTRand), over a black fill. TODO: what sets `+0x2b4`.
2. `IMAGE_AQUARIUMn` (n = tank background `+0x3e8`) at (0, 0).
3. **Wave strip** (`0x00500c90`, drawn at y = 88, additive): cel `c` = `IMAGE_WAVE`'s animation (rows 12, ping-pong, framedelay 4, begindelay 6) at `t = +0x444`, the level tick. In order:
   - `IMAGE_WAVESIDE` row `c` at x 0;
   - `IMAGE_WAVE` row `c` at x 160 and at x 320;
   - `IMAGE_WAVESIDE` row `c`, mirrored, at x 480.
4. **Light bands** (additive, colourised white): `IMAGE_TANKLIGHTING` (640×80) at y = 365.
   - Band 0 (`+0x3d8`) at alpha 40.
   - Band 1 (`+0x3dc`) at alpha 100.
   - Band 2 (`+0x3e0`) at alpha 70, mirrored.

   Each band is drawn twice, at x = trunc(pos) and at x = trunc(pos − 640.0), so it wraps around.
5. **Foreground cut-out:** `IMAGE_TANKMASKn` at (0, 365), normal blending. At load (`LoadingThreadProc` @ `0x005510b0` → `CreateMaskedCutout` @ `0x00503120`) this image is replaced by the 640×80 strip of `AQUARIUMn` at y = 365, with the mask as its alpha. So the bands only show where the mask is dark.

The three band positions are floats.
- **Init:** the Board ctor (`0x00542150`) sets them in order to `MTRand % 640`. These are the "3 app draws" of `sim/level-start.md`; the earlier note naming them `+0x34c..0x354` was wrong.
- **Each unpaused tick** (`Board::Update` @ `0x0054805d`, right after the money-flash decrement): pos0 −= 1.6, pos1 += 1.8, pos2 += 2.2. Each step is computed in double precision and stored back as f32.
- **Wrap:** a position above 640 becomes 0; one below 0 becomes 640. `confirmed` in `traces/frames/tank12_start_01`: 629.8 → 57.2 over 31 ticks (+2.2 per tick, through 0), and 9.6 → 601.6.

## Message line

`Sexy::MessageWidget` (ctor `0x00534490`, vtable `0x005a6f5c`), held at `board+0x118`. It is a single text line along the bottom of the tank: "Welcome to the next level!", "Click on coins for extra money!", and so on.

| Field | Meaning |
|---|---|
| `+0x30..+0x3c` | rect (90, 445, 460, h), h = outline font height + 6 |
| `+0x8c` | `std::string` text |
| `+0xa8` | text colour (180, 250, 90) |
| `+0xb8` | outline colour (0, 75, 0) |
| `+0xcc` | outline font `FONT_CONTINUUMBOLD14OUTLINE` (`ContinuumBold14outback`) |
| `+0xd0` | **timer**: ticks left; drawn while > 0. The ctor sets 185 |
| `+0xd4` | flash flag |
| `+0xd8` | id of the last one-time message (−1 = none) |

- **Update** (slot 22 @ `0x00530800`): when the board exists and isn't paused (`board+0x94`), the timer decreases by 1 while > 0.
- **Draw** (slot 26 @ `0x00530830`), while the timer > 0. Let `asc` be the outline font's ascent (slot 1) and `fh` its height (slot 4).
  - Baseline y = asc + (h − fh) / 2.
  - x = (460 − StringWidth(text)) / 2 + 2 (StringWidth from the outline font).
  - Both are C integer divisions, relative to the widget at (90, 445).
  - Order: the text in the outline font and outline colour, then in `FONT_CONTINUUMBOLD14` and the text colour, at the same position.
  - **Flash:** when the flag is set, nothing is drawn on ticks where `timer % 32 ≥ 27` (a blink).

**Show** = `Board::ShowMessage(text, flash, id)` @ `0x0053e950`:

- `id = −1`: set the text, flash = `flash`, timer = **500** if flashing, else **100**.
- `id ≥ 0`: only if the board's one-time byte `board+0x4b0+id` is still 0. Set the text, flash, and timer **185** (a flashing one also passes through 500 first, then is overwritten). Then set the byte to 1.
- Either way, `+0xd8 = id` and the colours are reset.

The one-time bytes `board+0x4b0..+0x4e5` are zeroed by InitLevel. The three hint *dialogs* (`framework/dialog.md`) use bytes 3, 4 and 0x11 of the same table.

Tank-1 callers. "1-1" means the tutorial predicate `0x00537b80` (tank 1, level 1, `profile+0x59` clear). "Idle" means only when the timer is ≤ 0.

| Where | Condition | id, flash | Text |
|---|---|---|---|
| `PopulateLevel` end (`0x00549be1`), Adventure | 1-1 | 0, no | Welcome to the Insaniquarium! |
| 〃 | 1-2, `+0x59` clear | 0x10, no | Welcome to the next level! |
| 〃 | 2-1, `+0x59` clear | 0x13, no | Welcome to the second tank! Now things get tricky! |
| `Board::Update` (`0x0054913b`), Adventure, `+0x444 == 150` | 1-1 | 1, no | Here are your first fish! Take good care of them! |
| 〃 | 1-2 (`0x00537bb0(1,2)`) | 0x14, no | On this level you will meet your 1st alien opponent! |
| `AddFood` (`0x0054337c`), food limit reached | 1-1, idle | 6, no | You can only drop 1 food pellet at a time for now |
| 〃 | 1-2 (`0x00537bb0(1,2)`), idle, slot 3 revealed (`+0x3a7`) | 0xd, no | Upgrade Food Quantity to drop more food at once! |
| `Fish::UpdateCoin` (`0x004f1502`), after each coin-drop attempt | 1-1 | chain 5 → 0x19 → 0x1a, yes | Click on coins for extra money! |
| `Fish::UpdateHunger`, hunger == −1, announcing fish | – | chain 8 → 0x17 → 0x18, yes | Fish are hungry! Click to drop food! |
| 〃 hunger == −200 | – | 0x15, yes | Fish are REALLY hungry! Click to feed! (+ sound 0x106); id = byte 0x15 |
| 〃 hunger == −400 | – | 0x16, yes | Fish are about to die of hunger! Click on aquarium! |
| `Fish::OnStageUp` stage 1, first reveal of slot 0 | 1-1, idle | 9, no | Your fish has grown! Good work! |
| 〃 stage 2, first reveal of slot 11 | 1-1, idle | 7, no | Buy 3 egg pieces to complete level! |
| 〃 stage 2, slot 2 not yet revealed | 1-2, `+0x59` clear | 0xb, no | Upgrade Food Quality to make food more nourishing! |
| `Purchase` slot 0 | 1-1 and money < price | 0xe, no | You can't afford new fish yet! Collect more money! |
| `Purchase` slot 2, first reveal of slot 3 on 1-2 | `+0x59` clear | 0xc, no | Upgrade Food Quantity to drop more food at once! |
| `Purchase` slot 3 | tank 1, quantity was 1 | −1, no | Hold down mouse button to Auto-Feed! |
| `BuyEggPiece` | 1-1, counter 1, idle | 0xa, `EDI` | Collect 2 more egg pieces to finish level! |

**Chains:** the coin and hunger messages show up to three times per level.
- Coin: if byte 0x19 is set, show with id 0x1a; else if byte 5 is set, show with id 0x19; else show with id 5.
- Hunger: if byte 0x17 is set, show 0x18; if byte 8 is set, show 0x17; then show 8. All three calls are made and only unused ids take effect, so the 1st, 2nd and 3rd times use 8, 0x17 and 0x18. `static`; the 1-2 rows are `confirmed` by `traces/frames/tank12_start_01` (both 1-2 texts on screen; the live widget showed id 0x14 for the alien line).

`confirmed` for 1-1 by `traces/original/tank1-1_pres_01.trace.jsonl` (the whole level, with `board.msg`). The replay test `tank_1_1_from_start_with_messages` matches text, timer and blink on every tick: welcome, first fish (tick 150), grown, the coin message three times (ids 5, 0x19, 0x1a) and the egg-piece hint. A new message's timer shows 184 on its first tick: the widget updates after the board in the same tick.

### The alien warning line

At wave countdown **275** (the warning tick, `Board::Update` @ `0x005486e1`), before the alien music starts and the spawn position is drawn, the board writes the message line directly: it doesn't go through `ShowMessage`, so no one-time byte is involved and it shows on every wave.

- Text by the wave's alien kind (`+0x2bc`):
  - kinds 9–12: "MULTIPLE ALIEN SIGNATURES DETECTED" (or, on 4-4 without byte `+0x4de`, "FOURTEEN ALIEN SIGNATURES DETECTED");
  - 4 "ALIEN SIGNATURE TYPE-G DETECTED", 5 "…TYPE-D…", 6 "…TYPE-U…", 7 "…TYPE-P…", 8 "…TYPE-II…";
  - 0x15: "%s OF DOOM APPROACHING" with an ordinal of the times beaten;
  - otherwise (kinds 1, 2, 3 on tank 1): **"ENEMY APPROACHING"**.
- Flash off, **timer 274** (`0x112`).
- Colours: text **(255, 50, 50)**, outline **(100, 20, 20)**.

`traced` for kind 2 (`tank1-3_full_01`: "ENEMY APPROACHING", timer 273 on the first snapshot after the warning, both waves) and kind 9 (`tank1-5_full_01`: "MULTIPLE ALIEN SIGNATURES DETECTED"). Kind 1 (1-2) is `static` (older 1-2 captures don't record the message line), but it is the same branch. The colours are `static`.
