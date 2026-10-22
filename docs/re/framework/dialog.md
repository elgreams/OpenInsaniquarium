# Board dialogs (`MoneyDialog`)

Status: `draft` (static). Covers the in-tank message boxes ("DANGER!", "YOUR LAST FISH HAS DIED!", "GAME OVER", "OUT OF MONEY!", and the other `Board::Update` hints).

## Creation

`WinFishApp::DoDialog` (`0x0054b5b0`, args: id, modal, header, body, button text, button mode) → `SexyAppBase::DoDialog` → `WinFishApp::NewDialog` (slot 26 @ `0x0054b940`), which builds a **`MoneyDialog`** (`0x00530920`, a SexyApp `Dialog`). It then sets 30 as some time-out value (slot 76 `0x1e`, TODO) and adds the dialog to the widget manager.

- Images: panel `IMAGE_DIALOG` (`dialogue-panel`, 240×240), button `IMAGE_DIALOGBUTTON` (`dialogbutton2`, 288×33: three 96×33 states, normal / hover / pressed).
- Fonts (`0x00504060`):
  - header `FONT_JUNGLEFEVER12OUTLINE`;
  - body (lines) `FONT_JUNGLEFEVER15OUTLINE`;
  - button `FONT_JUNGLEFEVER10OUTLINE`.
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

## Board dialogs

| Id | Header | Body | When |
|---|---|---|---|
| 0x0e | `DANGER!` | `A vicious alien is about to enter your tank! Defeat it with your laser weapon by clicking on it!` | 1-2 countdown 276 (`alien-sylvester.md`) |
| 0x0e | `BATTLE TIP` | `Shoot the alien's head to push it downwards! …` | later hint |
| 0x0e | `WARNING!` | `A new breed of alien is fast approaching! …` | later hint |
| 0x10 | `YOUR LAST FISH HAS DIED!` | `Normally this would end your game, but this time we'll give you another fish to keep playing! Make sure you keep it well fed!` | tutorial 1-1 |
| 0x11 | `GAME OVER` | `Oops!  All of your fish have died!` (or `… pets …`) | no fish left |
| 0x13 | `OUT OF MONEY!` | `You've run out of money!  Because you're new, we'll float you a loan, but be careful next time!` | tutorial, money 0 |

All of them use the button `Click to Continue`.

A click anywhere other than the button does nothing. The button fires on mouse-up. (Our app currently accepts any click; TODO: hit-test the button.)
