# The Bonus screen and how pets 20–23 are unlocked

Class `Sexy::BonusScreen` (ctor inlined in Ghidra's `BonusScreenOverlay::BonusScreenOverlay` @ `0x0052dad0`, dtor `0x0051bb30`), shown by `0x0054bf60` and stored at `app+0x818`. The screen sells the four bonus pets (20–23) in a fixed order, then two pets-per-level upgrades. An overlay widget (`BonusScreenOverlay`, built by `0x0051c8e0`) draws the "Level Info" / "Bonus Reward" panels and the shell count-up animation; those visuals are not fully followed.

| Function | Address | Notes |
|---|---|---|
| ctor (inlined in overlay ctor) | `0x0052dad0` | Ghidra labels it `BonusScreenOverlay::BonusScreenOverlay` |
| dtor | `0x0051bb30` | vtable `0x005a1fc4`, ButtonListener vtable `0x005a1fa4` at `+0x88` |
| Offer setup | `0x00516f70` | from the ctor |
| Buy action | `0x005172e0` | owns the pet, deducts shells |
| **Update** (slot 22) | `0x00517090` | ~117 bytes; **no RNG** (only calls Widget::Update base) |
| **Draw** (slot 26) | `0x0051daa0` | |
| **DrawOverlay** (slot 44) | `0x0051dc70` | drawn by the overlay widget, on top of everything |
| AddedToManager (slot 20) | `0x00516fd0` | adds +0xac, +0xa8, +0xb0, +0xa4 to the WidgetManager |
| RemovedFromManager (slot 21) | `0x00517030` | |
| ButtonPress (listener slot 1) | `0x00517560` | plays sound; for id 2 (buy), extra visual feedback |
| **ButtonDepress** (listener slot 2) | `0x0051e060` | Continue, Menu and buy handlers |
| `App::ShowBonusScreen` | `0x0054bf60` | |
| `App::RemoveBonusScreen` | `0x0054ae30` | zeros `app+0x818` |
| Dialog `0x23` handler | `0x0054b900` | WinFishApp slot 3 |

Confidence: `traced` for fields, Update (app stream), offer, and music — by `bonus_screen` (`bonus_screen_01`). The global stream finding is `confirmed` (see [RNG](#rng)). Buying, button handlers, exit flow, and layout/overlay visuals remain `static`. The capture ended while still on the bonus screen (purchased = 0 throughout, no exit observed).

Notation: `app` = WinFishApp. `profile` = `[app+0x8b8]`. `n` = `profile+0xb0` (bonus purchases made, 0 for a fresh profile). One tick = 28 ms (`framework/timing.md`). Offsets below are `this`-relative to the BonusScreen (Ghidra's `this[1]` in the overlay ctor; the methods use them directly).

## When it is shown

`App::ShowBonusScreen` @ `0x0054bf60` (close dialogs, `app+0x884 = 1`, build the screen, stop music, play music (2, 0x36)) is called from:

- **`Board::Update` @ `0x00547b6e`**: the end of a **bonus round** (level 6 of tanks 1–3). Once its time is up and the coin list is empty, a counter `board+0x3f8` runs past 100 ticks; then the profile advances (`0x00501370`), the result is recorded (`0x00514510`, levels ≤ 5) and the screen opens.
- **Level complete** (`0x00545960`, the 4th egg piece):
  - Challenge mode (`app+0x87c == 4`): always, after setting `profile+0x77 + tank`.
  - After beating the game (`profile+0x59` set), on 5-1 only.
- Dialog `0x26`'s handler (`0x0054c6d0`), and `0x0054b900` (below) when the BonusScreen isn't up.

So in Adventure the player meets it after tanks 1, 2 and 3 (each bonus round), and on every replay of 5-1.

`confirmed` (tank 1 bonus round → bonus screen observed in `bonus_screen_01`; the full calling path is `static`).

## Known fields

Offsets are raw `this`-relative, confirmed by cross-referencing the ctor (`0x0052dad0`), the Update (`0x00517090`), the offer setup (`0x00516f70`), the buy action (`0x005172e0`) and the ButtonDepress handler (`0x0051e060`).

| Offset | Type | Meaning | Initial |
|---|---|---|---|
| `+0x28` | int | **widget update counter** (incremented by `Widget::Update` base) | 0 |
| `+0x54` | byte | visible (Widget base) | 1 |
| `+0x58` | byte | **mouse is down** on this widget (Widget `mIsDown`) | 0 |
| `+0x88` | ptr | ButtonListener sub-object vtable (`0x005a1fa4`) | – |
| `+0x8c` | ptr | app | ctor arg |
| `+0xa4` | ptr | **overlay** widget (`BonusScreenOverlay`) | new |
| `+0xa8` | `DialogButton*` | **Menu** button, id **1**, label "Menu" | new |
| `+0xac` | `DialogButton*` | **Continue** button, id **0**, label "Click Here To Continue" | new |
| `+0xb0` | `DialogButton*` | **Buy** button, id **2**, label "" (drawn by overlay) | new; hidden when `n > 5` |
| `+0xb4` | ptr | secondary panel widget (destroyed on purchase, replaced with egg strip) | new |
| `+0xd0` | int | **price** in shells; 0 when `n > 5` | offer setup |
| `+0xd4` | int | **animation counter** (set to 1 by buy; mouse-down skip sets it to 140) | 0 |
| `+0xe0` | byte | **purchase-made flag** (set to 1 after a buy) | 0 |
| `+0xe1` | byte | flag checked by Update for the buy-button blink (adjacent to `+0xe0`; semantics unclear) | 0 |

`traced` (`bonus_screen`): `+0x28` (t), `+0xd0` (price), `+0xd4` (anim) and `+0xe0` (purchased) matched every tick of `bonus_screen_01`. Other offsets `static`.

## Construction (ctor, inlined in `0x0052dad0`)

Builds the BonusScreen and the overlay. Key steps for the sim:

1. Widget base; vtables at `+0x00` and `+0x88`; `+0x8c = app`.
2. `App::StopAllMusic` (`0x0054b020`) then `PlayMusic(2, 54, 0)` → track 4 order 49.
3. **Menu button** `+0xa8` = `DialogButton(id=1, listener=this+0x88, label="Menu", image=IMAGE_MAINBUTTON)`. Hidden (`+0x54 = 0`) when profile at 5-2 (just beat the boss).
4. **Continue button** `+0xac` = `DialogButton(id=0, listener=this+0x88, label="Click Here To Continue", font=FONT id 0x16)`. In Challenge mode (`app+0x87c == 4`) with a board present (`app+0x730 != 0`), the label is changed to **"Click for Story Time"** and the Menu button is hidden.
5. **Buy button** `+0xb0` = `DialogButton(id=2, ...)` with an empty label (the overlay draws the buy text). Hidden when `n > 5` (offer setup).
6. Overlay `+0xa4` = `BonusScreenOverlay`.
7. Offer setup (`0x00516f70`): sets `+0xd0` (price) and hides the buy button if `n ≥ 6`.
8. `app+0x884 = 1`.

`AddedToManager` (`0x00516fd0`) adds widgets in order: +0xac (Continue), +0xa8 (Menu), +0xb0 (Buy), +0xa4 (overlay). The overlay is last, so it draws on top of the buttons.

`static` (construction details); music `traced` (`bonus_screen`: track 2, order 54 confirmed every tick).

## The offer (`0x00516f70`, from the ctor)

- `n ≥ 6`: nothing is for sale (price `+0xd0 = 0`, the buy widget `+0xb0` hidden via vtable slot 31).
- else **price `+0xd0 = min((n + 4) · 5000, 50000)`** shells: 20 000, 25 000, 30 000, 35 000 for the four pets, then 40 000 and 45 000 for the upgrades. (The 50 000 cap never triggers since max is (5+4)·5000 = 45 000.)

What's offered (`0x005172e0`, and the confirm texts in the ButtonDepress handler `0x0051e060`):

| `n` | Item | Price |
|---|---|---|
| 0 | pet **20 BRINKLEY** | 20 000 |
| 1 | pet **21 NOSTRADAMUS** | 25 000 |
| 2 | pet **22 STANLEY** | 30 000 |
| 3 | pet **23 WALTER** | 35 000 |
| 4 | "Bonus Upgrade": pets per level (`profile+0xb4`) raised to at least **4** | 40 000 |
| 5 | "Bonus Upgrade": raised to at least **7** | 45 000 |

The order is fixed: the pets come one per purchase, Brinkley first. `profile+0xb4` is the pet screen's "most pets you may choose" (`pet-screen.md`, 3 for a fresh profile; the pet screen caps it at 4 outside the Virtual Tank).

`traced` (`bonus_screen`): n = 0, price 20 000 confirmed in `bonus_screen_01`. Other rows `static`.

## Buying (`0x005172e0`, called from dialog `0x23` handler)

1. Stop all music, play music (2, 0x36) — same track as on entry.
2. Destroy the secondary panel widget at `+0xb4` (set to null).
3. `n` 0–3: **own pet `20 + n`** via `0x00501420(profile, 20+n, 1)` (owned byte set, `profile+0x18` + 1). Draw the egg image `EGGBRINKLEY`/`EGGNOSTRADAMUS`/`EGGSTANLEY`/`EGGWALTER` (image id `0x6e + n`) into a strip (stored at `+0xb4`).
4. `n` 4 or 5: `profile+0xb4 = max(profile+0xb4, 4 or 7)`.
5. `profile+0xb0 += 1`; **shells −= price** (`0x00501200`, clamped to 0..9 999 999).
6. `+0xd4 = 1` (starts the animation counter).
7. `+0xe0 = 1` (purchase-made); hide the buy widget.

No RNG anywhere in this path.

`static`.

## ButtonDepress (`0x0051e060`)

The handler dispatches on the button id argument. `static`.

### Button 0 — Continue

1. `App::RemoveBonusScreen` (`0x0054ae30`): remove from WidgetManager, safe-delete, `app+0x818 = 0`.
2. If mode 1: `0x00552230` (save, GameSelector). *Unreachable in Adventure.*
3. If `app+0x738 != 0` (a board-related pointer): same save + GameSelector path.
4. If mode 4 (Challenge): `0x00552e50`. *Out of scope.*
5. If profile at 5-2 (just beat the boss): `0x0054bd80` → **InterludeScreen** (ending).
6. Otherwise: **`App::NextLevel(1, 1)`** @ `0x00552380`.

Note: the bonus screen calls `NextLevel(1, 1)` whereas the hatch screen calls `NextLevel(0, 0)`. The first arg is `resume` (enables a Mode 5 setup path), the second is `help` (enables tutorial HelpScreens for 1-1..1-3). After a bonus round the profile is at the next tank's level 1, so neither flag changes behavior in Adventure.

`static`.

### Button 1 — Menu

1. `App::RemoveBonusScreen` (`0x0054ae30`).
2. `0x00552230` → save + **GameSelector** (main menu). The profile is already at the next level.

`static`.

### Button 2 — Buy

1. If shells (`profile+0x48`) **< price** (`+0xd0`, accessed as `[esi+0x48]` from the listener at `+0x88`):
   - If `n < 4`: dialog `0x0e`, "Not Enough Shells" / "…to purchase this Bonus Pet."
   - If `n ≥ 4`: "…this Bonus Upgrade."
   - Button OK. Nothing else.
2. Else **"Confirm Purchase"** dialog `0x23` (`0x0054fdd0`):
   - If `n < 4`: "Would you like to\nbuy this Bonus Pet?"
   - If `n ≥ 4`: "…Bonus Upgrade?"
   - Buttons BUY / CANCEL.
3. BUY (`0x0054b900(1)` → `0x005172e0`): see [Buying](#buying-0x005172e0-called-from-dialog-0x23-handler).

`static`.

## Update (`0x00517090`)

Called once per tick. **No RNG calls** — the only call is to `Widget::Update` base (`0x0046e310`), which increments `+0x28`.

1. `Widget::Update` base.
2. **Mouse-down skip:** if `+0x58` (mouse held down on the screen) and `0 < +0xd4 < 140`: set `+0xd4 = 140`. This skips the shell count-up animation, analogous to the hatch screen's egg-skip.
3. **Buy-button blink:** only when `+0xd0 > 0` (price set), `shells ≥ price`, and `+0xe1 == 0`:
   - If `10 < +0x28 < 180`: toggle `buy_widget+0x59` based on `(+0x28 / 10) % 2`. The buy button blinks on and off every 10 ticks (~0.28 s) to signal affordability.
   - If `+0x28 == 180`: a resize/reshow operation on the buy widget (the exact call goes through vtable slot 68 of the widget).
4. **Exit hide** (unconditional): if `+0x28 == 30`: hide all three buttons (`+0xa8`, `+0xac`, `+0xb0`) via `SetVisible(false)` (vtable slot 37). This is the close-out animation after a purchase starts (the buy action sets `+0xd4 = 1`, and the ctor runs the offer setup; the counter == 30 check likely fires during the purchase's count-up sequence when the screen is being removed).

**Consequence for determinism:** the **app stream** passes through untouched. The **global stream** is consumed at a variable rate (~2–6 draws per tick), likely by the overlay's visual effects (shell count-up animation, particle effects). The BonusScreen::Update itself makes no RNG calls (confirmed), so the global draws come from elsewhere in the widget tree (the overlay or a background system).

`traced` (`bonus_screen`): app MTI unchanged across 3908 ticks; global MTI changes confirmed in `bonus_screen_01`. The Update function's logic (mouse-down skip, buy-button blink, exit hide at tick 30) is `static`.

## Exit flow

Confirmed from disassembly of `ButtonDepress` at `0x0051e060`:

- **Continue** (button 0): removes the BonusScreen. In Adventure the profile was already advanced before the screen opened (`0x00501370`, see [When it is shown](#when-it-is-shown)), so the next step is `App::NextLevel(1, 1)` → pet screen (if > 3 pets owned) → `App::StartLevel`, or directly `StartLevel` when ≤ 3 pets. At profile 5-2 (game beaten), goes to InterludeScreen instead.
- **Menu** (button 1): removes the BonusScreen → save + `App::ShowGameSelector` (main menu).
- The Continue button is labeled **"Click Here To Continue"** in Adventure. In Challenge mode with a board present, it reads **"Click for Story Time"** (leads to a `StoryScreen`). The Menu button is hidden at profile 5-2 and in Challenge mode.

`static` (the ButtonDepress handler; no exit was observed in `bonus_screen_01` — the recording ended on the bonus screen).

## Music

`App::ShowBonusScreen` stops all music (`0x0054b020`) then plays **(2, 0x36)** = **(2, 54)**, remapped to **track 4 (`Insaniq2.mo3`), order 49** (`assets/music.md`). This is the same track as the hatch screen. The buy action (`0x005172e0`) also restops and restarts the same track.

`traced` (`bonus_screen`): track 2 order 54 confirmed every tick of `bonus_screen_01`.

## RNG

**No RNG** in the offer setup (`0x00516f70`), the buy path (`0x005172e0`), the ButtonDepress handler (`0x0051e060`), **or the Update function** (`0x00517090`). The Update's only call is to the Widget::Update base, which does not consume RNG.

**App stream:** passes through untouched (app MTI constant across 3908 ticks in `bonus_screen_01`). The sim's bonus screen correctly takes no app draws.

**Global stream:** consumed at ~2–6 draws per tick (variable; occasionally 0 or up to 13). The source is **not** the BonusScreen::Update (confirmed: 117 bytes, only calls Widget::Update base). The draws likely come from the `BonusScreenOverlay`'s visual effects (the shell count-up animation or particle system). For replay, the sim re-syncs the global stream at the next level start, so these draws don't affect level determinism.

`traced` (`bonus_screen`): app MTI unchanged, global MTI changes confirmed in `bonus_screen_01`. The overlay's draw path is `static`.

## Shells

The price is far above what tank 1 pays: shells come from the Virtual Tank and from beating the final boss (+5000 with Presto, `hatch-screen.md`). For recordings, pets 20–23 are given with the CE profile edit (owned bytes `+0x14..+0x17`), as for the other pet zoos (`dynamic-analysis.md#recording-a-pet-zoo-any-pets-any-level`); nothing in the board depends on how they were obtained.

`static`.

## Other unlock paths

- `0x00501450` (owns pets 0 … k−1 for the profile's progress; all 20 when `profile+0x59` is set) only covers ids < 20: the bonus pets aren't part of it.
- No other caller of `0x00501420` with ids 20–23 was found (xrefs: `0x00501450`, `0x00545960`, `0x005172e0`).

`static`.

## Open questions

- **`+0xe1` flag**: the Update checks `byte [this+0xe1]` to gate the buy-button blink, but the Buy action writes `byte [this+0xe0]`. These are adjacent bytes; the setter of `+0xe1` is not identified (possibly the shell count-up animation or the overlay). Not needed for the sim (no gameplay effect), but clarify if building the visual blink.
- **`+0x28` semantics at counter == 30**: the Update hides all buttons when the Widget update counter reaches 30. This likely interacts with the purchase animation (the buy action sets `+0xd4 = 1` to trigger the count-up). The exact interplay between `+0x28` and `+0xd4` during the purchase sequence is not fully traced.
- **Layout, panels and shell count-up** (`0x0051c8e0`, `0x0051daa0`, `0x0051dc70`): the visual positions of the panels, the animated shell counter, and the egg strip drawn on purchase. Needed for drawing but not for the sim.
- **Widget vtable slot 37 / slot 68**: assumed to be `SetVisible` and a resize method respectively, based on argument patterns. Not verified.
