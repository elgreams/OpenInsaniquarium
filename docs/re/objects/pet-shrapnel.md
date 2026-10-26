# Shrapnel the robot fish (`Sexy::FishTypePet`, pet id 11)

Shrapnel swims like a guppy and, every 633–652 calm ticks (17.7–18.3 s; the interval is drawn once, when he is built), drops a **bomb**: a kind-0x11 `Coin` that falls like a coin. Clicked, it is banked for **$150**. Not clicked, from 31 ticks after the drop (falling or resting) it **kills the first fish it touches** (guppies, carnivores, Ultravores, beetlemunchers, starcatchers, Guppycrunchers, breeders) and explodes; if nothing touches it, it explodes harmlessly after the usual coin rest time on the floor. He flashes white for the last 50 ticks before each drop.

Same class, update and fields as Itchy (`pet-itchy.md`); only the id-11 differences are listed. Raw `this`-relative offsets.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`, id-11 branch (after the id-0x15 branch)
- Abilities (slot 81) @ `0x004e5900`, id-11 branch at `0x004e5b2b`..`0x004e5b77`
- Bomb: `Board::AddCoin` @ `0x00544430` → `Coin::Coin` @ `0x004eead0`; `Coin::Update` @ `0x004f5050`; coin removal `0x004d5510` (the explosion); effect shot `0x005436f0`
- Coin click `0x004e51f0`, value `0x004d53a0`, Coin draw (slot 27) @ `0x004dd7a0`
- Animation (slot 89) @ `0x004dfc50`, id-11 branch `0x004e0039`; draw body `0x004f8610` case 11 → `0x004df890`

Confidence: everything here is `static` (read from code; no capture with Shrapnel yet).

## Fields used by id 11

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), plus:

| Offset | Type | Meaning |
|---|---|---|
| `+0x220` | int | bomb counter (the guppy coin timer `Fish::Init` zeroes) |
| `+0x224` | int | bomb interval: **`r % 20 + 633`** (633..652), fixed for his life |
| `+0x258` | double | flash phase, starts 0.0 (ctor), ramps −1.0..1.0 in the animation |

Divisor `+0x180` stays the `Fish::Init` draw (2.0, 1.8 or 1.6); bounds are a guppy's (floor 370).

`static`.

## Construction

`Board::AddPet(11, -1, -1)` (`0x00544a90`), **app MTRand** (`app+0x7b0`, `MTRand__NextNoAssert`) in this order:

1. Position: `r % 265 + 105`, `r % 520 + 20` (2 draws).
2. `Fish::Fish` → `Fish::Init` (`0x004de370`): the guppy's 6 draws (facing, divisor, hunger, growth, mode, coin interval).
3. **FishTypePet ctor, id-11 branch: 1 draw, `+0x224 = r % 20 + 633`**, replacing the coin interval `Fish::Init` drew. This comes after all of `Fish::Init`'s draws (the ctor body runs after `Fish::Fish` returns) and before the shadow.
4. Shadow (`0x00544800`): type-0 shadow, 1 draw (`% 5`).

So adding Shrapnel costs **10 app draws** (Itchy's 9 plus the interval). In the Virtual Tank (`app+0x87c == 5`) step 3 is instead `Sexy::Rand() % 200 + 1080` (the `Sexy::Rand` stream, not the app MTRand), so the app stream sees 9.

`static`.

## Update (`0x004f7ad0`), id 11

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the behaviour gate (slot 82, `0x004e9790`) has no id-11 branch and returns false: **the swim modes always run**, aliens or not (as Vert).
- **Step 6:** the bomb drop (below). Like every slot-81 ability it is only called when there are no aliens (`+0xb8`) and no Bilaterus (`+0xf4`), and returns at once on tank 5 (`board+0x3c8 == 5`) or while paused. The counter **freezes during a wave**.
- **Animation:** own branch (below).

The re-roll draws (`r % 10`, then `r % 9 + 1` on 0, every 21 ticks) are unchanged.

`static`.

## Bomb drop (slot 81, `0x004e5900`, id 11)

Each call (calm ticks only):

1. `counter (+0x220) += 1`. If `counter < interval (+0x224)`, stop.
2. `counter = 0`.
3. **`AddCoin(wx + 15, wy + 10, kind 0x11, 0, −1.0, 0)`** (`0x00544430`), `wx, wy` = his widget position. Unlike Vert there is no `0x004d70b0` gate.
4. Sample **`0x143` (`UNLEASH`)**, gap 3, volume 1.0 (`Board::PlaySample` `0x00538230`).

The interval is **not re-drawn**: every bomb after the first comes exactly `+0x224` calm ticks after the previous one. **RNG: none** in the drop itself.

`static`.

## The bomb (Coin kind 0x11)

### Construction and list

`Coin::Coin` (`0x004eead0`) as for any coin (`coin.md#fields-raw-offsets`): no RNG, 72×72 widget, fall speed 1.5 (the −1.0 argument keeps the default), `+0x190 = 0`, widget `+0x24 = 0`. `Board::AddGameObject` (`0x00542ee0`, at `0x00543182`) puts a coin in `+0xec` if its `+0x1a0 > 1` or kind is 15, in `+0xe8` if kind 16, else in **`+0xa8`**: the bomb joins the ordinary coin list `+0xa8`. It sits after the pets in the z-order, so a bomb made in Shrapnel's update **also updates in that tick** (`sim/board.md#tick-order`).

Stinky's and Clyde's coin search skips kinds 0x11/0x12 (`pet-stinky.md#coin-chasing`), so pets never collect it.

`static`.

### `Coin::Update` (`0x004f5050`), kind 0x11, normal game

1. Return if paused; `PreUpdate`. `anim (+0x16c) += 1` (wraps to 0 above 79), `age (+0x170) += 1`.
2. **Cel** `+0x174 = (anim / 2) % 5` (the kind-0x11 case; other coins use `% 10`).
3. Expiry countdown `+0x178`: never set for a bomb in the normal game (step 5 removes it directly). If it were, it would count down and remove as for any coin.
4. **Collected** (`+0x198` set by a click): the normal glide to (550, 30), 1/7 of the remaining distance per tick, then remove and bank once `widget_y < 40` (`coin.md#coin-update-0x004f5050-normal-game-basic-kinds`). It never explodes or hits anything while collected.
5. **Falling** (not collected): `y += 1.5`, or **`y += 0.8` while Seymour is in the tank** (`board+0x16c ≠ 0`, `pet-seymour.md`). When `y ≥ 370`: `y = 370`, `floor_time (+0x19c) += 1`; once `floor_time ≥ limit` (20; 150 on the 1-1 tutorial `0x00537b80`; 108 in the Virtual Tank `app+0x880`; 200 with Seymour), the bomb is **removed at once** (no 5-tick fade) through `0x004d5510`, which **explodes** it (below), and the update returns.
6. Widget position = `trunc(x), trunc(y)`.
7. **Contact**, every falling or resting tick from `age ≥ 31` (the first 30 ticks after the drop are safe). With `cx = x + 36`, `cy = y + 36` (doubles, the bomb's centre) and each fish's widget `fx, fy` (ints), a fish is hit when its `+0xac < 0` (normal alive state) and
   `fx + 10 < cx < fx + 70` and `fy + 10 < cy < fy + 70` (all strict),
   with 150 instead of 70 for Ultravores (160-px widget). The lists are walked in this order, each in list order, and the **first** hit ends the search:
   1. guppies `+0xa0` → slot 88 (`Fish::Die`, argument 1)
   2. Oscars/carnivores `+0xa4` → slot 88 (1)
   3. Ultravores `+0xd4` → slot 88 (1)
   4. Gekkos (beetlemunchers) `+0xcc` → slot 88 (1)
   5. Pentas (starcatchers) `+0xd0` → `0x004f3c60(1)`
   6. Grubbers (Guppycrunchers) `+0xc4` → `0x004f2890(1)`
   7. Breeders `+0xc0` → `0x004dd5d0(1)`

   Pets (`+0xb0`, `+0xb4`), aliens and the `+0xf8` fish are never hit. The victim dies through its own death routine with argument 1: for a guppy or carnivore that is the starvation death (`fish-guppy.md#dead-fish`, `carnivore.md#starvation-slot-88-fishdie-0x004d5f90-shared`: `DIE` sample `0x117` gap 3, a `DeadFish` corpse, no RNG); the Penta/Grubber/Breeder routines also play `DIE` and add a corpse (`0x004f3c60` read; the other two assumed alike). Then, still in the bomb's update:
   - remove the bomb (`0x004d5510`) → **explosion** (below);
   - **one more effect shot**: `s = r % 3 + 3`, then `EffectShot(trunc x, trunc y, s)` (`0x005436f0`) at the bomb's top-left.

`static`.

### Explosion (`0x004d5510`, coin removal)

Every coin removal goes through `0x004d5510` (WM removal, app registry, board list `0x00541d90`). For **kind 0x11 that is not collected** it then:

1. plays **`0x11d` (`EXPLODE`)**, gap 3, volume 1.0;
2. **draws `n = r % 3 + 2`** (2..4);
3. `n` times: **draw `s = r % 3 + 3`**, **draw `y = r % 30 − 10 + wy`**, **draw `x = r % 30 − 10 + wx`** (in that order; `wx, wy` the bomb's widget ints), then `EffectShot(x, y, s)`.

`EffectShot` only creates a shot while `+0xd8` holds fewer than 20, and each created shot's ctor costs 1 more app draw (`rand(200)`) plus the heap-dependent extra draw described in `alien-sylvester.md#deviations`; the coordinate draws are made either way. All draws are **app MTRand**.

So per tick: a floor explosion costs `1 + 3n` draws (+ shot ctors); a contact explosion `1 + 3n + 1` (+ shot ctors, `n + 1` shots). A banked bomb is removed with `+0x198` set and does not explode.

`static`.

### Clicking (`0x004e51f0`)

Kind 0x11 is accepted like a coin: `collected = 1`, flight starts, and the sound is the coin helper **`0x005383d0`** (`POINTS` family: 1 app draw `r & 3` if `0x12c` is off its 3-tick cooldown; `coin.md#collecting`). Value (`0x004d53a0`): **150** in the normal game (multiplier × 5 in the Virtual Tank or on `board+0x2a5` boards). With aliens on the board the click also counts as a shot (as for any coin).

`static`.

### Drawing (`0x004dd7a0`, kind 0x11)

`IMAGE_MISCITEMS` (`0x005e8bc4`, 3 cels) cel 0 at (0, 0); then, in additive draw mode, `IMAGE_SPARKS` (`0x005e8d04`, 5 cels) cel `+0x174` (0..4) at (10, −10): the lit fuse. (Kind 16, the rising note, uses `MISCITEMS` cel 1.) No expiry fade (the countdown is never set).

`static`.

## Animation (`0x004dfc50`), id 11

Turning is Itchy's (turn ±20, cel `9 − turn/2` / `turn/2 + 9`); while turning nothing else below runs. When not turning:

- Swim cycle `+0x1b8` += 1 (speed bucket `+0x1bc` ≤ 2) or 2; set to 0 once ≥ 60. **Cel = cycle / 6** (0..9).
- Flash phase `+0x258 += 0.1`; if it is then ≥ 1.0, set it to −1.0.

Facing follows vx (guppy rule). No RNG.

`static`.

## Drawing (`0x004df890`)

`IMAGE_SHRAPNEL` (`0x005e8c2c`, 10 × 2 cels of 80 × 80), src `(cel·80, row·80)`:

| when | row | mirror |
|---|---|---|
| turning (`+0x1c4 ≠ 0`) | 1 | iff turn > 0 |
| otherwise | 0 | guppy rule |

Then, **while `+0x220 > +0x224 − 50`** (the last 49 ticks before a drop), the same cel again with colourised images, additive mode and colour (255, 255, 255, `trunc(|+0x258| · 255)`): a white pulse with a ~20-tick period. No alien rows. A type-0 shadow follows him.

`static`.

## Open questions

- Check the 31-tick arming, the contact box and the list order against a capture (any level with Shrapnel and a bomb landing on a guppy).
- `0x004f2890` (Grubber) and `0x004dd5d0` (Breeder) death routines weren't read; assumed to be their die-with-corpse paths.
- Effect-shot kinds 3–5 (the explosion sizes): only their RNG cost is described here.
- `coin.md` step 5 calls kind 0x11 a "star coin"; it is this bomb.
- Whether the `+0x258` ramp hits exactly 1.0 after 20 steps (float accumulation) only changes the pulse phase by a tick.
