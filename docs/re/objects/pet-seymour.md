# Seymour the turtle (`Sexy::FishTypePet`, pet id 10)

Seymour swims like a guppy and has no ability code of his own. His effect lives entirely in `Coin::Update`: while at least one Seymour is in the tank, **every falling coin falls at 0.8 px per tick instead of 1.5**, rests on the floor for **200 ticks** before it starts to fade (instead of 20, or 150 on the tutorial), and the coin spin animation runs at half speed.

Same class, update and fields as Itchy (`pet-itchy.md`); only the id-10 differences are listed. Raw `this`-relative offsets.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`: no id-10 branch
- Gate (slot 82) `0x004e9790`, abilities (slot 81) `0x004e5900`: no id-10 branch
- Reader of his presence: `Coin::Update` @ `0x004f5050`, pet counter **`board+0x16c`** (`board+0x144 + 4·10`) at `0x004f5120`, `0x004f514b`, `0x004f5375`, `0x004f53f7`. A byte scan of `.text` for `[reg + 0x16c]` loads off a board pointer (`app+0x730`) finds only these four.
- Animation (slot 89) `0x004dfc50`, id-10 branch `0x004e00f9`; draw body `0x004f8610` case 10 → generic pet draw `0x004dfbb0` with `IMAGE_SEYMOUR`

Confidence: everything here is `static` (no capture with Seymour yet).

## Fields used by id 10

As Itchy's (`pet-itchy.md#fields-used-by-id-2`). Divisor `+0x180` is the `Fish::Init` draw (2.0, 1.8 or 1.6); bounds are a guppy's (floor 370). `+0x220`/`+0x224` are unused.

`static`.

## Construction

`Board::AddPet(10, -1, -1)` (`0x00544a90`): the same **9 app draws** as Itchy (2 position, 6 `Fish::Init`, 1 type-0 shadow); no extra draw. Adding him to `+0xb4` does `board+0x16c += 1` (`Board::AddGameObject` `0x00542ee0`), and the board's list removal `0x00541d90` (type 0x15 case) does `−= 1`, so the effect holds exactly while he is in the tank.

`static`.

## Update (`0x004f7ad0`), id 10

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the gate has no id-10 branch and returns false: the swim modes always run, aliens or not (as Vert).
- **Step 6:** slot 81 has no id-10 branch.
- **Animation:** own branch (below).

RNG: the re-roll draws only (`r % 10`, then `r % 9 + 1` on 0, every 21 ticks).

`static`.

## Effect on coins (`Coin::Update` `0x004f5050`)

While `board+0x16c ≠ 0`, in the normal game (`coin.md#coin-update-0x004f5050-normal-game-basic-kinds`):

| Step | Without Seymour | With Seymour | Kinds |
|---|---|---|---|
| Cel `+0x174` | `(anim / 2) % 10` | **`(anim / 4) % 10`** | every kind except 0x11 (bomb: `(anim / 2) % 5` either way) |
| Fall per tick | `y += 1.5` (or the coin's own speed `+0x188` on `board+0x2a5` boards) | **`y += 0.8`**, on every board | every coin that takes the falling branch (below) |
| Floor rest before fading | 20; 150 on the 1-1 tutorial; 108 with `app+0x880` | **200** (checked first, overrides the others) | same |

Which coins fall: every kind except the rising kinds (5 pearl, 8–14, 0x12), which first rise by their own speed table (unchanged by Seymour) and take the falling branch only once their "risen" byte `+0x168` is set. Kind 15 (Niko's pearl) and kind 16 (Meryl's note) have their own branches and are unaffected. A collected coin's glide to the counter is unaffected. In the Virtual Tank (`0x004d5360` true) the basic-kind cel divisor is 2, or **4** with Seymour, modulo 20.

So a silver or gold coin dropped at y ≈ 100 reaches the floor (370) in about 338 ticks instead of 180, then waits 200 ticks instead of 20 before the 5-tick fade. This also applies to Shrapnel's bombs (`pet-shrapnel.md#the-bomb-coin-kind-0x11`), which fall at 0.8 and rest for 200 before exploding. The 0.8 constant is `0x005c3d18`; the effect is the same with one or more Seymours.

No RNG.

`static`.

## Animation (`0x004dfc50`), id 10

Turning is Itchy's (turn ±20, cel `9 − turn/2` / `turn/2 + 9`). When not turning, the id-10 branch:

- Swim cycle `+0x1b8` += 1 (speed bucket `+0x1bc` ≤ 1) or 2; set to 0 when it exceeds 79.
- **Cel = cycle / 8** (0..9): one cel every 8 ticks, a quarter of a guppy's rate.

Facing follows vx (guppy rule). No RNG.

`static`.

## Drawing (`0x004dfbb0`)

The generic pet draw (as Vert's, `pet-vert.md#drawing-0x004dfbb0`) with `IMAGE_SEYMOUR` (`0x005e8b0c`, 10 × 2 cels of 80 × 80), src `(cel·80, row·80)`:

| when | row | mirror |
|---|---|---|
| turning (`+0x1c4 ≠ 0`) | 1 | iff turn > 0 |
| otherwise | 0 | guppy rule |

No alien rows. With the "slow" byte `+0x245` set, `IMAGE_ZZZ` (`0x005e8b44`) is drawn first at (30, −10), or (50, −10) when mirrored; never in Adventure. A type-0 shadow follows him.

`static`.

## Open questions

- Whether every pet removal path (sell, Virtual Tank storage, level end) goes through `0x00541d90`; the counters are presumably zeroed per level.
- The "slower drift" in the pet description: confirmed as the fall speed only; nothing else (food, shells, the Virtual Tank's shell coins) reads `board+0x16c`.
