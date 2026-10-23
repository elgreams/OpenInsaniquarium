# Board dialogs (`MoneyDialog`)

Status: `draft` (static). Covers the in-tank message boxes ("DANGER!", "YOUR LAST FISH HAS DIED!", "GAME OVER", "OUT OF MONEY!", and the other `Board::Update` hints).

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

## Drawing (`Dialog::Draw`, slot 26 @ `0x00476710`)

1. The **panel image drawn as a 3×3 grid** (`0x00456430`, SexyApp "DrawImageBox"):
   - The source is cut into thirds, 80×80 each for this panel.
   - Corners are drawn at the corners of the dialog rect.
   - Edges and the centre are **tiled** (repeated, clipped), not stretched.
2. The header text, horizontally centred, at the top content inset plus the font's ascent.
3. The body text, word-wrapped to the content width, centred, below the header (header line spacing + 10).
4. The button at the bottom (inset by the content margins, height = button image height). It is drawn with the same 3×3 box routine from the button image's current state cel, with its label centred.

Text wrap details (`Widget` slots 63/65) are approximated: greedy word wrap, centred lines.

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
| 0x0e | `WARNING!` | `A new breed of alien is fast approaching!  Lasers can't hurt this baddie, so find another way to defeat it!` | 1-3, countdown 276, once (`+0x4c1`) |
| 0x10 | `YOUR LAST FISH HAS DIED!` | `Normally this would end your game, but this time we'll give you another fish to keep playing! Make sure you keep it well fed!` | tutorial 1-1 |
| 0x11 | `GAME OVER` | `Oops!  All of your fish have died!` (or `… pets …`) | no fish left |
| 0x13 | `OUT OF MONEY!` | `You've run out of money!  Because you're new, we'll float you a loan, but be careful next time!` | tutorial, money 0 |

All of them use the button `Click to Continue`.

The three hints are chosen in order at countdown 276, when not in Virtual Tank mode and the profile hasn't completed the game (`0x00537bb0(tank, level)`). Each sets its board byte when shown, so a hint appears at most once per level. Its board byte is cleared at level start. `confirmed`: `tank1-2_full_02` paused at 276 on both waves.

A click anywhere other than the button does nothing. The button fires on mouse-up.
