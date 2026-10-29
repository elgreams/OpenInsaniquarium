# Options dialog (the in-game Menu) and Leave Game?

The tank's **Menu** button opens the **Options** dialog. The same dialog opens from the main menu's Options button, minus Back to Main Menu. There is no separate pause menu, and no "Restart Level" in this dialog. (The strings "Restart Level" / "Restart Level?" exist, but for another flow, not this dialog.)

Status: per section. Layout and flow are `confirmed` by screenshots of the original (`traces/frames/menus_01/`, 2026-10-28; local, gitignored); constructor details are `static`.

## The tank's Menu button

A `ButtonWidget` owned by the Board, id **0x7b** (`Board::Board`, around `0x00542c71`), rect **(525, 3, 101, 29)**. Images: up `IMAGE_BLANK` (the orange "Menu" plate is part of `IMAGE_MENUBAR`), over `IMAGE_OPTIONSBUTTON` (the blue plate), down `IMAGE_OPTIONSBUTTOND`. `static`; the hover image over the plate is `confirmed` (`08_menubtn_hover.png`: changed pixels in (534, 7)–(620, 28)).

On release over it (`ButtonDepress`, Board's listener slot 3 @ `0x00538070`): id 0x7b → `App::ShowOptions(0)` (`0x0054c620`). `static`; that the dialog opens is `confirmed`.

## Dialog

`App::ShowOptions(fromMenu)` @ `0x0054c620`: closes the Presto dialog if open (`0x0054b450(−1)`), builds an `OptionsDialog` (`0x005349d0`; a `MoneyDialog` with header **"Options"**, no body, footer button **"OK"**), sizes it to **width 348** at **(143, 26)** with the height from slot 73 (`0x00530e60`), and adds it as dialog **id 1**. `static`. The board is frozen while it is up: two screenshots 1.5 s apart were identical. `confirmed` (`16_opt_a.png`, `17_opt_b.png`).

Contents, top to bottom (`OptionsDialog::Resize` slot 40 @ `0x00531030`, `Draw` slot 26 @ `0x00534f60`). Rects are relative to the dialog unless marked. `static` for the numbers, `confirmed` for the look (`09_menu_dialog.png`):

- Labels in `FONT_JUNGLEFEVER10OUTLINE`, in the dialog's line colour: **"Music"** at (48, 78), **"Sound Fx"** at (48, 110) (baselines).
- Two `Slider`s (`IMAGE_SLIDERTRACK`, `IMAGE_SLIDERTHUMB`), ids 5 (music) and 6 (sound), at (152, 58) and (152, 90), width = dialog width − 192, height 33. They start at the app's music and sound volumes.
- Three `Checkbox`es, 46×45: id 7 **Fullscreen** at (33, 128), id 8 **Custom Cursors** at (161, 128), id 9 **Hardware Acceleration** at (33, 172). Each label is drawn at checkbox + (43, 24).
- A column of full-width buttons (x = left inset, width = dialog width − both insets, height 33, 34 px apart), stacked **upward from the OK button**: Back to Main Menu (id 3), then Help (id 1); Register (id 0), Check Updates (id 4) and a web link (id 2) are hidden in this build's state. On screen: **Help**, **Back to Main Menu**, **OK**. Back to Main Menu's hover changes (187, 290)–(442, 311) on screen, i.e. the button is at screen y ≈ 290.
- With `fromMenu` (the main menu's Options) **Back to Main Menu is hidden**.
- Buttons use the game's `DialogButton` art (`IMAGE_DIALOGBUTTON`, purple; blue on hover). `confirmed` (`10_dlg_hover.png`).

## Buttons

`OptionsDialog::ButtonDepress` (listener slot 2 @ `0x00535430`, ids 0..4) and the app's dialog-button handler for the footer. `static` unless noted:

- **OK** (footer): close the dialog; the board runs again.
- **Help** (1): the help screen (`0x0054c220(0)`).
- **Back to Main Menu** (3): if the game mode isn't 5 and a board exists and **`board+0x4ee`** is set ("level in progress": any fish has eaten, `0x004d6a30`, or the board turned into a bonus round), open **Leave Game?** (`0x0054e760`). Otherwise leave at once (`0x00552230`). `confirmed`: right after starting 1-1 it went straight to the main menu (`11_confirm.png`); after the fish had eaten it asked (`18_leave.png`).
- Sliders and checkboxes change the volume / screen settings at once.

## Leave Game?

`0x0054e760`: dialog **id 0x16**, header **"Leave Game?"**, body **"Do you want to return to the\nmain menu?\n\nYour game will be saved."**, two buttons labelled **LEAVE** and **CANCEL**. Standard dialog placement (`framework/dialog.md`: at (143, 142), width 348), opened **over** the Options dialog. `confirmed` (`18_leave.png`: LEAVE around (244, 285), CANCEL around (385, 285)).

- **LEAVE** (id 2000 + 0x16 in the `WinFishApp` handler `0x005527c0` → `0x00552230`): save the level (`0x0054afd0`), `0x0054ba40`, destroy the board (`0x0054bc30`), show the main menu (`0x00552100`). `confirmed` (`20_menu_after_leave.png`).
- **CANCEL**: close Leave Game?; the Options dialog stays up. `confirmed` (`19_after_cancel.png`).

## Deviations

- v0.1.0: the sliders and checkboxes are drawn but inactive (no settings are stored yet); Help is inactive. OK, Back to Main Menu (with Leave Game? when a fish has eaten) and LEAVE / CANCEL work.
- No saves: LEAVE doesn't save the level. The body text still says "Your game will be saved." in the original; we say so in the release notes rather than change the text.
