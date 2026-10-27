# Presto the tadpole (`Sexy::FishTypePet`, pet id 19)

Presto swims like a guppy. His power is **player-chosen**: **right-clicking** him opens the "PRESTO CHANGE-O" dialog, which offers every pet the profile owns; picking one replaces Presto with a brand-new pet of that kind at his spot (`GROW` sound, a 20-tick smoke puff). The new pet keeps a "Presto" flag: it has a green **"Presto"** label under it, and right-clicking it opens the dialog again, but only after a **360-tick (10 s) recharge** (a red bar sweeps across the label; clicking early plays `BUZZER` and floats "Recharging..."). Picking Presto himself turns it back into the tadpole. There is no RNG in the choice and no timer that forces a change; the only automatic change is the revert when the disguised pet is knocked by the boxing glove (pet 23).

Same class, update and fields as Itchy (`pet-itchy.md`). Only the id-19 / Presto-flag differences are listed here. All offsets are raw `this`-relative.

- `Board::AddPet` @ `0x00544a90`: forces the flag argument to 1 for id 0x13
- Ctor `FishTypePet::FishTypePet` @ `0x004ef420` (flag → `+0x230`, recharge `+0x238`)
- Right-click: FishTypePet slot 54 @ `0x004fbe20` → `0x004fb680` (first block); OtherTypePet equivalent `0x004f3a00`
- Recharge check `0x004f2650`; open dialog `0x0054c6f0` (dialog id `0x1f`, `PetDialog` ctor `0x005355c0`); dialog click slot 56 @ `0x00532a90` → hit test `0x005329f0` → `0x0054b450` (close and apply)
- Find the Presto pet `0x00539540`; change: FishTypePet slot 73 @ `0x004f89a0`
- Revert on a punch: Update `0x004f7ad0`, push-timer step; the glove `BoxingGlove::Update` @ `0x004e9a60` → `0x004e0ed0`
- Drawing: draw body `0x004f8610` case 0x13 → `0x004dfbb0` with `IMAGE_PRESTO`; puff and label at the end of `0x004f8610`; label `0x004f2450`

Confidence per section below. `presto_01` (1-2, Presto alone; replayed by `presto_changes` in `crates/harness/tests/tank1_1.rs` from the game's own demo) right-clicks him, changes him into Itchy, Shrapnel, Stinky and back into Presto, and right-clicks each changed pet once while it recharges. The punch revert (pet 23) and the drawing are not covered. `traced` where a section says so, else `static`.

## Fields

| Offset | Type | Meaning |
|---|---|---|
| `+0x230` | byte | **Presto flag** (Ghidra `FishTypePet_data.offset_0x4`; the earlier specs call it the "drop-in flag"): set on Presto (AddPet forces it) and on every pet he turns into, 0 on ordinary pets |
| `+0x238` | int | **recharge**: 360 at construction when the flag is set and the id isn't 19 (0 for Presto himself and in the Virtual Tank, `app+0x87c == 5`); −1 per tick in the update (step 9, Itchy's "drop-in countdown") until 0 |
| `+0x248` | int | puff counter (Ghidra `offset_0x1c`): 0 at construction, +1 per tick while < 20, for flagged pets (and id 21) |
| `+0x20c` | int | the Fish push timer (set by the boxing glove) |

On an `OtherTypePet` (Stinky, Niko, Rufus, Clyde, Rhubarb) the same roles are played by byte `+0x1c8` (flag), int `+0x1ac` (recharge, 360 unless id 19, 0 in the Virtual Tank) and `+0x1a8` (puff). Its ctor (`0x004eb5c0`) skips the floor y of ids 0/7/14 and Niko's fixed spot when flagged, and its update (`0x004f2e80`) makes flagged ids 0, 1, 7, 14 sink (`vy += 0.1` while y < 380).

Pet ctors that branch on the flag: Gash's hunger clock starts at **1520** instead of −1550 (`pet-gash.md`), so a fresh Gash made by Presto is hungry after 51 calm ticks.

`traced` by `presto_changes` (`presto_01`): the flag and recharge on fish pets and on Stinky (`+0x1c8`/`+0x1ac`), the puff counter.

## Construction

`Board::AddPet(19, -1, -1)`: the flag is forced to 1, then the usual **9 app draws** (2 position, 6 `Fish::Init`, 1 type-0 shadow, `% 5`). No id-19 ctor branch; recharge 0 (id 19), puff counter 0.

`traced` by `presto_changes` (`presto_01`) (his draws at level start).

## Update (`0x004f7ad0`), id 19

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`): the gate and slot 81 have no id-19 branch, so he always swims; the animation is the default (cycle / 2). Additionally, for any flagged pet:

- **Puff:** `+0x248 += 1` while < 20.
- **Recharge:** `+0x238 −= 1` while non-zero (Itchy's step 9).
- **Punch revert** (in step 7, the push timer): after the push timer is decremented, if the pet is flagged, its id isn't 19 and the new timer value is < 41 (so from the 10th tick of a 50-tick glove push), set `+0x238 = 0` and call slot 73 with **19** (the change below): it turns back into Presto, and its update returns at once.

`traced` by `presto_changes` (`presto_01`) for swimming, the puff and the recharge. The punch revert is `static`.

## How a right-click reaches him

Presto (like every pet) isn't mouse-visible (`+0x55` = byte `0x005e8f14`, 0), so a right-click lands on whatever mouse-visible widget is on top: a coin, a guppy, food or the board. All of them pass a right-click (negative click count) to **`0x0053bfb0(x, y)`** in tank coordinates (`Coin` slot 55 `0x004e5450`, `Food` `0x004d62f0`, `Breeder`, the board's own mouse-down `0x00543f00` when `y > 40`):

1. `0x00539630(x, y)` (re-entry guard `0x005e9630`): walk `+0xb4` in list order for the **first pet with the Presto flag `+0x230` whose 80 × 80 widget rect contains the point** (slot 69, `Widget::Contains`); if none, the same over `+0xb0` with flag `+0x1c8`. If one is found, call its mouse-down (slot 54) with the pet-relative point and click count −1, and return true.
2. If no flagged pet was hit: play **`SOUND_TAPGLASS`** directly through the app (`app` slot 54 with `0x005e8b68`; not the board's throttled sample table, so no `samples` entry). In the Virtual Tank (`app+0x87c == 5`) the fish also react (not followed).

Unflagged pets are skipped: a right-click on a pet without the flag is just a glass tap.

`traced` by `presto_changes` (`presto_01`) for the lookup (right-clicks on Presto pets, every one landing on him). The glass tap isn't compared (no sample-table entry); the Virtual Tank branch is `static`.

## Right-click (`0x004fb680`, first block)

Slot 54 passes the click to `0x004fb680`; a negative click count is the **right button**. For a right-click:

- **Not flagged** (`+0x230 == 0`): the click goes to the board as a tank click at that spot (`0x0053bfb0`). (Every pet; nothing Presto-specific.)
- **Flagged:** `0x004f2650(+0x238)`:
  - recharge > 0: sample **`0x112` (`BUZZER`)**, gap 3, and a floating text **"Recharging..."** at `(wx + w/2 − 20, wy − 10)`, colour 2 (`0x005444e0`: a kind-0x10 rising `Coin` carrying the text, as Meryl's note; no RNG). Nothing else.
  - recharge 0: **open the PetDialog** (`0x0054c6f0`): close any open dialog, store the clicked pet in `app+0x8b4`, add dialog `0x1f`.

Either way the click is consumed (returns true). Left clicks on a flagged FishTypePet take the normal pet path (Amp's zap, Walter's glove; nothing for Presto).

`traced` by `presto_changes` (`presto_01`): BUZZER (sample table) and the "Recharging..." `Coin`, which is kind 16 with colour 2 (`+0x1a0`), so `AddGameObject` (`0x00543182`) puts it on the **`+0xec`** list (with pearls), not `+0xe8`. It rises 1 px a tick and is removed after 100 ticks (`0x004f51e6`: past 50 for colours ≥ 3; above y −50 with a colour, 50 without).

## The PetDialog (`0x005355c0`)

Title **"PRESTO CHANGE-O"**, button "CANCEL", a `MoneyDialog` with id `0x1f`, **not modal**, at (0, 0) and 640 × 480: it covers the whole screen, so it takes every mouse event while it is up. `0x0054c6f0` adds it with `AddDialog` (`0x0047ac40`) directly, not through `DoDialog`, so there is **no 30-tick button delay** (`framework/dialog.md#button-delay`).

**The board keeps running under it.** `AddDialog` calls `WinFishApp` slot 81 (`0x0054cf90`), which pauses the board only if `0x0054cf40` says so: when one of the screens `app+0x80c/0x810/0x824/0x828` is up, or when the first dialog's id is **neither 0x1f nor 0x20**. So fish swim, coins fall and aliens attack while the player chooses.

**24 pet buttons** (50 × 50), button `i` for pet `i`:

| Buttons | x | y |
|---|---|---|
| 0–6 | 10 | 120, 170, …, 420 |
| 7–17 | 62, 114, …, 582 (52 apart) | 420 |
| 18–23 | 582 | 370, 320, …, 120 |

Button `i` is enabled iff the profile owns pet `i` (`profile = [app+0x8b8]`, byte `profile + i` via `0x00501410`; `screens/hatch-screen.md`). So the choice is any owned pet, **including Presto (19) and pets already in the tank** (duplicates are allowed). The CANCEL button (a `DialogButton`, child widget) is at **(270, 30)**, 100 wide, the button image's height (`PetDialog` resize `0x005313d0`).

Mouse handling (the hit test `0x005329f0` finds the first button, in index order, whose rect holds the point; −1 if there is none or it is disabled):

- **Down** (slot 55 `0x00532a60`): pressed `+0x15c` = hover `+0x158` = hit. Any button, so a right-button down counts too.
- **Up** (slot 57 `0x00532a90`): hover = hit; if hit == pressed and pressed ≠ −1, **`0x0054b450(hit)`**; pressed = −1.
- CANCEL (button id `0xbd7` in `WinFishApp` slot 3 `0x00552a8a`): `0x0054b450(−1)`.

`0x0054b450(i)`: close dialog `0x1f`; if `i ≥ 0`, a pet was stored in `app+0x8b4` and the board exists, find the Presto pet with `0x00539540` (the **first** pet in `+0xb4` with `+0x230` set, else the first in `+0xb0` with `+0x1c8` set; not necessarily the one clicked, but there is only one Presto) and call its slot 73 with `i`. Clear `app+0x8b4`.

`traced` by `presto_changes` (`presto_01`): the board kept running under the dialog (no paused ticks), and picks counted on mouse-up with no button delay (one pick ~0.6 s after the dialog opened). CANCEL and the right button inside the dialog are `static`.

## The change (slot 73, `0x004f89a0`)

Called with the new id `n` (from the dialog, or 19 from the punch revert):

1. If `n` is the pet's own id, nothing happens.
2. Recharge check `0x004f2650(+0x238)`; if still recharging (only possible from the dialog path in theory, since the dialog only opens at 0), `BUZZER` + "Recharging..." and stop.
3. Sample **`0x122` (`GROW`)**, gap 3.
4. **`Board::AddPet(n, wx, wy, flag = 1, 0)`** at the old pet's **widget** position: no position draws. Then the new pet's construction draws, **app MTRand**, as in its own spec minus the 2 position draws: for a FishTypePet `Fish::Init`'s 6 + its id-specific ctor draws (Shrapnel 1, id 21 1) + 1 shadow draw (type 2 for Amp, type 1 for id 20, else type 0); for an OtherTypePet its ctor and shadow draws (`pet-stinky.md`, `pet-clyde.md`, `pet-rufus.md`, `pet-rhubarb.md`, `pet-niko.md`). Being flagged, the new pet starts with recharge 360 (0 if `n` = 19) and puff 0. It joins the end of its list (pet counter `board+0x144 + 4n` += 1).
5. Remove the old pet: off the widget manager, the app registry and its board list (`0x00541d90`, pet counter −1), delete its shadow.
6. Between ids 19 and 23 (either way): copy the old `+0x264` and `+0x260` into the new pet (Walter's glove state; not followed).
7. `0x004d6250` on the old pet: if it was a sheltering Wadsworth, the shelter count drops (`pet-wadsworth.md#count-bookkeeping`); if it was Meryl, the song flag `0x005e89cc` is cleared (`pet-meryl.md`).

Nothing else is carried over: timers, hunger clocks, charges and modes start fresh (e.g. Amp's charge restarts at 300, Shrapnel's bomb counter at 0). The swap happens during input handling, so the new pet gets its first update in the same tick (`sim/board.md#tick-order`).

`traced` by `presto_changes` (`presto_01`) for FishTypePet → FishTypePet (Itchy, Shrapnel), FishTypePet → OtherTypePet (Stinky, kept at Presto's position: neither floor y nor shadow-first), OtherTypePet → Presto, the GROW sample, the draws and list order. The Wadsworth/Meryl hand-over and step 6 are `static`.

## Drawing

Presto (`0x004dfbb0` with `IMAGE_PRESTO`, `0x005e8d60`, `resources.xml`: 10 × 3 cels of 80 × 80): row 1 while turning (mirrored iff turn > 0), else row 0 with the guppy mirror rule; cel = the default swim cel. If the slow byte `+0x245` is set, `IMAGE_ZZZ` at (30, −10), or (50, −10) when mirrored. A type-0 shadow follows him.

After the body, for **any** flagged pet (and id 21) while the puff counter `+0x248 < 20`: `IMAGE_PRESTO` **row 2**, cel `+0x248 / 2` (0..9), 80 × 80 at (0, 0), additive and colourised with (175, 175, 175, 255): the smoke puff over a freshly changed pet (and over Presto when he is added).

For flagged pets other than Presto, a **label** (`0x004f2450`): the text "Presto" in font `0x005e8cf0`, colour `0xaaffaa`, centred under the widget (baseline at widget height − 2). While recharging, the right part is redrawn in red `0xff3333`, clipped from `x = left + tw − tw · recharge / 360` (tw = text width): the red shrinks away as the recharge runs out.

`static`.

## Open questions

- Niko made by Presto: the fixed spot is skipped for a flagged pet (`0x004d89f0`), and Niko never gets a shadow; neither is captured.
- CANCEL, and a right-button pick inside the dialog.
- The in-game Menu click (`presto_01` was cut before it): the menu isn't modelled.
- The boxing glove (pet 23) and the `+0x260`/`+0x264` hand-over belong to that pet's spec.
- The earlier specs' "drop-in flag/countdown" for `+0x230`/`+0x238` should be renamed "Presto flag / recharge" when they are next edited.
