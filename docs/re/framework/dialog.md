# Board dialogs (`MoneyDialog`)

Covers the in-tank message boxes ("DANGER!", "YOUR LAST FISH HAS DIED!", "GAME OVER", "OUT OF MONEY!", and the other `Board::Update` hints).

Status: per section. Drawing is `confirmed` by the frame diff (GAME OVER, pixel-exact); the hint dialogs' timing is `traced` on 1-2.

## Creation

`WinFishApp::DoDialog` (`0x0054b5b0`, args: id, modal, header, body, button text, button mode) → `SexyAppBase::DoDialog` → `WinFishApp::NewDialog` (slot 26 @ `0x0054b940`), which builds a **`MoneyDialog`** (`0x00530920`, a SexyApp `Dialog`). It then sets 30 as some time-out value (slot 76 `0x1e`, TODO) and adds the dialog to the widget manager.

- Images: panel `IMAGE_DIALOG` (`dialogue-panel`, 240×240), button `IMAGE_DIALOGBUTTON` (`dialogbutton2`, 288×33: three 96×33 states, normal / hover / pressed).
- Fonts (`0x00504060`, through `Dialog` slots 70/71/72 = SetButtonFont `0x00476360`, SetHeaderFont `0x004763b0` → `+0x6c`, SetLinesFont `0x004763e0` → `+0x70`):
  - header `FONT_JUNGLEFEVER15OUTLINE`;
  - body (lines) `FONT_JUNGLEFEVER10OUTLINE`;
  - button `FONT_JUNGLEFEVER12OUTLINE`.
- Colours: header (255, 200, 0), body (255, 255, 255), button text (255, 255, 100).
- Content insets `(36, 15, 36, 36)` (left, top, right, bottom); the space between lines is set to 15.
- **Size** (`NewDialog`): width `w = header font's text width + 150`, at least 348. Height = the dialog's preferred height for that width, at least `2/3 · DIALOGBUTTON height + 10`. **Position (143, 142)**, i.e. near the centre of the 640×480 board.
- Button mode 1 is a single button whose label is the "button text" argument ("Click to Continue" for every board hint).
- **Two buttons** (`Dialog::Dialog` @ `0x00475c40`, button modes 1/2 with Yes/No or OK/Cancel ids 1000/1001), e.g. the pet screen's "Select More Pets?" (YES / NO): `Dialog::Resize` (`0x00476af0`) splits the content width into two buttons `(inner − 8) / 2` wide, 8 px apart. The dialog comes out **13 px taller** than the one-button height rule above gives; the text stays in place and the buttons sit 13 px lower. `confirmed` by the frame diff (`pet_screen_dialog_frames_match_original`, pixel-exact); where the 13 px come from in `GetPreferredHeight` (`0x00476570`) is `guess`.

`confirmed` (size 348×170 and position read live, and matched by the frame diff).

## Drawing (`Dialog::Draw`, slot 26 @ `0x00476710`)

1. The **panel image drawn as a 3×3 grid** (`0x00456430`, SexyApp "DrawImageBox"):
   - The source is cut into thirds, 80×80 each for this panel.
   - Corners are drawn at the corners of the dialog rect.
   - Edges and the centre are **tiled** (repeated, clipped), not stretched.
2. The header text, horizontally centred, at the top content inset plus the font's ascent.
3. The body text, word-wrapped to the content width, centred, below the header (header line spacing + 10).
4. The button at the bottom (inset by the content margins, height = button image height). It is drawn with the same 3×3 box routine from the button image's current state cel, with its label centred.

Text wrap details (`Widget` slots 63/65) are approximated: greedy word wrap, centred lines.

`confirmed` by the frame diff for a one-line body (GAME OVER). Multi-line wrapping (the hint dialogs) is `static`: no frame captured with one up.

## Buttons

`DialogButton::Draw` (`0x00476f70`), when the button has a font, draws its label at:

- pen x = `(w − StringWidth(label)) / 2 + textOffsetX` (`+0x12c`);
- baseline y = `(h − ascent/6 − ascentPadding − 1 + ascent) / 2 + textOffsetY` (`+0x130`).

Both divisions are C integer divisions, and the coordinates are relative to the button. `ImageFont::StringWidth` (slot 7 @ `0x0045bf50`) sums `CharWidth(c, prev)` (slot 9 @ `0x0045bfd0`): the glyph width plus the kerning pair (prev, c) plus the layer spacing (0 in every game font), times the point-size scale (1 at the default size).

The game's button setup helper (`0x005034e0`) sets the text offset to **(−2, 0)**. It also sets the three image cels (up / over / down = thirds of the button image) and draws the image as a 3×3 box. Every game button goes through it: the dialog buttons (from `0x00504060`), `0x00506820`/`0x00506760` (hatch screen, help, pet, tank, high-score, sim and bonus screens) and the game selector. `confirmed`: `+0x12c = −2` on both hatch-screen buttons live, and the hatch-screen frames match pixel for pixel with it.

## Board dialogs

| Id | Header | Body | When |
|---|---|---|---|
| 0x0e | `DANGER!` | `A vicious alien is about to enter your tank! Defeat it with your laser weapon by clicking on it!` | 1-2, countdown 276, once (`board+0x4b3`) |
| 0x0e | `BATTLE TIP` | `Shoot the alien's head to push it downwards! Shoot its tail to deflect it upwards!` | 1-2, countdown 276, the next wave (`+0x4b4`) |
| 0x0e | `WARNING!` | `A new breed of alien is fast approaching!  Lasers can't hurt this baddie, so find another way to defeat it!` | **2-3** (`0x00537bb0(2, 3)` @ `0x005485f6`), countdown 276, once (`+0x4c1`) |
| 0x10 | `YOUR LAST FISH HAS DIED!` | `Normally this would end your game, but this time we'll give you another fish to keep playing! Make sure you keep it well fed!` | tutorial 1-1 |
| 0x11 | `GAME OVER` | `Oops!  All of your fish have died!` (or `… pets …`) | no fish left |
| 0x13 | `OUT OF MONEY!` | `You've run out of money!  Because you're new, we'll float you a loan, but be careful next time!` | tutorial, money 0 |

All of them use the button `Click to Continue`.

The three hints are chosen in order at countdown 276, when not in Virtual Tank mode and the profile hasn't completed the game (`0x00537bb0(tank, level)`). Each sets its board byte when shown, so a hint appears at most once per level. Its board byte is cleared at level start. `traced`: `tank_1_2_music_and_dialogs_match` compares the dialog id every tick (both 1-2 waves). The 2-3 row is `static` (corrected 2026-10-23: it was wrongly listed as 1-3).

A click anywhere other than the button does nothing. The button fires on mouse-up.

## Button delay

Every board dialog's button **ignores clicks for its first 30 updates**. `WinFishApp::DoDialog` (`0x0054b5b0`) calls the new dialog's slot 76 with 30. `MoneyDialog`'s slot 76 (`0x00530a30`) stores it at `+0x154` and disables both buttons. Its Update (slot 22, `0x00530a70`) counts the widget's updates (`+0x28`) and re-enables the buttons once the count reaches the stored value. The dialog updates every tick while the board is paused under it, so the button works from the 31st tick after the dialog opened.

`traced` by `tank_1_2_from_demo` (inputs from the game's own demo):
- the "DANGER!" dialog opened on update 3214;
- clicks on its button at updates 3220 and 3241 did nothing;
- the click at 3262 closed it.

The sim counts board updates with the dialog up (`Board::dialog_age`) and ignores a dismiss below `DIALOG_BUTTON_DELAY`.

## Game Paused (dialog 0x12)

`App::ShowPaused` @ `0x0054eca0`: if a board exists, the app isn't in demo playback (`app+0x358`, `0x00479fc0`) and the board isn't already paused (`board+0x94`), it pauses the board (`0x0053db80(1)`), closes the Presto dialog (`0x0054b4c0`), and opens dialog **0x12**: header **"GAME PAUSED"**, body **"Click to resume game"**, button mode 3 with the label **"Resume Game"**. `static`.

Callers: `WinFishApp` slot 83 (`0x00551d50`, the app losing focus: with a board and not in Virtual Tank, mode 5) and the board's key handler (`0x00549d70`, key `0x20`, space). `static`; focus loss is `confirmed` (`traces/frames/menus_01/13_paused_raw.png`) and `traced` in the replays (`dynamic-analysis.md`).

- Regaining focus doesn't close it; only its button does. `traced` (replay harness: `DemoEvent::Activate(true)` is ignored, and the captures resume on the click).
- Layout: the standard one-button dialog (above): panel at (143, 142), width 348. The Resume Game button spans screen (187, 248)–(442, 269). `confirmed` (hover diff between `14_paused_mouse_off.png` and `15_paused_hover.png`).
- The button is purple, blue on hover (`IMAGE_DIALOGBUTTON` cels). `confirmed`.
- Its button resumes the board (`WinFishApp` handler, id 0x12 → close dialog). `static`; the resume is `traced`.

## GAME OVER button

Dialog 0x11's button ("Click to Continue") goes to the main menu: the `WinFishApp` dialog-button handler (`0x005527c0`, jump table at `0x00552d48`, case 0x11 @ `0x00552a94`) closes the dialog, destroys the board (`0x0054bc30`) and shows the GameSelector (`0x00552100`, `screens/game-selector.md`). There is no restart. `static`. The 30-update button delay ([Button delay](#button-delay)) applies as for every board dialog. `static` for GAME OVER specifically.

Our sim ignored this button until v0.1.0 (the board stayed paused under GAME OVER).
