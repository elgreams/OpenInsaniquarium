# The Bonus screen and how pets 20–23 are unlocked

Class `Sexy::BonusScreen` (ctor inlined in Ghidra's `BonusScreenOverlay::BonusScreenOverlay` @ `0x0052dad0`, dtor `0x0051bb30`), shown by `0x0054bf60` and stored at `app+0x818`. The screen sells the four bonus pets (20–23) in a fixed order, then two pets-per-level upgrades. An overlay widget (`BonusScreenOverlay`, built by `0x0051c8e0`) draws the "Level Info" / "Bonus Reward" panels and the shell count-up animation; those visuals are not fully followed.

Confidence: `static` throughout (nothing captured).

| Function | Address | Notes |
|---|---|---|
| ctor (inlined in overlay ctor) | `0x0052dad0` | Ghidra labels it `BonusScreenOverlay::BonusScreenOverlay` |
| dtor | `0x0051bb30` | |
| Overlay ctor/Draw | `0x0051c8e0` | panels, count-up; not followed |
| Offer setup | `0x00516f70` | from the ctor |
| Buy action | `0x005172e0` | owns the pet, deducts shells |
| ButtonDepress (slot 3) | `0x0051e060` | buy button, id 2 |
| `App::ShowBonusScreen` | `0x0054bf60` | |
| Dialog `0x23` handler | `0x0054b900` | WinFishApp slot 3 |

Notation: `app` = WinFishApp. `profile` = `[app+0x8b8]`. `n` = `profile+0xb0` (bonus purchases made, 0 for a fresh profile). One tick = 28 ms (`framework/timing.md`). Offsets below are `this`-relative to the BonusScreen (Ghidra's `this[1]` in the overlay ctor; the methods use them directly).

## When it is shown

`App::ShowBonusScreen` @ `0x0054bf60` (close dialogs, `app+0x884 = 1`, build the screen, stop music, play music (2, 0x36)) is called from:

- **`Board::Update` @ `0x00547b6e`**: the end of a **bonus round** (level 6 of tanks 1–3). Once its time is up and the coin list is empty, a counter `board+0x3f8` runs past 100 ticks; then the profile advances (`0x00501370`), the result is recorded (`0x00514510`, levels ≤ 5) and the screen opens.
- **Level complete** (`0x00545960`, the 4th egg piece):
  - Challenge mode (`app+0x87c == 4`): always, after setting `profile+0x77 + tank`.
  - After beating the game (`profile+0x59` set), on 5-1 only.
- Dialog `0x26`'s handler (`0x0054c6d0`), and `0x0054b900` (below) when the BonusScreen isn't up.

So in Adventure the player meets it after tanks 1, 2 and 3 (each bonus round), and on every replay of 5-1.

`static`.

## Known fields

Offsets are raw `this`-relative, extracted from the code at `0x00516f70`, `0x005172e0` and `0x0051e060`. The full struct layout (overlay pointer, tick counter, other widgets) has not been traced; what's listed is only what the offer and buy paths reference.

| Offset | Type | Meaning |
|---|---|---|
| `+0xb0` | Widget* | **buy widget** (button); hidden when `n > 5` or after a purchase |
| `+0xd0` | int | **price** in shells; 0 when `n > 5` |
| `+0xe0` | int | **purchase-made flag** (set to 1 after a buy) |

`static`.

## The offer (`0x00516f70`, from the ctor)

- `n > 5`: nothing is for sale (price `+0xd0 = 0`, the buy widget `+0xb0` hidden).
- else **price `+0xd0 = min((n + 4) · 5000, 50000)`** shells: 20 000, 25 000, 30 000, 35 000 for the four pets, then 40 000 and 45 000 for the upgrades.

What's offered (`0x005172e0`, and the confirm texts in slot 3 `0x0051e060`):

| `n` | Item | Price |
|---|---|---|
| 0 | pet **20 BRINKLEY** | 20 000 |
| 1 | pet **21 NOSTRADAMUS** | 25 000 |
| 2 | pet **22 STANLEY** | 30 000 |
| 3 | pet **23 WALTER** | 35 000 |
| 4 | "Bonus Upgrade": pets per level (`profile+0xb4`) raised to at least **4** | 40 000 |
| 5 | "Bonus Upgrade": raised to at least **7** | 45 000 |

The order is fixed: the pets come one per purchase, Brinkley first. `profile+0xb4` is the pet screen's "most pets you may choose" (`pet-screen.md`, 3 for a fresh profile; the pet screen caps it at 4 outside the Virtual Tank).

`static`.

## Buying (`BonusScreen` slot 3 @ `0x0051e060`, button id 2)

1. If the profile's shells (`profile+0x48`) are **below the price**: dialog `0x0e` "Not Enough Shells", "Sorry, but you need more Shells to purchase this Bonus Pet." ("this Bonus Upgrade" when `n ≥ 4`), button OK. Nothing else.
2. Else the **"Confirm Purchase"** dialog `0x23` (`0x0054fdd0`): "Would you like to\nbuy this Bonus Pet?" (or "…Upgrade?" when `n ≥ 4`), buttons BUY / CANCEL.
3. BUY (`WinFishApp` slot 3, id `0x23` → `0x0054b900(1)`): close the dialog; if the BonusScreen is up, **`0x005172e0`**:
   - stop all music, play music (2, 0x36);
   - `n` 0–3: **own pet `20 + n`** (`0x00501420`: owned byte set, owned count `profile+0x18` + 1), and draw the egg image `EGGBRINKLEY`/`EGGNOSTRADAMUS`/`EGGSTANLEY`/`EGGWALTER` (image id `0x6e + n`) into a strip shown on the screen;
   - `n` 4 or 5: `profile+0xb4 = max(profile+0xb4, 4 or 7)`;
   - `profile+0xb0 += 1`; **shells −= price** (`0x00501200`, clamped to 0..9 999 999); `+0xe0 = 1`; hide the buy widget.

   (`0x0054b900(1)` with the Virtual Tank store screen `app+0x810` up goes to that store instead, `0x0052bd70`.)

No RNG anywhere in this path.

`static`.

## Exit flow

The BonusScreen has at least three buttons visible: **Continue** (or "Story Time" after beating the game), **Menu**, and the buy button (id 2, above). The exact button ids and handler addresses for Continue and Menu are not identified; the following is inferred from the entry callers and from the analogous hatch-screen and pet-screen patterns.

- **Continue / Story Time**: removes the BonusScreen. In Adventure the profile was already advanced before the screen opened (`0x00501370`, see [When it is shown](#when-it-is-shown)), so the next step is `App::NextLevel` → pet screen (if > 3 pets owned) → `App::StartLevel`, or directly `StartLevel` when ≤ 3 pets. After 5-1 (game beaten, `profile+0x59`), the path likely goes to the InterludeScreen (ending) instead, as the hatch screen does.
- **Menu**: removes the BonusScreen and goes to `App::ShowGameSelector` (the main menu), as the hatch screen's Menu does.

`static`.

## Music

`App::ShowBonusScreen` stops all music (`0x0054b020`) then plays **(2, 0x36)** = **(2, 54)**, remapped to **track 4 (`Insaniq2.mo3`), order 49** (`assets/music.md`). This is the same track as the hatch screen. The buy action (`0x005172e0`) also restops and restarts the same track.

`static`.

## RNG

**No RNG** in the offer setup (`0x00516f70`) or the buy path (`0x005172e0`, `0x0051e060`). Whether the BonusScreen's update or ctor consumes any RNG draws (e.g. a BubbleMgr like the hatch screen has) is unknown from static analysis alone. If there is no per-tick RNG, the app and global streams pass through the screen untouched, and replay can re-sync at the next level start.

`static`.

## Shells

The price is far above what tank 1 pays: shells come from the Virtual Tank and from beating the final boss (+5000 with Presto, `hatch-screen.md`). For recordings, pets 20–23 are given with the CE profile edit (owned bytes `+0x14..+0x17`), as for the other pet zoos (`dynamic-analysis.md#recording-a-pet-zoo-any-pets-any-level`); nothing in the board depends on how they were obtained.

`static`.

## Other unlock paths

- `0x00501450` (owns pets 0 … k−1 for the profile's progress; all 20 when `profile+0x59` is set) only covers ids < 20: the bonus pets aren't part of it.
- No other caller of `0x00501420` with ids 20–23 was found (xrefs: `0x00501450`, `0x00545960`, `0x005172e0`).

`static`.

## Open questions

- **Continue / Story Time / Menu buttons**: the button ids, handler addresses and exact behavior (which `App::` function each calls). Required to build the screen's exit flow. Ghidra: look for ButtonDepress at a nearby address to `0x0051e060` (the buy handler is slot 3, so the same ButtonListener likely handles Continue and Menu).
- **Update function**: whether the BonusScreen has a per-tick Update (the shell count-up animation suggests it does). Determines whether any RNG is consumed per tick and whether the screen needs a tick loop in the sim.
- **Full struct layout**: the fields beyond `+0xb0`, `+0xd0`, `+0xe0` (overlay pointer, tick counter, Continue/Menu widget pointers).
- **Layout, panels and shell count-up** (`0x0051c8e0`): the visual positions of the panels, the animated shell counter, and the egg strip drawn on purchase. Needed for drawing but not for the sim.
- Capture a purchase (the dialogs, the shells, `+0xb0`) once a profile has the shells.
