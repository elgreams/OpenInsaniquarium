# Prego the momma fish (`Sexy::FishTypePet`, pet id 3)

Prego swims like Itchy (`pet-itchy.md`) but never chases anything. While no alien is in, a timer runs, and every 930 ticks (33 s) she gives birth to a small guppy. She is the pet hatched after tank 1-4 and first appears on 1-5.

Same class, update and fields as Itchy; only the id-3 differences are listed here. Raw `this`-relative offsets.

Confidence: `traced` overall, per section below. `tank1_1.rs` `tank_1_5_won` compares Prego (`fish_pets`), every guppy and the app RNG on every tick of `tank1-5_full_01` (15,500 ticks, 13 births, Balrog waves and a two-alien wave). The level was played with Stinky, Niko and Prego chosen on the pet screen.

## Construction

`Board::AddPet(3, -1, -1)` (`0x00544a90`): the same 9 app draws as Itchy (2 position, 6 `Fish::Init`, 1 shadow). The id-3 ctor branch (`0x004ef535`) then, with no RNG:

- **birth interval `+0x224` = 930**, replacing the coin interval `Fish::Init` drew;
- **floor `+0x190` = 360** (10 px above a guppy's).

The birth counter is `+0x220` (the guppy coin timer), starting at 0.

`traced` (`tank_1_5_won`: first tick, interval 930, counter 1 after one update, floor 360).

## Update (`0x004f7ad0`), id 3

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the behaviour gate (slot 82, `0x004e9790`) has no id-3 branch: it returns false, so **the swim modes always run**, aliens or not.
- **Step 6, birth timer** (slot 81, `0x004e5900`, id-3 branch `0x004e5934`). It runs only while there are no aliens or Bilaterus (the counter **freezes during a wave**), not on tank 5 and not while paused:
  1. `+0x220 += 1`.
  2. (Virtual Tank only: at `interval − 210` a census can set the counter back by 300.)
  3. When `counter ≥ interval`:
     - counter = 0;
     - **`AddGuppyAt(widget_x + 7, widget_y + 25)`** (`0x00546ef0`): the level-start guppy builder, 6 ctor draws plus 1 shadow draw, **7 app draws inside Prego's update**. There's no growth preset and no splash.
     - The birth sound `0x005382c0` (no RNG, throttle 3) picks a sample by profile flags `+0x84` (cheat toggles): bit 6 → `0x121` FART2 ("Ultra Prego"); else bit 1 → `0x120` FART ("Original Prego Sound"); else **`0x108` BABY**, the default.
     - **Next interval** by the guppy count including the newborn: > 10 → **2000**; > 5 → **1230**; else **930**.
- **Animation** (slot 89, `0x004dfc50`): if `counter > interval − 220` when a turn would start, **vx = 0** instead (she doesn't turn in the ~8 s before a birth, and with vx 0 she drifts down).

**The newborn updates in the same tick.** It joins the end of the widget list, and the walk reaches it after the pets: its first snapshot already shows one `Fish::Update` (hunger one down, its mode timer and re-roll draws counted). The newborn's shadow and z-order follow the guppy rules.

`traced` (`tank_1_5_won`):
- births at 930-tick intervals, with the counter frozen through each wave;
- the 1230 interval after the count passed 5;
- the 7 draws, and the newborn's first update in its birth tick;
- the turn freeze.

The recorded profile has bit 1 set, so the capture plays `0x120`; the default is BABY. The replay takes the sample from the capture. The census and the 2000 interval are `static`.

## Drawing (`0x004df230`)

`IMAGE_PREGO`, 80×80 cels at `(cel·80, row·80)`. With c = `+0x220` and I = `+0x224`:

| when | row | cel |
|---|---|---|
| turning | 2 | swim cel; mirrored iff turn > 0 |
| c ≤ I − 200 | 0 | swim cel |
| I − 200 < c < I − 190 | 3 | `(c − I + 200) / 2` |
| I − 190 ≤ c ≤ I − 5 | 1 | swim cel |
| c > I − 5 | 3 | `c − I + 10` |

Otherwise mirroring follows the guppy rule. `static` (not frame-checked).
