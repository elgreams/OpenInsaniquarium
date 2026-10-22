# Stinky the snail (`Sexy::OtherTypePet`, pet id 0)

Status: `draft`, **static only** (nothing verified live yet).
Class: `Sexy::OtherTypePet` (type code `0x14`), vtable `0x00596d74`, object size `0x1d0`. The same class also implements pets 1, 5, 7 and 14. Only pet 0 is described in detail here; branches for other ids are noted only where they share code.

- `OtherTypePet::OtherTypePet` @ `0x004eb5c0`
- `Update` (slot 22) @ `0x004f2e80`
- Draw (slot 26) @ `0x004f37c0`
- Mouse-down (slot 54) @ `0x004f3a70`

**Offset note:** Ghidra's `OtherTypePet_data` struct is based at `this+0x154`. All offsets below are **raw `this`-relative**.

## Fields

| Offset | Type | Meaning (our name) | Initial value |
|---|---|---|---|
| `+0x30`, `+0x34` | int | widget x / y = `trunc(x)`, `trunc(y)` | from x, y |
| `+0x38`, `+0x3c` | int | widget size | 80 × 80 |
| `+0x8c` | int | type code | `0x14` |
| `+0x158` | double | **x** | ctor arg 1 |
| `+0x160` | double | **y** | ctor arg 2, **forced to 370.0 for pet 0** (unless the "drop-in" flag is set) |
| `+0x168` | double | **vx** | 0 |
| `+0x170` | double | **vy** | 0 |
| `+0x178` | double | facing (last non-zero vx sign source) | 1.0 |
| `+0x180` | double | **target vx** | 0 |
| `+0x188` | double | speed divisor | **1.2 for pet 0** (2.0 default, 1.5 for pet 5) |
| `+0x190` | int | **wander mode** (0 stop, 1 left, 2 right) | `Next() % 10` (see note) |
| `+0x194` | int | free-running counter (+1/tick) | 40 |
| `+0x198` | int | mode-reroll timer | 0 |
| `+0x19c` | int | turn-animation counter (±20 → 0) | 0 |
| `+0x1a0` | int | animation counter | 0 |
| `+0x1a4` | int | **cel** | 0 |
| `+0x1a8` | int | counter, +1/tick up to 20 | 0 |
| `+0x1ac` | int | countdown (−1/tick to 0) | 360 if drop-in flag and id ≠ 0x13, else 0; 0 in Virtual Tank |
| `+0x1b0` | byte | "slow" flag (movement divisor ×5) | 0 |
| `+0x1b4` | int | **shell counter** (0..9; >0 = hiding) | 0 |
| `+0x1b8` | int | ? | `Next() % 250 + 250` |
| `+0x1bc` | int | ? | 0 |
| `+0x1c0` | byte | ? | 0 |
| `+0x1c4` | int | **pet id** | 0 |
| `+0x1c8` | byte | "drop-in" flag (AddPet flagA) | arg |

TODO: `+0x1b8`, `+0x1bc`, `+0x1c0` for pet 0.

## Construction

### `Board::AddPet(id, x, y, flagA, flagB)` @ `0x00544a90`

1. If id == `0x13`, force flagA = 1.
2. If `x == -1`: **app RNG draw 1**: `x = Next() % 265 + 105`; **draw 2**: `y = Next() % 520 + 20`. (The ranges look swapped relative to the tank, x 105..369 and y 20..539. For Stinky the y is overwritten anyway; see below.) `static`
3. Ids 0, 1, 5, 7, 14 → `new OtherTypePet(x, y, id, board[+1000], flagA)`; others → `FishTypePet`.
4. Virtual Tank-only (`app+0x87c == 5` and !flagB): sets `+0x24 = (flagA ? 0x13 : id) + 1000`.
5. `AddGameObject` (joins list `+0xb0`, `++board[+0x144 + 4*id]`), `wm->AddWidget`, shadow creation `0x00544800`, `RebuildZOrder`.

### Constructor `0x004eb5c0`

In order:
1. `GameObject` base ctor, type `0x14`, id/flag stored.
2. `x = arg1`, `y = arg2`. Pet 0 without the drop-in flag: **`y = 370.0`** (pet 14: 355, pet 7: 365).
3. `0x004d89f0(board[+1000])`: only for pet 1 (fixed spawn positions); no-op for pet 0.
4. Widget pos = `trunc(x), trunc(y)`; vx, vy, target vx = 0; size 80×80; facing = 1.0; divisor = 1.2.
5. **App RNG draw 3**: `mode = Next() % 10`.
6. **App RNG draw 4**: `+0x1b8 = Next() % 250 + 250`.

### Shadow (`0x00544800`)

**App RNG draw 5**: shadow y = `410 + Next() % 5` (see board.md / fish-guppy.md).

So `AddPet(0, -1, -1, 0, 0)` costs **5 app-RNG draws**: x, (discarded) y, mode, `+0x1b8`, shadow.

### Update order

`+0xb0` OtherTypePet comes after aliens (pass 1) and before FishTypePet, missiles, shots and coins in `RebuildZOrder`. So in tank 1-2 Stinky updates **after all guppies and food and before the `+0xa8` coins** — he sees coin positions from the previous tick.

## `Update` (`0x004f2e80`), pet 0, normal game

TODO

## Coin chasing

TODO

## Animation

TODO

## Interactions

TODO

## Summary / open questions

TODO
