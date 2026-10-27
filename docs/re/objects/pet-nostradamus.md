# Nostradamus the nose (`Sexy::FishTypePet`, pet id 21)

Nostradamus swims like a guppy but keeps off the bottom. He has two powers:

- **Potions.** Every 300–599 calm ticks (fixed per nose, drawn when he is made) he drops a **kind-2 food item**, a potion, that sinks like a pellet. A hungry small or medium guppy that eats it **turns into another Nostradamus**. The player can click the potion instead: it flies to the money counter and pays **$1**. Unclaimed potions vanish at the bottom.
- **Sneeze of Power.** While any nose is in the tank, each time the alien-wave countdown reaches 30 there is a 50 % chance (global RNG) that the wave is **postponed**: the countdown goes back to 635, the tank shakes, the noses blow bubbles and sneeze, the tank music comes back and the message line says "Attack Postponed by Sneeze of Power!".

He is a bonus pet, bought with shells (`screens/bonus-screen.md`). Noses made from guppies carry a flag (`+0x240`) that makes Amp's zap kill them like guppies (`pet-amp.md#the-zap`).

Same class, update and fields as Itchy (`pet-itchy.md`). Only the id-21 differences are listed here. All offsets are raw `this`-relative (Ghidra's `FishTypePet_data.offset_0xN` = raw `+0x22c + N`, `Fish_data.field_0xN` = raw `+0x154 + N`, `Food_data.offset_0xN` = raw `+0x154 + N`).

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`: id-0x15 branch (the potion interval)
- Update (slot 22) @ `0x004f7ad0`: id-21 re-roll condition, puff counter
- Abilities (slot 81) @ `0x004e5900`, id-0x15 branch at `0x004e5ace`: the potion clock; potion maker `0x005434e0` → `Food::Food` @ `0x004ef840` with kind 2
- Potion readers: `Food::Update` (slot 22) @ `0x004f8aa0`; Food mouse-down (slot 55) @ `0x004d62f0` → pick-up `0x004d6290`; board-click pick-up `0x00539f30`; edibility `0x004d5f20`; `Fish::Eat` (slot 77) @ `0x004f1ba0` (the transformation)
- Sneeze: `Board::Update` @ `0x00547610` (call at `0x005482bd`) → `0x00543f40`
- Draw body `0x004f8610` case 0x15 (shake, then the generic pet draw `0x004dfbb0` with `IMAGE_NOSTRADAMUS`, then the puff)

Confidence: everything below is `static` (read from the code; nothing captured yet).

## Fields used by id 21

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), with:

| Offset | Type | Meaning for Nostradamus | Initial |
|---|---|---|---|
| `+0x220` | int | potion clock (the guppy coin timer): +1 per calm tick, 0 after each potion | 0 |
| `+0x224` | int | potion interval: **`r % 300 + 300`** (app MTRand); Virtual Tank: `Sexy::Rand() % 200 + 1080` (global stream) | drawn |
| `+0x240` | int | **"made from a guppy"**: set to 1 by `Fish::Eat` right after the potion's AddPet; 0 for a bought or Presto-made nose | 0 |
| `+0x248` | int | puff counter: +1 per tick while < 20, **for every id-21 pet** (not only Presto-flagged ones) | 0 |

Divisor `+0x180` stays `Fish::Init`'s draw; bounds are a guppy's (floor 370). The board's per-id pet counter for id 21 is `board+0x144 + 4·21 = board+0x198` (`board.md#object-lists`); the sneeze reads it.

`static`.

## Construction

`Board::AddPet(21, -1, -1)` (`0x00544a90`), app MTRand, in order:

1. Position: `x = r % 265 + 105`, `y = r % 520 + 20` (2 draws; none when AddPet gets a position: Presto's change, a guppy's transformation).
2. `Fish::Init` (`0x004de370`): the guppy's 6 draws.
3. Ctor id-0x15 branch: **`+0x224 = r % 300 + 300`** (1 app draw). In the Virtual Tank (`app+0x87c == 5`) instead `Sexy::Rand() % 200 + 1080` from the **global** stream (`0x0040f9e0`; not the C `rand()`).
4. Shadow (`0x00544800`): type 0 (id ≠ 16/20), 1 draw (`% 5`).

So a bought Nostradamus costs **10 app draws**, a nose made from a guppy or by Presto **8**.

`static`.

## Update (`0x004f7ad0`), id 21

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the gate has no id-21 branch: the swim modes always run, aliens or not.
- **Step 5, re-roll:** it fires when `reroll_timer > 20` **or y > floor − 50** (y > 320 with the default floor). So every tick he is below 320: reset the timer, `r % 10`, and on 0 `mode = r % 9 + 1`. That is 1–2 app draws **per tick** while he is low, not every 21 ticks. Modes 1/2 and 5–9 lift him (vy −0.5), so he soon rises above 320 again.
- **Step 6:** the potion clock (below), on calm ticks only.
- **Puff:** `+0x248 += 1` while < 20 (the counter Presto pets use; for id 21 it runs unconditionally).
- **Animation:** the default branch (Itchy's: cycle +1/+2 by speed bucket, wraps above 19, cel = cycle / 2).

`static`.

## Potion clock (slot 81, `0x004e5900`, id 21)

Slot 81 runs only on calm ticks (no alien `+0xb8`, no Bilaterus `+0xf4`), not on tank 5 (`board+0x3c8 == 5`) and not while paused. Each such tick:

1. `+0x220 += 1`. If `+0x220 < +0x224`, stop.
2. `+0x220 = 0`; **make a potion** (`0x005434e0`) at **(wx + 15, wy + 10)** (his widget position):
   - `Food::Food(x, y, motion 0, tossed 0, kind 2)`: x, y as doubles, widget `trunc`; vy −2.0, vx −3.0 (unused for motion 0); level `+0x188 = 0`; **edible delay `+0x198 = 20`** (left at the ctor's default, unlike a player drop); because kind is 2, the item is **mouse-visible** (`+0x55 = 1`, `+0x28 = 1`). **1 app draw** `(r & 1) + 3` (animation speed), as every food.
   - `AddGameObject(food, 1)` → the food list **`+0xac`**, added to the widget manager, `RebuildZOrder`.

   It does **not** go through `Board::AddFood`: no food-count limit, no `DROPFOOD` sound, no price.

The interval never changes, so a nose drops a potion every `+0x224` calm ticks (300–599, 8.4–16.8 s at 28 ms/tick). The clock freezes during waves.

RNG per potion: 1 app draw.

`static`.

## The potion (Food kind 2)

`Food::Update` (`0x004f8aa0`) for a kind-2 item:

- **Falling:** as a basic pellet (`food.md#foodupdate-0x004f8aa0-basic-pellet`): y += 1.5 per tick, edible delay counts down, no drift (motion 0).
- **Bottom:** when y > 410 a kind-2 item is **removed** at once (no fade, no rest).
- **Clicked:** a left click on it (Food slot 55 `0x004d62f0`, or a tank click that lands on it, `0x00539f30`) calls `0x004d6290` once: widget slot 38 with 0, `+0x7c = 0`, `+0x55 = 0` (no longer clickable), **carried byte `+0x185 = 1`**, and the coin-collect sound `0x005383d0` (sample `0x12c` `POINTS`, gap 3; **1 app draw `r & 3`** when not throttled, `coin.md`). Then each tick it glides `x += (550 − x)/7`, `y += (30 − y)/7` (or the mirrored subtraction when past the target); once its widget y < 40 it is removed and **$1** is banked (`0x0053c1e0(1)`: money +1, level earnings +1). A carried potion can't be eaten (`0x004d5f20`).

**Who can eat it** (`0x004d5f20`, used by the guppy's find-food and eat checks, `food.md#find-food-slot-85-0x004ef050`): kind 2 is edible only when not carried, the edible delay is 0, and (outside the Virtual Tank) the fish's **stage `+0x1a0` < 3** and its **hunger `+0x9c` < 108**. So only a quite hungry small/medium/large guppy goes for it. The bite box is the pellet's.

**A guppy eats it** (`Fish::Eat`, slot 77 `0x004f1ba0`, outside the Virtual Tank), in order:

1. The slurp (`0x005384c0`: sample `0x138`, gap 3, 1 app draw `r % 5` when not throttled) and the hunger-floor helper, as for any food.
2. The guppy's slot 87 with `true` (`0x004df0b0`): removed from its list, no corpse, no `DIE`, fish-lost count `board+0x488 += 1` (as Gash's snack, `pet-gash.md`).
3. **`Board::AddPet(21, wx, wy, flag 0, 1)`** at the guppy's widget position: no position draws; 6 `Fish::Init` + 1 interval + 1 shadow = **8 app draws**. The new pet gets **`+0x240 = 1`**.
4. If the tank now has no fish (`0x005392a0`, the test Amp uses): **`Board::AddRandomGuppy`** (11 app draws, `fish-guppy.md#spawn-boardaddrandomguppy-0x00546d70`).

No hunger, growth or coin. (The eat check then removes the potion as for any food.) Stage ≥ 3 never gets here (not edible). In the Virtual Tank a potion is not edible by pets with `+0x24 < 0`, and a fish that eats one only gets its hunger capped at 30 and a floating text (not followed).

Other readers of kind 2: aliens' food searches skip it (`0x004ee6b0`, `0x004ee850`); Nimbus never targets it but can toss one (`pet-nimbus.md#open-questions`), and his re-drop through `AddFood` passes kind 0, so a tossed potion becomes an ordinary pellet; Brinkley's target and eat have no kind filter, so he eats potions like pellets (`pet-brinkley.md#eat-0x004fe740-id-20`).

`static`.

## Sneeze of Power (`0x00543f40`)

Called from `Board::Update`'s wave block (`alien-sylvester.md#per-tick-countdown`, step 4): on the tick the countdown `board+0x2c0` **becomes 30**, if the id-21 pet count `board+0x198` is non-zero and the tank isn't 5. (The wave block is skipped while any alien, Bilaterus or missile exists and in the Virtual Tank unless `board+0x4fe`.) All draws here are on the **global** stream (`Sexy::Rand`, `0x0040f9e0`):

1. **`grand() % 100`**; if ≥ 50, nothing happens (the wave comes as usual).
2. Otherwise, walk the FishTypePet list `+0xb4` in order; for each **id-21** pet (bought, Presto-made or guppy-made): count it, and blow **5 bubbles**, each with two draws, **y first**: `by = wy + 40 + grand() % 21`, then `bx = wx + 30 + grand() % 21`, then `Board::AddBubble(bx, by)` (`0x00538ac0`, which makes its own global draws while fewer than 30 bubbles exist; `alien-sylvester.md`).
3. **Shake timer `board+0x2b4 = 30`**; sample **`0x110` (`BUBBLES`)**, gap 3.
4. **Sneezes:** for `min(count, 5)` noses, one `grand() % 150 + 100` each, the delay of a queued **`SOUND_SNEEZE`** (volume 1.0) in a list handed to the board's sound queue `board+0x90` (`0x00515e30`). The queue's time unit is not followed.
5. **Countdown `board+0x2c0 = 635`**: the wave restarts its last 635 ticks. It counts down again, so the **warning (at 275) comes again** with its 4 app position draws, and at 30 the sneeze gets another 50 % chance.
6. `WinFishApp::PlayTankMusic` (`0x0054c390`, argument 0): back to the tank music after the warning's alien music.
7. Message line: **"Attack Postponed by Sneeze of Power!"** (`Board::ShowMessage`, `0x0053e950`; `sim/board.md#message-line`).

The shake timer is decremented by `Board::Update` (`0x0054786c`, earlier in the update than the wave block) while non-zero. It is what `sim/board.md#backdrop` calls the screen shake (backdrop offset while it is 1..19, C `rand()`, draw only).

App-stream RNG: none in the sneeze itself. The repeated warning costs the usual 4 app draws.

`static`.

## Effects on other pets and the board

- **Amp** (16): the zap kills every unflagged id-21 pet with `+0x240 > 0` (a nose made from a guppy), leaving a $200 coin and a spark, with no coin cap (`pet-amp.md#the-zap`, step 5).
- **Presto** (19): a Presto-made Nostradamus is a normal nose (8 draws, `+0x240 = 0`), counted by the sneeze.
- **Guppies:** the potion turns a hungry one into a nose. A tank of noses has no guppies left to make coins.
- **Clicks:** Nostradamus himself is not mouse-visible (`+0x55` = 0) and the mouse-down `0x004fb680` has no id-21 case of its own; only his potions take clicks.

`static`.

## Drawing (`0x004f8610` case 0x15)

1. **Shake:** if `board+0x2b4` is in 1..24 and the board isn't paused, translate by `(rand() % 21 − 10, rand() % 21 − 10)` (C `rand()`, x first; a draw-time effect, not sim RNG).
2. The generic pet draw (`pet-vert.md#drawing-0x004dfbb0`) with `IMAGE_NOSTRADAMUS` (global `0x005e8ed4`; `resources.xml` `nostradamus`, 10 × 2 cels of 80 × 80, framedelay 2): row 1 while turning (mirrored iff turn > 0), row 0 otherwise (guppy mirror rule); `IMAGE_ZZZ` when the slow byte is set (Virtual Tank only). Then the translation is undone.
3. **Puff:** while `+0x248 < 20`, `IMAGE_PRESTO` row 2, cel `+0x248 / 2`, additive and colourised (175, 175, 175) (`pet-presto.md#drawing`): every new nose appears in a puff of smoke (a guppy's transformation included).

Type-0 shadow. The potion is drawn by `Food`'s own draw (not followed; `food.md`).

`static`.

## Open questions

- Everything is static. The transformation (guppy → nose with 8 draws, `+0x240 = 1`, AddRandomGuppy when the tank empties) and the sneeze are the parts to capture first.
- The board sound queue `board+0x90` (`0x00515e30`): its time unit, so when each `SNEEZE` plays.
- Whether the eat check's pellet removal after `Fish::Eat` behaves when the eater was just removed (the potion should still be removed; order of the two removals not followed).
- Breeders eat food too; whether they can eat a potion (their eat code was not followed).
- The Food draw for kind 2 (which image/cels the potion uses) was not followed.
- Virtual Tank paths (the global-stream interval, the eat branch with the floating text) were not followed.

`static`.

## Recording

A pet zoo on tank 1-2 with Nostradamus, in two parts (one capture chunk each, keeping the RNG header):

1. **Potion.** The first potion comes 300–599 calm ticks after he is added; no Cheat Engine is needed, but `+0x220` can be pushed to `+0x224 − 5` to save time (the replay must adopt that write, as it does for a fish pet's `+0x220`). Let the guppies get hungry (no feeding: hunger < 108 is needed) so one eats the potion and turns into a nose; click a later potion to collect it ($1, `POINTS` draw).
2. **Sneeze.** Needs the 1-2 alien wave: let the countdown reach 30 with a nose in the tank. The outcome is a global-stream draw (50 %), so capture the global MT state too; if the wave is postponed, the warning repeats 360 ticks later, giving a second roll. Kill the alien if it comes.

Optional: Amp (16) in a later session, zapping a guppy-made nose (`--zap-amp`).

`static` (a plan).
