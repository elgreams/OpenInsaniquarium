# Vert the skeleton (`Sexy::FishTypePet`, pet id 6)

Vert swims like a guppy and drops a gold coin ($35) every 216 ticks (6.0 s) while the tank is calm. He never eats and never gets hungry. He is the pet hatched after tank 2-2 (`screens/hatch-screen.md#pet-award-table-adventure-first-play-through`) and can first appear on 2-3.

Same class, update and fields as Itchy (`pet-itchy.md`); only the id-6 differences are listed here. Raw `this`-relative offsets.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`, id-6 branch
- Abilities (slot 81) @ `0x004e5900`, id-6 branch `0x004e5a60`; coin gate `0x004d70b0`
- Coin made through `Board::AddCoin` @ `0x00544430` → `Coin::Coin` @ `0x004eead0`
- Animation (slot 89) @ `0x004dfc50`; draw body `0x004f8610` → generic pet draw `0x004dfbb0` with `IMAGE_VERT`

Confidence: `static` throughout. Nothing here is under a replay test yet.

## Fields used by id 6

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), plus the coin counter `+0x220` (the guppy coin timer, starts at 0) and the coin interval `+0x224`. Divisor `+0x180` stays the `Fish::Init` draw (2.0, 1.8 or 1.6) and the bounds are a guppy's (floor 370).

`static`.

## Construction

`Board::AddPet(6, -1, -1)` (`0x00544a90`): the same **9 app draws** as Itchy (2 position, 6 `Fish::Init`, 1 shadow). The id-6 ctor branch sets the **coin interval `+0x224` = 216**, replacing the coin interval `Fish::Init` drew, with no RNG.

In the Virtual Tank (`app+0x87c == 5`) the interval is instead `Sexy::Rand() % 200 + 1080` (one draw from the `Sexy::Rand` stream, not the app MTRand), drawn inside the ctor before the shadow.

`static`.

## Update (`0x004f7ad0`), id 6

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the behaviour gate (slot 82, `0x004e9790`) has no id-6 branch and returns false, so **the swim modes always run**, aliens or not (as Prego).
- **Step 6:** the coin ability (below). Like every slot-81 ability, it is only called when there are no aliens (`+0xb8`) and no Bilaterus (`+0xf4`), and it returns at once on tank 5 (`board+0x3c8 == 5`) or while the board is paused. So the counter **freezes during a wave**.
- **Animation:** see below.

The re-roll draws (`r % 10`, then `r % 9 + 1` on 0, every 21 ticks) are the same as Itchy's.

`static`.

## Coin drop (slot 81, `0x004e5900`, id 6)

Each call (calm ticks only):

1. `counter (+0x220) += 1`. If `counter < 216`, stop.
2. `counter = 0`.
3. Gate `0x004d70b0`: **always true outside the Virtual Tank**. (In the Virtual Tank it is true unless the board's byte `+0x500` is set and a further check on `board+0x400` against the global `0x005df590` fails; not needed for the main game.)
4. **`AddCoin(wx + 15, wy + 10, kind 2, 0, −1.0, 0)`** (`0x00544430`), with `wx, wy` = Vert's widget position.

So the first coin comes on Vert's 216th calm update, and one every 216 calm ticks after that. There is no stage, hunger or growth gate.

**RNG:** none. **Sound:** none (a guppy's coin drop is silent too).

`static`.

## Vert's coin

`Coin::Coin` (`0x004eead0`) with these arguments is the same object a large guppy drops (`coin.md#guppy-coin-drop-fishupdatecoin-slot-81-0x004f1400`):

- kind `+0x194` = **2** (gold), worth **35** (`coin.md#values-0x004d53a0-normal-game`);
- the −1.0 argument means "keep the default fall speed" (1.5); the ctor only stores it when it isn't −1.0. A guppy's drop passes the same −1.0;
- `+0x190` = 0, the last argument (0) goes to the widget field `+0x24` as for a guppy coin;
- no RNG in the ctor.

The only difference from a guppy's coin is the spawn point: **(wx + 15, wy + 10)**, where a guppy uses (wx + 5, wy + 10). It falls, rests, expires and is collected exactly as `coin.md` describes, and Stinky chases it like any coin.

`static`.

## Animation (`0x004dfc50`), id 6

Turning is Itchy's (turn ±20, cel `9 − turn/2` or `turn/2 + 9`). When not turning, id 6 has its own branch, a **ping-pong** swim:

- The swim cycle `+0x1b8` advances by 1 (speed bucket `+0x1bc` ≤ 1) or 2, and wraps to 0 above 19.
- Cel = cycle if cycle < 10, else `19 − cycle`. So the cels run 0, 1, … 9, 9, 8, … 0 (at speed 1).

Facing follows vx as for the guppy. No RNG. `static`.

## Drawing (`0x004dfbb0`)

The generic pet draw, also used by Seymour, Blip, Nimbus, Presto, Brinkley and Nostradamus (`assets/sprite-variants.md`): `IMAGE_VERT`, 80×80 cels at `(cel·80, row·80)`:

| when | row | mirror |
|---|---|---|
| turning (`+0x1c4 ≠ 0`) | 1 | iff turn > 0 |
| otherwise | 0 | guppy rule (by vx, facing when vx is 0) |

There are no alien rows. With the "slow" byte `+0x245` set, an extra image (`0x005e8b44`) is drawn first at (30, −10), or (50, −10) when mirrored; never in Adventure. A type-0 shadow follows him like a guppy's.

`static`.

## Open questions

- The Virtual Tank gate `0x004d70b0` (`board+0x500`, `board+0x400`, `0x005df590`): what it limits (probably a coin cap in the Virtual Tank).
- Whether a new coin made in Vert's update also updates in that tick (coins sit after `+0xb4` in the z-order, so very likely yes, as for a guppy's coin).
