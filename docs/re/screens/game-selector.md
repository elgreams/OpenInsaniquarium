# GameSelector (main menu)

The main menu: the mermaid, the speech bubble and the mode buttons. It is shown after the title screen, after leaving a level, and after GAME OVER.

Status: per section. The button rects and labels are `static` (ctor). The look of each state is `confirmed` by screenshots of the original (`traces/frames/menus_01/`, 2026-10-28; local and gitignored), not by a frame diff yet.

## Entry and ownership

`App::ShowGameSelector` @ `0x00552100`: removes any current GameSelector, kills the hatch screen if one is up (`app+0x8b8` check, `0x00514b00`), builds a `GameSelectorOverlay` (`0x0052e070`, RTTI `GameSelector` inlined) the size of the window, adds it to the widget manager and gives it focus. With no profile (`app+0x8b8 == 0`) it then opens the **new-user dialog** (`0x0054b6a0`; see [No profile](#no-profile)). `static`.

Callers seen: GAME OVER's button (dialog `0x11`, `WinFishApp` button handler `0x005527c0` → `0x0054bc30` destroy board → `0x00552100`), Leave (`0x00552230`: save, `0x0054ba40`, destroy board, `0x00552100`), the hatch and pet screens' Menu buttons. `static`. Back to Main Menu then Leave is `confirmed` (screenshot `20_menu_after_leave.png`).

## Widgets

Button ids, images and rects (x, y, w, h), set in the ctor. `static`.

| id | Label | Kind | Images (up / over / down) | Rect |
|---|---|---|---|---|
| 1 | Adventure (see [labels](#labels)) | `ButtonWidget` | `IMAGE_BLANK` / `IMAGE_BATTLETANKBUTTON` / `IMAGE_BATTLETANKBUTTOND` | (357, 48, 217, 66) |
| 8 | Time Trial | `ButtonWidget` | blank / `IMAGE_MIDDLEBUTTON` / `IMAGE_MIDDLEBUTTOND` | (359, 142, 213, 48) |
| 9 | Challenge | `ButtonWidget` | blank / `MIDDLEBUTTON` / `MIDDLEBUTTOND` | (359, 212, 213, 48) |
| 2 | Virtual Tank | `ButtonWidget` | blank / `BATTLETANKBUTTON` / `BATTLETANKBUTTOND` | (357, 287, 217, 66) |
| 6 | Hall of Fame | `DialogButton` | `IMAGE_MAINBUTTON` (3 cels) | (401, 380, 127, h) |
| 3 | Options | `DialogButton` | `IMAGE_LEFTBUTTON` | (325, 412, 92, h) |
| 7 | Help | `DialogButton` | `IMAGE_CENTERBUTTON` | (419, 412, 93, h) |
| 4 | Quit | `DialogButton` | `IMAGE_RIGHTBUTTON` | (514, 412, 89, h) |

`h` is the image height (29 for all four images).

- The four big buttons use font `FONT_JUNGLEFEVER15OUTLINE` with label colours (255, 240, 0) normal and (200, 200, 255) when hovered (the ctor's two colours). Their "up" image is blank: the purple pill is part of `IMAGE_SELECTORSCREEN`. Hover draws the over image (a yellow pill) on top; pressing draws the down image. `confirmed`: `h_adv.png` (hover: yellow pill, lavender label), `h_down.png` (pressed: darker-rimmed pill), `h_none.png` (no hover: purple pill from the backdrop, yellow label).
- The small buttons are standard game `DialogButton`s (`0x00506820` → `0x005034e0`): three cels up / over / down, label in `FONT_JUNGLEFEVER10OUTLINE`, colours (255, 240, 0) and (255, 255, 255), text offset (−2, 0) (`framework/dialog.md#buttons`). Hover turns the button blue. `confirmed`: `h_quit.png`.
- Adventure has a second line under its label: `Tank T-L` (or `Bonus Level T` at level 6, or a story line at 5-1), in `FONT_JUNGLEFEVER10OUTLINE`, centred at x = button centre + 0x1d1 offset arithmetic (`0x0051e810`), and only once the profile isn't at 1-1. Virtual Tank shows `N Shells` when the profile has any. `static` for the formula; `confirmed` that both lines show (`02_menu.png`: "804 SHELLS"; `h_none.png`: "TANK 2-1").

## Labels

`0x0051eb70`: with a profile, Adventure reads **"Bonus Adventure"** after the game is beaten (`profile+0x59`), **"Start Adventure"** at 1-1, else **"Adventure"**. With no profile it reads "Adventure". `static`; "Start Adventure" (profile at 1-1) and "Adventure" (no profile; 2-1) are `confirmed` by screenshots.

## Drawing

`GameSelector::Draw` (slot 26 @ `0x0051e690`), in order. `static`, and the result is `confirmed` by the screenshots:

1. `IMAGE_SELECTORSCREEN` at (0, 0). It holds the mermaid, the brass panel and the purple pills.
2. Trophy (`profile+0xb0 > 5`): `IMAGE_BONUSAWARD` / `BONUSAWARD2` at (170, 310) and sparkles. Not shown on a fresh profile.
3. The mermaid's blink `IMAGE_MERYLBLINK` at (139, 196) and tail flop `IMAGE_MERYLTAIL` at (39, 301), each at its animation's current cel (both are `anim="once"` in `resources.xml`).
4. The speech bubble (`0x0051e280`): `IMAGE_SELECTORSPEECHBUBBLE` at (45, 20). Inside, either the welcome (`IMAGE_WELCOMEBACK` or `IMAGE_WELCOMETO` centred at y 33, the profile name in `FONT_BLAMBOTPRO15` at y 77 in (0, 0, 100), and "If this is not you, click here." under it), or the hovered button's hint in `FONT_BLAMBOTPRO8`, word-wrapped in the rect (64, 40, 214, 60), vertically centred. When the hover changes, the old text fades out over 5 updates while the new one fades in (`0x00517610`; the change only takes after the hover has differed for 5 updates).

Hint texts (`0x0051e280`), by hovered button: Adventure "New to Insaniquarium?\nClick here to start your aquatic adventure!" at 1-1 with the game not beaten, else "Feed fish and fight aliens!\nClick here to continue\nyour adventure..."; Time Trial "How much money can you earn before time runs out?"; Challenge "Can you fend off the increasingly difficult aliens?"; Virtual Tank "Buy and raise your own\ncustom fish, then use them\nas a screensaver!"; the trophy "Wow!  Nice trophy!" (or the long champion text). `static`; Adventure, Time Trial and Virtual Tank hints are `confirmed` (`h_adv`, `h_tt`, `h_vt`).

The blink and tail timers draw from the **app MTRand stream** (`app+0x7b0`): at construction `r % 150 + 100` (blink) then `r % 500 + 200` (tail); in Update, when the blink timer runs out, `r % 7 == 0` picks a quick re-blink `r % 15 + 15`, else `r % 150 + 100`; the tail timer reloads `r % 500 + 200`. `static`. This stream is the one the levels use, so menu time changes a level's RNG (see Deviations).

## Buttons

`GameSelector::ButtonDepress` (`0x0051f160`). `static`:

- **1 Adventure:** game mode 0 (`app+0x87c`), `0x0054b360`, then `0x00552380(1, 1)` = start Adventure at the profile's level. Tank 1 levels 1–3 open the **Instructions** help screen first (`0x0054c220(1)`); its Continue starts the level. `confirmed`: the Instructions screen came up on 1-1 (`04_instructions.png`, `12_after_adv.png`).
- **2 Virtual Tank, 8 Time Trial, 9 Challenge:** other modes. Time Trial needs a tank completed (`profile+0x1c ≥ 2` or beaten) and Challenge needs the game beaten; otherwise a "Not Yet!" dialog (id `0x0e`, button "OK").
- **3 Options:** the Options dialog (`0x0054c620(1)`, `screens/options-dialog.md`).
- **4 Quit:** dialog `0x0b` "Quit" / "Stop the insanity?" with two buttons, QUIT and CANCEL. `confirmed` (`21_quit_confirm.png`). QUIT closes the game.
- **6 Hall of Fame, 7 Help:** their screens.

## Music

Construction: unless music is already on track 2 order 0, stop all music and play `(2, 0)` (`0x0054b1a0(2, 0, 0)`). By `music.md`'s remap that is track 4 (`Insaniq2.mo3`) order `0x2d`. `static`.

## No profile

With no `userdata/users.dat` profile, the GameSelector opens a **NEW USER** dialog ("Please enter your name.", a text box, OK / CANCEL). CANCEL brings up "Enter Your Name" ("Please enter your name to create a new user profile for storing high score data and game progress.", OK only), so you can't play without a profile. With no profile the bubble shows no welcome, and the buttons have no second lines. `confirmed` (`23_noprofile.png`, `24_cancel_user.png`).

## Deviations

- **No Instructions screen:** the original opens it before 1-1 to 1-3 (Adventure → Instructions → Continue → level). v0.1.0 goes straight into the level.
- **v0.1.0 builds a reduced menu:** the background, the four big buttons with their hover/press art and the four small buttons, Adventure and Quit active. Time Trial, Challenge, Virtual Tank, Hall of Fame, Options and Help are drawn but inactive (the original's "Not Yet!" dialog fits Time Trial and Challenge, but not the rest). No profiles: there's no name, no welcome and no NEW USER dialog; the bubble shows only the hovered button's hint. No trophy, no mermaid blink/tail animation, no hint cross-fade (the hint switches at once), and Adventure always reads "Start Adventure" with no tank line. Hover/press art and labels follow the spec; not frame-checked yet.
- **Menu RNG:** the original's menu draws from the app stream while it is up, so the next level's RNG depends on how long the menu stayed open. Our level seeds come from the app at start, which already differs from a real session; nothing is lost by not running the menu's draws.
