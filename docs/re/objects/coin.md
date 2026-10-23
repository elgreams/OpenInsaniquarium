# Coins (`Sexy::Coin`, type `0x19`)

Status: `draft`, **confirmed** for guppy coins. Replays bit-exact against `tank1-1_coin_01` (4800 ticks: silver and gold drops from one guppy growing 0 → 2, fall, rest, expiry) and `tank1-1_coin_04` (5332 ticks: 17 gold coins clicked, glided and banked, with feeding in between).

Only the coin kinds a guppy drops in the normal game are covered. Pet, star, treasure, Virtual Tank shells and "drop from above" behaviour are only noted.

Confidence: silver, gold and diamond coins on tank 1 are `traced` (`coin_json` compared each tick). Other kinds (stars, chests, pearls, beetles) are `static`.

## Fields (raw offsets)

| Offset | Type | Meaning |
|---|---|---|
| `+0x158`, `+0x160` | double | x, y (top-left of the 72×72 widget) |
| `+0x168` | byte | "risen" flag for the rising kinds (pearls etc.) |
| `+0x16c` | int | animation counter, 0..79 |
| `+0x170` | int | age in ticks |
| `+0x174` | int | cel = `(anim / 2) % 10` (basic coins, normal speed) |
| `+0x178` | int | expiry countdown; the coin is removed when it reaches 0 |
| `+0x17c` | int | flight step (collected coins, Virtual Tank flight only) |
| `+0x180`, `+0x184` | int | position when collected |
| `+0x188` | double | fall speed (1.5; only used by some boards) |
| `+0x194` | int | kind: 1 silver, 2 gold, 3 star, 4 diamond, 5 pearl… (see value table) |
| `+0x198` | byte | collected |
| `+0x19c` | int | ticks resting on the floor |
| `+0x1c0` | int | value multiplier (1) |
| `+0x30`, `+0x34` | int | widget position = `trunc(x), trunc(y)` |

Constructor `Coin::Coin` @ `0x004eead0`: zeroes the counters, sets multiplier 1 and fall speed 1.5, size 72×72. **No RNG.** `Board::AddCoin` @ `0x00544430` appends the coin to its list by kind and rebuilds the z-order.

`traced` for the fields in `coin_json`.

## Guppy coin drop (`Fish::UpdateCoin`, slot 81 @ `0x004f1400`)

It runs inside `Fish::Update` **after the mode reroll and before the damping timers**, and only when there are no aliens.

1. Skip if `stage ≤ 0` and `+0xac < 0` (small guppies never drop).
2. `coin_timer += 1`. The interval is `+0x224` (set at spawn: `Next() % 200 + 150`, passed through `0x004d71d0`, which only changes it in Virtual Tank). It is 35 if the global `0x005e89cc` is set.
3. When `coin_timer ≥ interval` (`0x004d7100`, normal game): reset the timer to 0. If `0x004d6bd0` allows it (always, outside Virtual Tank), choose the kind:
   - plain guppy (`+0x124 == −1`… see below): **kind = stage** (stage 1 → silver, 2 → gold). Nothing is dropped at stage ≤ 0.
   - with the "gold-capable" byte `+0x22a`: stage 1 → 4, 2 → 6, 4 → 7, other → 2.
   - pet-linked fish (`+0x124` = 0 or 5): kind 7, only from stage 2 for 5.
4. `AddCoin(widget_x + 5, widget_y + 10, kind, …)`.

Confirmed: a coin appears at `(fish.wx + 5, fish.wy + 10)` on the tick the timer would reach the interval, and the timer reads 0.

`traced`.

## `Coin::Update` (`0x004f5050`), normal game, basic kinds

1. Return if the board is paused. `PreUpdate`.
2. `anim += 1`, `age += 1`; `anim` wraps to 0 after 79. Cel = `(anim / 2) % 10` (other kinds use other divisors).
3. **Expiry:** if `expire ≠ 0`: when collected, cancel it (`expire = 0`). Otherwise `expire −= 1`, remove at 0, and **return**.
4. **Collected:** glide to the counter, 1/7 of the remaining distance per tick, toward (550, 30) on each axis. When `widget_y < 40` (checked on the previous position): remove the coin and **bank** its value.
5. **Falling:** `y += 1.5`. When `y ≥ 370`: `y = 370`, `floor_time += 1`. Once `floor_time ≥ limit`, start expiring (`expire = 5`; star coins (kind 0x11) are removed at once).
   - limit: 20 normally; **150 on the tutorial level** (tank 1 level 1, and not a "skip tutorial" profile); 108 in Virtual Tank-like modes; 200 when `board+0x16c` is set.
6. Set the widget position to `trunc(x), trunc(y)`.

Coins update **after** fish (z-order: `+0xa8` coins are near the front).

`traced` (fall, floor rest 150/20, expiry).

## Collecting (`Coin` mouse-down, slot 54 @ `0x004e5450` → `0x004e51f0`)

Coins sit in front of fish and food, so a click inside a coin's 72×72 widget goes to the coin, not to the food drop.

- Ignored if already collected.
- Set `flight = 1`, record the position, `collected = 1`.
- Sound by kind. Silver/gold/star (and their variants): **`0x005383d0`**. If sample `0x12c` is off cooldown (3 ticks), **one RNG draw** (`Next() & 3`) picks the variant. Diamonds and pearls play fixed samples (no RNG).
- With aliens on the board, the click also counts as a shot (`0x00539d40`).

`traced` (glide, banking, collect sounds).

## Values (`0x004d53a0`, normal game)

| Kind | Value |
|---|---|
| 1, 8, 0x12 | 15 (silver) |
| 2, 9 | 35 (gold) |
| 3, 10 | 40 (star) |
| 4, 5, 11, 12 | 200 (diamond, pearl) |
| 6, 13 | 500 (treasure chest) |
| 7, 14 | 2000 (beetle king / big treasure) |
| 0x11 | 150 |
| 0xf | 250 |

Banking (`0x0053c1e0`): `money += value`, capped at 9,999,999. Also adds to the level's earnings total `+0x460`.

`traced` for kinds 1, 2 and 4 (money compared every tick). The other rows are `static`.

## Ambient bubbles (Board::Update, related RNG)

Each running tick, `Board::Update` draws `Next() % 1000`. On 0 it emits a burst of `(Next() & 3) + 3` ambient bubbles, each with two draws (`x = 150 + r % 22`, `y = 400 + r % 6`, `0x00538a50`). That's skipped if the bubble emitter already holds ≥ 50. Confirmed: a 14-draw tick in `tank1-1_coin_01` (1 + 1 + 6×2).

`traced` for the app-stream draws. The bubbles themselves aren't modelled (global stream; see `sim/board.md`).
