# Level start (Adventure)

Status: `draft`, static. Covers how a tank level is set up and populated in the normal game. Addresses are inner-PE VAs.

## Call chain

`App::StartLevel` (`0x0054cbf0`):
1. `0x0054c540` (screen setup) constructs a fresh **`Board`** (`0x00542150`). The Board constructor makes **3 app draws**: `r % 640` each, stored as floats at `board+0x34c..0x354` (background element x positions; cosmetic). Confirmed by `tank1-1_full_01`. Then **`Board::InitLevel`** (`0x00541a20`), which resets per-level state and the shop. See `shop-bar.md#level-init-and-per-level-layout`. It sets money 200, food price 5, wave countdown 3000, egg counter 1, laser 2, food level 0 and food quantity 1, then calls `Board::SetupShop`, which overrides the alien kind and countdown per level (`objects/alien-sylvester.md#level-setup`). **No RNG** on tank 1.
2. If `0x0054b2a0` is set (the final bonus level, or `board+0x2a5`), a different path is taken. Not covered here.
3. Otherwise **`Board::PopulateLevel`** (`0x005498b0`), then `0x0054b2d0`.

## `Board::PopulateLevel` (`0x005498b0`), normal game

In order:

1. Unpause (`+0x94 = 0`); sound `SOUND_BUBBLES` (throttled, no RNG).
2. **Start bubble burst:** `n = (Next() & 3) + 2`, then `n + 1` bubbles via `0x00538a50`. Each bubble costs **2 app draws** (`150 + r % 22`, `400 + r % 6`; skipped if the emitter is full, ≥ 50). Same shape as the ambient burst in `Board::Update`.
3. **Pets.** Owned flags are `profile+0x00 + id` (24 bytes; `Profile::HasPet` @ `0x00501410` reads one). The pets *chosen* on the PetScreen are `profile+0x5a + id` (see `screens/hatch-screen.md#profile-fields-touched`).
   - Owned count `profile+0x18` ≤ 3 (and not the completed-game mode 1/4 case): for `id` in `0..18`, if **owned**, `Board::AddPet(id, -1, -1, 0, 0)`. This is the tank-1 case (1-2 adds Stinky).
   - Otherwise: for `id` in `0..23`, if **chosen** (`+0x5a + id`), add it.

   `AddPet` (`0x00544a90`) with x = −1: **2 app draws**, `a = r % 265 + 105` then `b = r % 520 + 20`. It constructs the pet and registers its shadow (Shadow ctor: **1 app draw**). Pet behaviour is per pet (`objects/pet-*.md`).
4. **Fish**, by tank:
   - **Tank 1–3** (the default branch): two guppies. Each costs 2 draws, `x = r % 520 + 20` then `y = r % 265 + 105`, followed by **`Board::AddGuppyAt(x, y)`** (`0x00546ef0`). That is the Fish constructor (6 draws, `fish-guppy.md`) and its shadow (1 draw); unlike `AddRandomGuppy`, there's no surface splash. Then both guppies get **`+0x1a4 = 2`** (growth counter preset: they start 2 pellets toward medium). On **1-1 only**, both also get **`+0x228 = 1`** (they announce hunger).
   - Tank 4: one breeder via `0x005471e0` at a random position (2 draws), with its `+0x1a4 = 2`.
   - Tank 5: wave countdown 300, and all 18 pets `0..0x11`.
5. Time Trial (mode 1) only: a timer `+0x3bc` (300 or 600).
6. Welcome message by level: 1-1 "Welcome to the Insaniquarium!", 1-2 "Welcome to the next level!", 2-1 "Welcome to the second tank!…". No RNG.

### App-RNG draws at a tank-1 level start

| Step | Draws |
|---|---|
| Board ctor | 3 (`r % 640`) |
| Start bubbles | 1 + 2 × ((r & 3) + 3) |
| Each owned pet (id order) | 2 (position) + pet ctor (see pet spec) + 1 (shadow) |
| Guppy 1 | 2 (position) + 6 (ctor) + 1 (shadow) |
| Guppy 2 | 2 + 6 + 1 |

On **1-1** (no pets): `3 + 1 + 2·k + 18` draws, where k is 3..6. Confirmed: 29 draws with k = 3 in `tank1-1_full_01`, and both guppies match exactly.

### Where the starting guppies sit

Unlike bought guppies, the starting pair is constructed in place at `(x, y)` with the constructor defaults (vy −0.5, no damping timer).

## Open questions

- `+0x1a4 = 2` on the starting guppies is the growth counter: they start 2 pellets toward medium. Confirm with a capture.
- `0x0054b2d0` (runs after populate): music and screen fade? Check for RNG.
