# Pet screen ("Choose Your Pets", `Sexy::PetsScreen`)

Between Adventure levels, a player who owns more than 3 pets picks which ones to take into the next level. The screen shows a 24-bubble grid. Owned pets are clickable and toggle on a click, up to 3. The centre panel shows the hovered pet's picture, name and description. Continue starts the level with the chosen pets.

Class `Sexy::PetsScreen` (Ghidra's `PetScreenOverlay::PetScreenOverlay` @ `0x00527820` is really its constructor), vtable `0x005a1044`, object size `0x138`, held at `app+0x80c`. Raw `this`-relative offsets.

Confidence: `traced` overall, per section below. The capture `traces/original/petscreen_01.trace.jsonl` records the screen's raw object and the profile's owned/chosen bytes on every tick of one session (5 pets owned, after 1-5). The session covers:
- hover in and out;
- Continue with nothing chosen, then No;
- three picks, a refused 4th, an un-pick and a re-pick;
- Continue into 2-1.

`tank1_1.rs` `pet_screen_matches_original` replays it.

## When it appears

`App::NextLevel` (`0x00552380`) clears the chosen bytes `profile+0x5a..+0x71`. Then, when the owned count `profile+0x18` is **> 3** (and it isn't a bonus round, Virtual Tank, or tank 5 in Adventure), it calls `App::ShowPetScreen` (`0x0054c2d0`) instead of `StartLevel` (`screens/hatch-screen.md#appnextlevelresume0-help0--0x00552380`). Starting Adventure from the main menu goes the same way.

- After tank 1, `NextLevel` shows a bonus round first; the pet screen follows its results screen.
- `ShowPetScreen` kills dialogs 0x1f/0x20, removes any old pet screen, builds a new one at (0, 0, 640, 480) and adds it to the widget manager.

`confirmed` (both entries seen while playing: after 1-5 → bonus → pet screen, and from the menu at 2-1).

## Fields

| Offset | Meaning | Initial |
|---|---|---|
| `+0x90`, `+0x94` | Continue (id 99), Menu (id 100) buttons | |
| `+0x9c + 4·id` | the 24 pet buttons | |
| `+0x120` | **tick counter** `t` | 0 |
| `+0x124` | **pets chosen** | 0 |
| `+0x128` | **pet shown** in the panel (−1 = none) | −1 |
| `+0x12c` | **fallback pet** (shown when a fade ends) | −1 |
| `+0x130` | **most pets you may choose** | `min(profile+0xb4, owned count)`, capped at 4 outside Virtual Tank |
| `+0x134` | **fade countdown** | 0 |

`profile+0xb4` is 3 for a fresh profile, so the max is 3. What raises it is unknown.

No pet starts chosen: the constructor clears `profile+0x5a + id` for all 24 ids.

`traced` for `+0x120..+0x134` (compared every tick). The `+0xb4` source of the max is `static`.

## Update (`0x00518b00`), each tick

1. If the fade countdown is non-zero, decrement it. When it reaches 0, **shown = fallback**.
2. `t += 1`.

**No RNG** in the screen at all (constructor, update, input, draw): both streams are untouched from the hatch screen's last tick until `StartLevel`.

`traced` (the counter, the 7-tick fade, and the app RNG position, flat through the whole session).

## Input

**Mouse enter on a pet button** (`0x00518b30`): shown = that pet, and the fade is cancelled (countdown 0).

**Mouse leave** (`0x00518b50`): if that pet is **not chosen**, countdown = 7. (Update then counts it down to 0, and the panel falls back.) Leaving a chosen pet does nothing, so its info stays up. In Adventure the fallback stays −1: the panel empties.

**Click on a pet** (mouse-up over the button, `PetButtonWidget::MouseUp` @ `0x00532f70`; the mouse-down plays `SOUND_BUTTONCLICK`):
- chosen → un-choose it, count − 1;
- not chosen and count < max → choose it, count + 1;
- otherwise nothing (a **4th pick is refused** silently).
- Unowned bubbles are disabled and never toggle.

**Continue** (mouse-up on id 99, `ButtonDepress` @ `0x005289f0`):
1. Copy every button's chosen flag into **`profile+0x5a + id`**. This happens even when the dialog follows.
2. If count ≥ **3**: remove the screen and **`App::StartLevel`**. `PopulateLevel` adds the chosen pets in id order (`sim/level-start.md`, step 3).
3. Otherwise open **dialog 0x14 (20)**: "Select More Pets?" / "Are you sure you want to continue to the next level without selecting 3 pets?" (with max > 3: "…without selecting at least 3 pets?"). Buttons **Yes** / **No**:
   - Yes: remove the screen and `StartLevel` with the pets chosen so far, even none.
   - No: close the dialog and stay on the screen.

**Menu** (id 100): remove the screen and go to the main menu.

There's no keyboard handling.

`traced`:
- enter/leave/fade;
- the cap at 3;
- toggling;
- dialog 20 on Continue with 0 chosen, then No;
- Continue with 3 chosen → 2-1 starts with Stinky, Prego and Zorf.

Yes, Menu and the disabled bubbles are `static`.

## Layout and drawing

The screen's own Draw is empty. The pet buttons draw themselves, and the rest is the overlay widget's Draw (`0x00528690`). The overlay is added after the pet buttons and before Continue and Menu, so it covers the bubbles and sits under the two buttons.

**Pet buttons:** 90×83 each, at these positions (x by column, y = 41 + 83·row):

| ids | x |
|---|---|
| 0–4 | 25 |
| 5–9 | 119 |
| 20–21 (rows 3–4) | 226 |
| 22–23 (rows 3–4) | 323 |
| 10–14 | 425 |
| 15–19 | 519 |

- **Art:** `IMAGE_PETBUTTON` cut into 4 cels across. Cel 0 disabled (unowned); 1 normal (purple); 2 pressed and chosen (yellow); 3 hover (cyan).
- **Chosen:** the button swaps its normal and pressed cels and loses its hover cel, so a chosen pet is yellow even under the mouse.
- **Icon:** owned pets draw `IMAGE_SCL_<PET>` at (+14, +10) (Amp: (+15, +24)), animated by the widget's update count.

**Overlay**, in order:
1. A background image built once in the constructor:
   - `IMAGE_SCREENBACK` boxed over (−5, −5, 650, 490);
   - `IMAGE_SCREENTITLE` boxed at (20, 0, 600, h);
   - `IMAGE_SCREENTITLEHOLE` boxed around Menu, (524, 3, 82, h);
   - `IMAGE_FISHBOX` at (216, 48) and `IMAGE_FISHBOXBUTTON` at (221, 243);
   - then a hole cut where each pet button sits (the `IMAGE_PETBUTTONHOLE` shape made transparent), so the bubbles show through.
2. For each of the 24 buttons: `IMAGE_PETBUTTONRING` at its position. Enabled buttons also get `IMAGE_PETBUTTONREFLECT` there.
3. Title "Choose Your Pets", `FONT_JUNGLEFEVER17OUTLINE`, (255, 200, 0), at (215, 25).
4. The **panel**, with c = fade countdown:
   - 1 ≤ c ≤ 4: the shown pet at alpha `c·255/5`, then the fallback pet (if any) at `255 − that`;
   - otherwise: the shown pet at full alpha.

   Each pet, `0x00527ec0`:
   - picture: its full sheet (`IMAGE_<PET>`), row 0, column f, at (280, 90) (Niko (280, 85), Amp (240, 90)), from tick t:
     - ids 1 and 6: `t % 18`, folded to `18 − f` above 9;
     - ids 5, 10, 11, 14: `(t % 40) / 4`;
     - others: `(t % 20) / 2`;
   - name (mixed case, "Prego"): `FONT_JUNGLEFEVER15OUTLINE`, (255, 200, 0, alpha), centred at y 180;
   - description: three lines, `FONT_TINY`, white, centred at y 200, 215, 230. The wording is the hatch screen's.
5. **Prompt:** `FONT_TINY`, white, centred at y 75 and 90.
   - Line 1, with n = max − count: "Choose %d more pets" (n > 1), "Choose 1 more pet" (n = 1), empty (n ≤ 0).
   - Line 2: "to take to the next level" (empty when n ≤ 0).
6. Continue: `DialogButton` "Click Here To Continue", `IMAGE_MAINBUTTON`, (225, 250, 186, 29), `FONT_JUNGLEFEVER10OUTLINE`. Menu: (525, 4, 80, 29).

"Centred" is `x = ((650 − width) >> 1) − 3`, as on the hatch screen.

`traced` by frame (`frames.rs` `pet_screen_frames_match_original`, four frames of `pets_01`, pixel-exact):
- the background with its bubble holes, rings, reflections, title, prompt, Continue and Menu;
- the bubble cels: purple, cyan on hover, yellow when chosen (also under the mouse);
- the pet icons with their manifest animations (Niko's ping-pongs);
- the panel's picture, name and description;
- the prompt vanishing at 3 chosen.

The cross-fade alphas, Amp's offsets and the disabled-bubble cel are `static`. The capture doesn't record the cursor, so the test supplies the mouse positions the frame driver used.

## Sounds and music

`SOUND_BUTTONCLICK` on mouse-down on any button. The music isn't touched: the previous screen's track keeps playing. `static`.
