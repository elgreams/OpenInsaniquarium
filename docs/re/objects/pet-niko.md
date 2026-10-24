# Niko the oyster (`Sexy::OtherTypePet`, pet id 1)

Niko sits still in the tank and makes a pearl every ~69 s (the first after 34.5 s); the player clicks the pearl for $250. Same class as Stinky (`pet-stinky.md`), so only the id-1 differences are listed here.

Confidence: `traced` for Niko's fields and RNG, and for the pearl, over the whole of tank 1-3 (`tank1_1.rs` `tank_1_3_from_start` compares `pets` and `pearls` every tick against `tank1-3_full_01`: eight pearl cycles). Drawing is per section below.

## Construction

`Board::AddPet(1, -1, -1, 0, 0)`: the same 2 position draws and 2 ctor draws as Stinky (`pet-stinky.md#construction`). **No shadow, so no shadow draw**: `0x00544800` gives `OtherTypePet` id 5 a type-0 shadow and every id other than 1 a type-1 shadow, and skips id 1 (`confirmed`: `tank1-3_full_01` starts with 3 shadows for 2 guppies and Stinky; the draw count itself is `static`, since that capture begins after the level was populated). Differences from pet 0:

- **y is not forced** to the floor: Niko stays at the drawn y. The draws are named `x = r % 265 + 105`, `y = r % 520 + 20`, so x lands in 105..369 and y in 20..539; y is clamped into range on the first update (below).
- Divisor `+0x188` = **2.0** (the default).
- `+0x1b4` (for Niko a **pearl timer**, not a shell counter) starts at 0.

`traced` (`tank1-3_full_01`, first tick: Niko at (95, 253) after the clamp, divisor 2.0).

## Update order

`RebuildZOrder` (`0x0053b604`) treats Niko specially: a pet with id 1 that isn't in the drop-in state is put **behind the board overlay** (WM slot 13, `+0x34`, with `board+0xfc`) instead of being brought to the front with the other pets. So Niko is the **back-most** game object and **updates first**, before shadows, dead fish, food and guppies. `traced`: with Niko updating among the pets, his re-roll draw came after the guppies' and the first wave diverged at tick 5082; updating him first matches.

## Update (`0x004f2e80`), pet 1

Niko never moves on his own: `+0x60` is 0 and he has no coin chase (`0x004efe80` returns false for id 1), and the target-vx branch for ids other than 0/5/7/14 isn't reached, so vx stays 0. Every tick (unpaused):

1. Common timers (`0x004f22c0`).
2. `counter += 1`, `reroll_timer += 1`. Re-roll when `reroll_timer > 20` (the edge conditions can't hold with vx = 0 except at x ≤ 10 with target ≤ 0): `reroll_timer = 0`, **`r % 10`**; if 0, **`mode = r % 3`**. Two draws at most, same as Stinky. Mode has no effect on Niko.
3. `countdown_1ac` −1 if non-zero; `counter_1a8` +1 up to 20.
4. **Pearl timer** (`0x004e2a20`, id 1, not on tank 5): `+0x1b4 += 1`, then:
   - **1224** (`0x4c8`): sound `0x129` via `PlaySound(id, 3)` (throttled).
   - **1233** (`0x4d1`): **spawn a pearl**: `AddCoin(widget_x + 1, widget_y − 2, kind 15, owner = Niko, vy 1.0)`; byte `+0x1c0 = 0` (pearl out); two bubbles `0x00538ac0` at (wx + 11, wy + 5) and (wx + 7, wy + 3) (**global stream**, not modelled).
   - **1440** (`0x5a0`): sound `0x12a` (close).
   - **1450** (`0x5aa`): **one app draw**, `+0x1b4 = r % 50`. The cycle restarts at 0..49.
5. Clamp x to [10, **540**] (id 1; Stinky's limit is 550); y to at most **350** (vy := 0) and at least 95. Then the "edge" mode overrides (x > 535 with vx > 0.1 → mode 1, x < 15 with vx < −0.1 → mode 2): never true for Niko.
6. **Animation** (`0x004eb7c0`, ids other than 0/5/7/14): `anim = (anim + 1) % 19`. The cel `+0x1a4` is untouched (stays 0). The cel the player sees comes from the draw code and the pearl timer (below).
7. Integrate (vx = vy = 0) and set the widget.

`traced` (all of the above that touches `pet_json`, the pearl's spawn and the app draw at 1450).

So the first pearl appears **1233 ticks (34.5 s)** after the level starts. At 1450 the timer resets to `r % 50`, so each later pearl appears **1233 − r** ticks after the previous reset (1450 + 1233 − r ticks after the previous pearl, about 69 s). In `tank1-3_full_01` the resets drew 4, 45, 17, 36, 31, 28, 49, 20. Sounds `0x129` at 1224 and `0x12a` at 1440 are `traced` (sample table).

The timer keeps counting while the board is paused only if the pet updates; it doesn't (paused boards skip object updates).

## The pearl (`Coin` kind 15)

`Board::AddCoin` puts kind 15 in the **third coin list `+0xec`** (with kind 16; `+0x1a0 > 1` also routes there). `RebuildZOrder` puts that list **last** (after `+0xa8`), so pearls are in front of everything and update after the other coins.

`Coin::Update` (`0x004f5050`) for kind 15 while not collected: the floor-time counter `+0x19c` `+= 1` every tick and nothing else: it doesn't fall or animate its position. The anim/age counters advance like any coin. At **`+0x19c ≥ 217`** (`0xd9`) it is removed (`vfunction76`), ~6 s after it appears: `static` (every pearl in `tank1-3_full_01` was clicked first).

Collected (clicked): glides to the money counter like other coins (`coin.md#collecting`), banking **250** (`value 0xf`). In the capture the glide takes 21 ticks and money rises by 250.

`traced` for the resting pearl, the glide and banking (`coins` list). The 217-tick expiry is `static`.

## Clicks on Niko

Mouse-down (`0x004f3a70` → `0x004f3a00`): not in the drop-in state, so it forwards the click to the board's right-click handler; a left click falls through to the board as a normal click (food drop). `static`.

## Drawing

`OtherTypePet::Draw` (slot 26 @ `0x004f37c0`) → `0x004f3590` → Niko's sprite `0x004e2bd0`. `IMAGE_NIKO` is 10×3 cels of 80×80; the cel (col, row) comes from the pearl timer `t = +0x1b4` and the anim counter `a = +0x1a0` (0..18):

| `t` | col | row | |
|---|---|---|---|
| < 1224 | `|a − 9|` | 0 | closed, breathing (ping-pong 9..0..9) |
| 1224..1233 | `(t − 1224) % 10` | 1 | opening |
| ≥ 1234, outside 1440..1449 | 9 | 2 if the pearl was taken (`+0x1c0`), else 1 | open (with / without the pearl) |
| 1440..1449 | `9 − (t − 1440) % 10` | 2 if taken, else 1 | closing |

Drawn at the widget position, never mirrored. (On backdrop 3 an extra image is drawn at (4, 67); not tank 1.)

So the pearl is part of **Niko's** sprite while it rests: `Coin::Draw` (`0x004dd7a0`) draws nothing for an uncollected kind-15 coin. Once collected, the pearl coin draws `IMAGE_PEARL` (72×72) at its widget position while it glides to the counter.

`traced` for the closed, opening and open-with-pearl cels (`frames.rs` `tank_1_3_niko_frames_match_original`: five frames from t = 1219 through the opening, pixel-exact outside the bubble masks) and again the closed cel (`tank_1_3_gameover_frames_match_original`). The closing cels and the gliding `IMAGE_PEARL` are `static`.
