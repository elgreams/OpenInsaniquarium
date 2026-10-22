# Hatch screen (`Sexy::HatchScreen`)

Status: `draft`, **static only**. Nothing here has been checked live yet.

This is the "You have found:" screen that appears after the last egg piece is bought on an Adventure level. An egg in an incubator cracks, the shards fly apart, and the new pet is revealed with its name and description. Then a button takes you to the next level.

Class `Sexy::HatchScreen` (Widget + WidgetContainer + ButtonListener sub-object at `+0x88`). Vtable `0x005a0904`, ButtonListener vtable `0x005a08e0`, object size `0xb8`. (Ghidra's `HatchScreenOverlay::HatchScreenOverlay` @ `0x0051f450` is actually the **HatchScreen** ctor. The real `HatchScreenOverlay` is a small child widget, described [below](#overlay-widget).)

| Function | Address |
|---|---|
| ctor `HatchScreen(app, petId)` | `0x0051f450` |
| dtor | `0x00517930` |
| AddedToManager (slot 20) | `0x005179e0` |
| RemovedFromManager (slot 21) | `0x00517a30` |
| **Update** (slot 22) | `0x00520760` |
| **Draw** (slot 26) | `0x0051f770` |
| DrawOverlay (slot 44, called by the overlay widget) | `0x00517a80` |
| ButtonPress(id) (listener slot 1) | `0x0051a6f0` |
| **ButtonDepress(id)** (listener slot 2) | `0x00517c70` |
| `App::ShowHatchScreen(petId)` | `0x0054be80` |
| `App::RemoveHatchScreen` | `0x0054adf0` |
| `App::NextLevel(resume, help)` (our name) | `0x00552380` |
| `App::StartLevel` | `0x0054cbf0` |

Notation: `app` = WinFishApp. `profile` = `[app+0x8b8]`. `mode` = `app+0x87c` (0 = Adventure). Board `tank`/`level` = `board+0x3c8`/`+0x3cc`. Profile `tank`/`level` = `profile+0x1c`/`+0x20`. One tick = one widget `Update` = **28 ms** (`framework/timing.md`).

## 1. From the last egg piece to the hatch screen

Everything happens **synchronously inside the mouse-down that buys the 3rd egg piece**. There is no delay and no animation on the board. `Board::BuyEggPiece` @ `0x00545960` (see `sim/shop-bar.md#egg-pieces`, steps 1–7) continues as follows once the counter reaches 4. `static`.

1. `board+0x3fc = ElapsedSeconds()`. `board+0x4ee = 0` ("level in progress" off).
2. `Board::SettlePendingCoins` (`0x0053c2d0`): coins already flying to the counter are banked. This affects only the board's money, which is thrown away (see §5), plus the score/time record.
3. Mode 4 (Challenge) and the `profile+0x59` ("game completed") replay paths branch off here. They're out of scope (see [Other paths](#other-paths)).
4. Remember `done = profile+0x59` (read **before** advancing).
5. **Record the level** `0x00514510(tank, level, profile, secs)`. For levels other than 5-1, `profile+0xb8 + 4*idx` keeps the **best (lowest) time** in seconds (`0x00501230`, with `-1` = none). The app high-score table at `app+0x8ac` is also updated. `idx = (tank−1)*5 + (level−1)`. No RNG.
6. **Advance the profile** `0x00501370` (see [Level advance](#level-advance)).
7. **Award the pet**, using the *board's* tank/level, which are unchanged:
   - Board tank 4, level 5: no pet; the hatch id is **999** ("Evil Alien Mastermind").
   - Board tank 5: id **0x13** (Presto).
   - Otherwise: id = `tank*5 + level − 6`, used only if 0 ≤ id < 24.

   Unlock = `0x00501420(profile, id, 1)`: if `profile[+id]` (byte) isn't already 1, set it and `++profile+0x18` (owned-pet count). For id 0x13, also `0x00501200(5000)`: `profile+0x48` (shells) += 5000, clamped to 0..9 999 999.
8. **`App::ShowHatchScreen(id)`** @ `0x0054be80` (its only caller):
   1. `0x0054b4c0`: kill dialogs 0x1f and 0x20 (app slot 77 = KillDialog).
   2. `app+0x884 = 1`. If a board exists, `board+0x4ee = 0`.
   3. Remove any existing hatch screen (`app+0x814`).
   4. `new HatchScreen(app, id)` → `app+0x814`. Resize it to `(0, 0, app w, app h)` = 640×480 and add it to the WidgetManager (`app+0x320`, vtable +0xc).
   5. `0x0054afd0` → `0x00514730`: **save the profile** to `userdata/user%d.dat`. This is what "Your game has been saved." refers to.
   6. `0x0054bc30`: **destroy the board**. `0x005497a0` deletes the in-progress level save `userdata\<mode-prefix><profileId>.dat`, because `+0x4ee` is 0. Then the board is removed from the WidgetManager, safe-deleted, and `app+0x730 = 0`.

So from the player's view, the click on the third egg piece instantly replaces the tank with the hatch screen.

### Pet award table (Adventure, first play-through)

| Level | id | Pet | | Level | id | Pet |
|---|---|---|---|---|---|---|
| **1-1** | **0** | **STINKY the Snail** | | 3-1 | 10 | SEYMOUR the Turtle |
| 1-2 | 1 | NIKO the Oyster | | 3-2 | 11 | SHRAPNEL the Robot Fish |
| 1-3 | 2 | ITCHY the Swordfish | | 3-3 | 12 | GUMBO the Angler |
| 1-4 | 3 | PREGO the Momma Fish | | 3-4 | 13 | BLIP the Porpoise |
| 1-5 | 4 | ZORF the Sea Horse | | 3-5 | 14 | RHUBARB the Hermit Crab |
| 2-1 | 5 | CLYDE the Jellyfish | | 4-1 | 15 | NIMBUS the Manta Ray |
| 2-2 | 6 | VERT the Skeleton | | 4-2 | 16 | AMP the Electric Eel |
| 2-3 | 7 | RUFUS the Fiddler Crab | | 4-3 | 17 | GASH the Shark |
| 2-4 | 8 | MERYL the Mermaid | | 4-4 | 18 | ANGIE the Angelfish |
| 2-5 | 9 | WADSWORTH the Whale | | 4-5 | 999 | (no pet: boss reveal) |
| | | | | 5-1 | 19 | PRESTO the Tadpole (+5000 shells) |

Ids 20–23 (BRINKLEY, NOSTRADAMUS, STANLEY, WALTER) also have hatch-screen text, but they are unlocked elsewhere (`0x005172e0`), not by egg pieces.

### Level advance

`Profile::AdvanceLevel` @ `0x00501370`, run on the profile:

1. `level += 1`.
2. If tank 4 and level == 6: tank = 5, level = 1. Tank 4 has no bonus round; you go straight to the final boss.
3. If tank 5 and level ≥ 2: `++profile+0x50` (times the game was beaten), `profile+0x59 = 1` (game completed).
4. If level > 6, or (`+0x59` and level > 5): `tank += 1`, `level = 1`.
5. `profile+0x58 = 1` ("has progress").

So **1-1 → 1-2**, and **x-5 → x-6** on tanks 1–3. Level 6 is the **bonus round**: `App::IsBonusRound` @ `0x0054b2a0` is true when mode 0 and profile level == 6, and `StartLevel` then uses `0x00537e20` instead of `PopulateLevel`. Finishing the bonus round (not this path) advances 6 → 7 → next tank, level 1. 4-5 → 5-1, then 5-1 → 5-2 sets the completed flag.

### Profile fields touched

| Offset | Type | Meaning | Written here |
|---|---|---|---|
| `+0x00 + id` | byte[24] | **pet owned** flags | unlock |
| `+0x18` | int | owned-pet count | +1 on a new unlock |
| `+0x1c` / `+0x20` | int | current tank / level | advance |
| `+0x44` | int | profile number (save-file suffix) | – |
| `+0x48` | int | shells | +5000 on 5-1 |
| `+0x4c` | int | levels started (++ in `0x0054c540`) | – |
| `+0x50` | int | times beaten | 5-1 |
| `+0x58` | byte | has progress | 1 |
| `+0x59` | byte | game completed | 5-1 |
| `+0x5a + id` | byte[24] | pets **chosen** for the next level (PetScreen) | cleared by NextLevel |
| `+0xb8 + 4*idx` | int[21] | best time (s) per level, −1 = none | record |

Correction to `sim/level-start.md`: owned flags are at `profile+0x00` (`Profile::HasPet` @ `0x00501410` reads `profile[id]`). `+0x5a` is the "chosen pets" array. `static`.

## 2. Fields

Offsets are raw `this`-relative.

| Offset | Type | Meaning | Initial |
|---|---|---|---|
| `+0x30..+0x3c` | int | widget rect | 0, 0, 640, 480 |
| `+0x55` | byte | mouse-visible (Widget) | 1 |
| `+0x58` | byte | **mouse is down on this widget** (Widget `mIsDown`, set by the WidgetManager) | 0 |
| `+0x88` | ptr | ButtonListener sub-object vtable | – |
| `+0x8c` | ptr | app | ctor arg |
| `+0x90` | `BubbleMgr*` | bubble emitter (updated, never drawn; see §6) | new |
| `+0x94` | int | **tick counter** `t` | 0 |
| `+0x98` | int | **pet id** (0..23, or 999) | ctor arg |
| `+0x9c` | int | egg jitter x, 0..3 | 0 |
| `+0xa0` | int | egg jitter y, 0..1 | 0 |
| `+0xa4` | int | boss y (999 only) | 480 |
| `+0xac` | `DialogButton*` | **Continue** button, id 99 | – |
| `+0xb0` | `DialogButton*` | **Menu** button, id 100 | – |
| `+0xb4` | `HatchScreenOverlay*` | overlay widget | – |

## 3. Construction (`0x0051f450`)

In order (`static`):

1. Widget base; set the vtables; `+0x8c = app`.
2. `BubbleMgr` (`0x00505350`, no RNG) → `+0x90`. Its emit rect is (190, 220, 258, 200) (`0x00500120`), and `0x00500180(10, 5)` sets max 10 bubbles and 5 % emit chance per tick. Then **`0x00512080` pre-warms it by running its update 500 times** (global-stream RNG, see §6).
3. `+0x94 = 0`, `+0x98 = petId`, `app+0x884 = 1`. Rect = (0, 0, app w, app h).
4. **Continue button** `+0xac` = `DialogButton` (via `0x00506820`): id **99**, listener `this+0x88`, image `IMAGE_MAINBUTTON` (3 cels: up/over/down), label **"Please Wait..."**.
   - Font `FONT_JUNGLEFEVER12OUTLINE` (the helper's default is `…10OUTLINE`).
   - Text colour (255, 240, 0); hilite colour (255, 255, 255).
   - **Mouse-visible = 0**, so it can't be clicked yet.
   - Rect **(186, 445, 264, h)**, with h = the button's own height (the image height, 29).
   - WidgetManager vtable +0x2c on it (probably bring-to-front; `guess`).
5. **Menu button** `+0xb0` = `DialogButton`: id **100**, label **"Menu"**, same image and colours, default font `FONT_JUNGLEFEVER10OUTLINE`, rect **(525, 4, 80, h)**. Hidden (`+0x54 = 0`) when the profile is at 5-2 (just after beating the final boss).
6. **Overlay** `+0xb4` = `HatchScreenOverlay` (vtable `0x005a07c4`, size `0x8c`, owner `+0x88 = this`): mouse-visible 0, 640×480.
7. `+0x9c = +0xa0 = 0`, `+0xa4 = 480`.
8. **Music:** stop all music (`0x0054b020`), then `0x0054b1a0(2, 54, 0)`. This records track (2, 54) in `app+0x88c/+0x890` and remaps it to **track 4 (`Insaniq2.mo3`), order 49** (`assets/music.md` remap table). It plays via music interface slot 2 (`BassMusicInterface::PlayMusic(id, order, noLoop=false)` @ `0x00498d10`): **looped, no fade**.

`AddedToManager` (`0x005179e0`) adds the Continue button, the Menu button and the overlay to the WidgetManager, in that order. `RemovedFromManager` removes the Menu button, the Continue button and the overlay. The dtor deletes the BubbleMgr, both buttons and the overlay.

## 4. Update (`0x00520760`), once per tick

Here `t` is the value of `+0x94` **before** this tick's increment.

1. BubbleMgr update (`0x005110e0`): global-stream RNG only.
2. MarkDirty (slot 17).
3. **App RNG draw A:** `+0x9c = Next() & 3`. **Draw B:** `+0xa0 = Next() & 1`. These happen **every tick, unconditionally.**
4. **Skip:** if `+0x58` (the mouse is held down on the hatch screen itself, not on a button) and `t < 140`: `t = 140` (the counter, `+0x94`).
   Otherwise, if `t == 141`: play **`SOUND_EVILLAFF`** (0x11c) if the profile is at 5-1 (i.e. the 999 boss reveal after 4-5), else **`SOUND_HATCH`** (0x124). This goes through app slot 54 (PlaySample, no RNG).
5. Mode 1 only (unreachable from `BuyEggPiece`; see [Other paths](#other-paths)): at `t` = 28, 48, 68 play `SOUND_BUY`; at 88 play `SOUND_GROW`.
6. Boss y (`+0xa4`), only when the profile is at 5-1:

   | `t` | Δy per tick |
   |---|---|
   | 140–160 | −12 |
   | 161–170 | −8 |
   | 171–180 | −5 |
   | 181–190 | −2 |
   | 191–219 | −1 |
   | ≥ 220 | bob: `t % 16` in 0..3 → −1; in 8..11 → +1; else 0 |

   That's −431 in total, so the boss settles at y = 49 and then bobs ±4.
7. If `t == 170`: the Continue label becomes **"Click Here to Continue"** and the button becomes mouse-visible (clickable).
8. `+0x94 += 1`.

### Timeline (normal pet)

`T` = counter value seen by Draw (the post-increment value). Draw `T` = number of Updates so far.

| T | What's shown | Ticks / time |
|---|---|---|
| 0–55 | Intact egg, `IMAGE_EGGCRACK1` cel 0, bobbing | 56 ticks, 1.57 s |
| 56–80 | `EGGCRACK1` cels `(T−56)/2` = 0..12, 2 ticks per cel (cel 12 for 1 tick) | 25 ticks |
| 81–119 | `IMAGE_EGGCRACK2` cel 0, **shaking** (random jitter) | 39 ticks |
| 120–140 | `EGGCRACK2` cels `(T−120)/2` = 0..9, 2 ticks each, shaking. At T = 140 the cel would be 10, which is out of range (see Deviations) | 21 ticks |
| 140–199 | **Shards** fly out (overlay) | 60 ticks |
| 141+ | **Pet** animates in the incubator window; **name** shown | – |
| (update at t = 141) | `SOUND_HATCH` | – |
| (update at t = 170) | button → "Click Here to Continue", clickable | 4.76 s after the screen opens |
| 171+ | Description lines and "Your game has been saved." | – |

Holding the mouse button down over the screen during T < 140 jumps straight to T = 141 (shards and pet). The button still waits until t = 170. A click whose down and up both land between two Updates won't be seen by step 4. `static` reasoning; CE check TODO.

## 5. Draw (`0x0051f770`)

All coordinates are screen coordinates (the widget is at 0,0). Text positions are the font baseline. "Centred" means `x = (650 − width) / 2 − 3`, using an arithmetic shift for the halving. `static`.

1. `IMAGE_HATCHSCREEN` (640×480) at (0, 0).
2. `IMAGE_SCREENTITLE` drawn as a 3×3-sliced box (`0x004563f0`, same box routine as dialogs) at (20, 0, 600, image height 44).
3. If the Menu button is visible: `IMAGE_SCREENTITLEHOLE` box at (btn.x − 1, btn.y − 1, btn.w + 2, image height), i.e. (524, 3, 82, 31).
4. Font `FONT_JUNGLEFEVER17OUTLINE`, colour (255, 200, 0): **"You have found:"** at (215, 25).
5. **Egg bob** `b` from `T % 32`:

   | `T % 32` | 0–3 | 4–7 | 8–11 | 12–15 | 16–19 | 20–23 | 24–27 | 28–31 |
   |---|---|---|---|---|---|---|---|---|
   | b | 0 | −1 | −2 | −1 | 0 | +1 | +2 | +1 |

6. Egg / pet, by T (`jx = +0x9c`, `jy = +0xa0`):
   - **T ≤ 80:** `IMAGE_EGGCRACK1` (13 cels of 118×145) cel `T < 56 ? 0 : (T − 56) / 2` at **(258, 60 + b)**. No jitter.
   - **Boss (profile at 5-1), T > 80:** `IMAGE_EGGCRACK2` (10 cels of 118×145) cel = 0 for T < 120, `(T − 120) / 2` for 120..139, 9 for ≥ 140, at (258 + jx, 60 + jy + b). For T > 160 the jitter fields are zeroed first (Draw writes `+0x9c/+0xa0 = 0`; Update re-randomises them next tick). Then skip to step 8.
   - **81 ≤ T ≤ 140:** `IMAGE_EGGCRACK2` cel `T < 120 ? 0 : (T − 120) / 2` at **(258 + jx, 60 + jy + b)**.
   - **T > 140: pet reveal.**
     - Clip rect: **(278, Y, 80, 80)**, with Y = 100 for Stinky (0) and Clyde (5), 70 for Niko (1), 90 otherwise. Amp (16) is the exception: (236, 100, 160, 60).
     - Frame `f`:
       - Ids 5, 10, 11, 14, 20: `(T % 40) / 4`.
       - Ids 1, 6: `T % 20`, folded to `19 − f` if > 9 (ping-pong).
       - Otherwise: `(T % 20) / 2`.
     - Draw the pet's swim sheet (`IMAGE_STINKY`, `IMAGE_NIKO`, …, `IMAGE_<NAME>` for ids 0..23) whole at **(278 − 80·f, Y)**. Amp is at (236 − 160·f, 100). The clip leaves row 0, column f visible: Stinky shows his normal crawl cels 0..9, 2 ticks each. No bob or jitter. Then colourise off.
     - **Name:** colour (255, 200, 0), font `FONT_JUNGLEFEVER15OUTLINE`, centred at y = 260. The long names (ids 9, 11, 14, 21) use `FONT_JUNGLEFEVER12OUTLINE`. Ids 20 and 22 also use 12OUTLINE and get a subtitle line ("the Scuba Diving Elephant" / "Small Sea Serpent") centred at y = 265, with the name moved up to y = 245.
7. (Pet ids have no `IMAGE_SCL_*` use here. The full-size swim sheets are used.)
   Tank 1 names and descriptions (three lines each, as stored in the binary):

   | id | Name | Description |
   |---|---|---|
   | 0 | STINKY the Snail | STINKY roams around the / bottom of your tank, catching / any coins you may have missed. |
   | 1 | NIKO the Oyster | NIKO produces pearls that / you can click on for a / hefty sum of money. |
   | 2 | ITCHY the Swordfish | ITCHY helps you by attacking / aliens when they appear. (2 lines) |
   | 3 | PREGO the Momma Fish | PREGO helps populate your / tank by giving birth to a new / baby guppy every so often. |
   | 4 | ZORF the Sea Horse | ZORF gives you a hand in / keeping your fish fed. (2 lines) |
8. **T > 170: text.**
   - Font `FONT_JUNGLEFEVER10OUTLINE`, white: the description, centred at y = **300, 320, 340**. Stinky: "STINKY roams around the" / "bottom of your tank, catching" / "any coins you may have missed."
   - Then colour (255, 255, 100): **"Your game has been saved."** at (221, 390), not centred.
   - Presto (0x13) instead gets "You have been awarded 5000 shells\nfor defeating the Final Boss!" through the widget's word-wrap writer (slot 64) in rect (180, 373, 284, 0) (layout `guess`).
   - 999 gets a 4-line description (4th line at y = 360) and its name "Evil Alien Mastermind" (15OUTLINE, (255,200,0), centred at y = 260). For 999 the name is drawn only from T > 170.

The two buttons draw themselves as standard `DialogButton`s with `IMAGE_MAINBUTTON` (up/over/down cels, horizontally stretched to 264 / 80 px; exact slicing `guess`) and their label centred.

### Overlay widget

`HatchScreenOverlay` (Draw slot 26 @ `0x00518910`) calls the owner's slot 44 (`0x00517a80`). It is added after the buttons, so it draws **on top of everything**, buttons included.

1. `IMAGE_HATCHREFLECTION` (`incubatorreflection`, 154×138) at **(240, 60)**: the glass reflection over the egg and pet.
2. Boss (profile at 5-1), T ≥ 140: clip (238, `+0xa4`, 160, 160) and draw `IMAGE_BOSS` at (238 − 160·((T % 20)/2), `+0xa4`).
3. Otherwise, **140 ≤ T < 200: shards.** With `s = T − 140`, for i in 0..7, draw `IMAGE_EGGSHARDS` (8 cels of 100×125) cel i at **(268 + vx[i]·s, 70 + vy[i]·s)**:

   | i | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
   |---|---|---|---|---|---|---|---|---|
   | vx | 1 | −11 | −14 | 12 | −15 | 11 | 13 | −1 |
   | vy | −10 | −15 | 3 | −13 | 10 | 5 | 14 | 12 |

   The motion is linear: no gravity, no fade.

## 6. Input

- **Mouse on the screen** (outside the buttons): only the skip in Update step 4. The overlay is mouse-invisible, so clicks reach the HatchScreen.
- **Buttons** are standard `ButtonWidget`s:
  - Mouse-down → listener slot 1 `ButtonPress(id)` @ `0x0051a6f0`: plays `SOUND_BUTTONCLICK` (0x105). No RNG.
  - Mouse-up while still over the button → listener slot 2 **`ButtonDepress(id)`** @ `0x00517c70`.
  - **The action fires on mouse-up** (unlike the shop bar, which acts on mouse-down). The Continue button can't receive clicks before t = 170.

### ButtonDepress

- **99 (Continue):**
  1. `App::RemoveHatchScreen` (WidgetManager RemoveWidget + SafeDeleteWidget, `app+0x814 = 0`).
  2. Mode 1 or 4: `0x00552230` (save, back to the game selector). Unreachable here.
  3. Profile at 5-2 (just beat the boss): `0x0054bd80` → **InterludeScreen** (ending).
  4. Otherwise: **`App::NextLevel(0, 0)` @ `0x00552380`**.
- **100 (Menu):** remove the hatch screen. At profile 5-2 → InterludeScreen. Otherwise `0x00552100` → **GameSelector** (main menu). The profile is already saved at the next level, so Adventure resumes there.

### `App::NextLevel(resume=0, help=0)` @ `0x00552380`

1. If the trial/demo limit check `0x0054bbb0` is hit → GameSelector instead.
2. `0x0054bc30`: destroy the board (already gone here, so no-op).
3. (Mode 5 one-time setup; `resume` path skipped.)
4. **Clear `profile+0x5a..+0x71`** (chosen pets).
5. (`help` = 0, so the tutorial HelpScreen branch for 1-1..1-3 is skipped.)
6. If not a bonus round, mode ≠ 5, **owned count `profile+0x18` > 3**, and (mode ≠ 0 or profile tank ≠ 5): **PetScreen** (`0x0054c2d0`) to choose pets. It calls StartLevel later.
7. Otherwise → **`App::StartLevel` @ `0x0054cbf0`** (`sim/level-start.md`):
   - `0x0054c540`: new Board (3 app draws), `app+0x884 = 0`, `++profile+0x4c`.
   - `InitLevel`: tank/level from the profile, so **1-2**; **money reset to 200**.
   - If bonus round: `0x00537e20`; else `PopulateLevel`.
   - Then `0x0054b2d0`.

**What carries over from 1-1 to 1-2:** only profile state (owned pets, tank/level, best time). The money, fish, food upgrades and every other board field are discarded with the board. `food_quality`/`food_quantity` are globals, but InitLevel resets them. On 1-2 there is 1 owned pet (≤ 3), so no PetScreen. `PopulateLevel`'s "≤ 3 owned" branch adds every owned pet with id 0..18 in id order, from `profile[id]`. So **Stinky is added first**, then the two guppies.

## 7. RNG

| Where | Stream | Draws |
|---|---|---|
| `BuyEggPiece` → `ShowHatchScreen` (unlock, save, board destroy) | – | **0 app** (no draws found; `static`) |
| HatchScreen ctor: BubbleMgr pre-warm (500 × `0x005110e0`) | **global** `Sexy::Rand` | variable (per-bubble wobble `&1`, emit `%100`, new-bubble 5–6, stream-bubble draws; see `sim/board.md#the-global-rng-stream-in-normal-play`) |
| Each Update, step 1 (bubbles) | global | variable |
| **Each Update, step 3** | **app** `[app+0x7b0]` | **2: `& 3` (jitter x), then `& 1` (jitter y)** |
| Button press sound | – | 0 |
| `NextLevel` → `StartLevel` | app | as in `sim/level-start.md` (Board ctor 3, …) |

**Consequence for determinism:** the app stream at the start of 1-2 = the state at the end of 1-1 + **2 × (number of HatchScreen Updates)**, i.e. the number of ticks the player spent on the hatch screen (≥ 171). For a straight replay of 1-1 → 1-2, the sim has to run the hatch screen tick by tick and make these 2 draws per tick. The harness can otherwise re-sync the app RNG at 1-2 start from the capture.

The bubbles are **never drawn**. Neither Draw nor DrawOverlay calls the BubbleMgr draw (`0x00504b60`), so the bubbles only consume global draws. We can drop them (the global stream is re-synced per capture), or model them only if a trace needs global-stream continuity across the screen.

## Other paths

- **`profile+0x59` set** (replaying after beating the game): no hatch screen.
  - Board 5-1: advance and BonusScreen (`0x0054bf60`).
  - Otherwise: `0x0053dc10`: `board+0x4ee = 1`, music (2, 54), and `0x00537e20`. The same board turns into a bonus round in place. TODO.
- **Mode 4** (Challenge): sets `profile+0x77 + tank`, records, and shows BonusScreen. **Mode 1** returns early ("random pet"), and mode 3 ends the level. So the mode-1 sound cues in Update and the mode-1/4 branch of ButtonDepress are dead code in this build.
- **Board 5 (`BuyEggPiece`):** before recording, the ids of pets *not present* on the board (`board+0x144[id] == 0`, id < 18) are stored in globals `0x005e8fe0[]`, count `0x005e9080`. Also, instead of the time, the number of pets is recorded as the score. Not relevant to tank 1.

## Deviations

- **Out-of-range cel at T = 140** (normal pets): `EGGCRACK2` cel 10 of a 10-cel image. `DrawImageCel` turns it into column 0, row 1, i.e. a source rect below the image, so (probably) nothing is drawn for one frame, while the shards start at the same frame. Implementation artifact. Our implementation should draw nothing for the egg at T = 140 to match what's visible (`guess`; confirm with a screenshot).

## Open questions

- CE: confirm the 2 app draws per hatch tick (watch `mti` at `[app+0x7b0]+0x9c0`), and confirm that the Update count before a click is ≥ 171. Check whether an Update runs on the frame the Continue click is processed.
- CE: whether a mouse-down + mouse-up inside one frame triggers the skip.
- `DialogButton` drawing with a 3-cel `IMAGE_MAINBUTTON` stretched to 264 px: slicing and label baseline. Belongs in a widget spec.
- KillDialog ids 0x1f / 0x20 (which dialogs).
- Exact layout of the Presto word-wrapped line (slot 64 arguments).
