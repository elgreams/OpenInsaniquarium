# Nimbus the manta ray (`Sexy::FishTypePet`, pet id 15)

Nimbus swims along the bottom of the tank (he can't go above y 320). While any coin or food is in the tank he hunts the nearest catchable one at double speed, and when he touches it he **replaces it with a "rising" copy** a little above himself: the coin or pellet flies up to about y 120, then falls again as usual. In-game text: "NIMBUS tosses any coins or food he catches back up toward the top of the tank". He is the pet hatched after tank 4-1 (`screens/hatch-screen.md#pet-award-table-adventure-first-play-through`).

Same class, update and fields as Itchy (`pet-itchy.md`); only the id-15 differences are listed. Raw `this`-relative offsets.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`: id-15 branch (divisor, ceiling)
- Gate (slot 82) `0x004e9790`: id-15 branch at the top
- Chase (slot 84) `0x004e02d0`, id-15 steering at `0x004e0934`; target (slot 85) `0x004e5fc0`, id-15 branch at `0x004e6afc`; toss (slot 86) `0x004fe740`, id-15 branch at `0x004fefab`
- Update `0x004f7ad0`: id-15 ceiling case at `0x004f847f`
- Animation (slot 89) `0x004dfc50` (ids 15 and 20 share a branch); draw body `0x004f8610` case 15 → generic pet draw `0x004dfbb0` with `IMAGE_NIMBUS`
- Readers of the toss flags: `Coin::Update` `0x004f5050` (rising kinds), `Food::Update` `0x004f8aa0` (tossed byte `+0x190`)

Confidence: everything here is `static` (read from code only; no capture with Nimbus yet).

## Fields used by id 15

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), with:

| Offset | Meaning for Nimbus | Value |
|---|---|---|
| `+0x180` | divisor | ctor **0.5**; the gate sets 0.5 while it chases, **1.8** (`0x00596780`) otherwise |
| `+0x198` | ceiling (min y) | **320** (guppy 95) |
| `+0x190` | floor | 370 (guppy default) |
| `+0x1b0` | mode timer, doubles as the 5-tick steering clock | |
| `+0x245` | slow byte; only the Virtual Tank sleepy helper sets it | 0 |

`static`.

## Construction

`Board::AddPet(15, -1, -1)` (`0x00544a90`), app MTRand, in order:

1. AddPet: `x = r % 265 + 105`, `y = r % 520 + 20`.
2. `Fish::Init` (`0x004de370`): the guppy's 6 draws (facing, divisor, hunger, growth, mode, coin interval). The divisor drawn here is then overwritten.
3. Ctor id-15 branch: divisor = 0.5 (`0x005c3c20`), ceiling = 320. No draw.
4. Shadow (`0x00544800`): FishTypePet with id ≠ 16/20 gets a **type-0 shadow**, 1 draw (`% 5`).

**9 app draws**, the same as Itchy. The random y is pulled up to 320 by the first update's clamp when it is above that.

`static`.

## Update (`0x004f7ad0`), id 15

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3, gate** (`0x004e9790`, not called on tank 5, `board+0x3c8 == 5`):
  1. Virtual Tank only (`app+0x87c == 5`): the sleepy helper `0x004d6b70` on `+0x245` (`Sexy::Rand() % 60`, see `pet-clyde.md`). Never in Adventure.
  2. If (the coin list `board+0xa8` is non-empty, **or** the food list `board+0xac` is non-empty and `app+0x87c ≠ 5`) **and** `+0x245 == 0`: divisor = **0.5**, run the chase ([Chase](#chase)), and return true. The swim modes are skipped, even if the chase finds no target.
  3. Otherwise divisor = **1.8** and return false: the guppy swim modes run.

  The list checks count every coin (collected ones, bombs, kinds he can't take). Aliens don't matter to the gate.
- **Step 6:** slot 81 has no id-15 branch.
- **Step 8 (drift) and step 13 (move)** divide by the current divisor, so while chasing he moves `2·vx` per tick and sinks `2/tick` at vx 0, and when idle `vx/1.8`.
- **Step 10, ceiling clamp:** y < 320 → y = 320. Only with the drop-in flag `+0x230` set (not in Adventure): instead of the clamp, mode = 0 and `vy += 0.25` (`0x00596818`) each tick he is above 320, so a dropped-in Nimbus eases down. (The floor clamp, 370, is the normal one; the drop-in "mode 2 below the floor" case is id 4's, not his.)
- **Animation:** own branch (below).

RNG: the 21-tick re-roll (`r % 10`, then `r % 9 + 1` on 0), as Itchy, plus the 1 food-ctor draw per tossed pellet (below). The chase, the target search and the coin toss draw nothing.

`static`.

## Chase

`0x004e02d0`, id 15, every tick the gate fires:

1. **Target** (`0x004e5fc0`, id 15). best = 10 000, strict `<` (first minimum wins). With `px = x + 40`, `py = y + 40`:
   - **Food** (`board+0xac`, list order), only when no alien (`+0xb8`) and no Bilaterus (`+0xf4`) exist and `app+0x87c ≠ 5`: `dx = trunc(px − (f.wx + 20))`, and **if `dx² < 250` then `dx = 50`**; `dy = trunc(py − (f.wy + 20))`; `d = trunc(f32 sqrt(f32(dx² + dy²)))`. Candidate if kind `+0x194 == 0` (pellet) and motion `+0x18c == 0`.
   - **Coins** (`board+0xa8`, list order, the same best): skip kind `0x11` (bomb) and `0x12`; kinds 3 and 10 (stars) while a starcatcher exists (`board+0xd0` non-empty); collected coins (`+0x198`); coins rejected by `0x004e4f90` (`app+0x880` set, a starcatcher present, kind 1 or 2; never in Adventure); and, when the app byte `app+0x358` is set, coins whose raw `+0x28 > 2160` (`0x005df588`). `dx = trunc(px − (c.wx + 36))`, `dx = 50` if `dx² < 250`; `dy = trunc(py − (c.wy + 36))`; `d` as above.

   The `dx → 50` substitution makes anything within ±15 px horizontally count as at least 50 px away, so a nearer item straight above or below him loses to one further to the side.
2. **No target:** divisor = 1.8, return (no toss). The gate still reports "chasing", so he only drifts this tick.
3. **Steering**, only when `+0x1b0 ≥ 5`; then `+0x1b0 = 0`. With `cx = x + 40`, `w = target.wx`, the first matching rule (the code nests these like a binary search):

   | Condition | vx |
   |---|---|
   | `cx > w + 54` and `vx > −2.3` | −0.5 |
   | `cx < w + 18` and `vx < 2.3` | +0.5 |
   | `cx > w + 44` and `vx > −1.3` | −0.1 |
   | `cx < w + 28` and `vx < 1.3` | +0.1 |
   | `cx > w + 36` and `vx > −0.3` | set to 0 |
   | `cx < w + 36` and `vx < 0.3` | set to 0 |

   Then with `cy = y + 40`, `h = target.wy`:

   | Condition | vy |
   |---|---|
   | `cy > h + 42` and `vy > −2` | −0.6 |
   | `cy < h + 30` and `vy < 3` | +1.0 |
   | `cy > h + 36` and `vy > −2` | −0.3 |
   | `cy < h + 36` and `vy < 3` | +0.5 |

   The 36 lines his centre up with a coin's centre (72-px widget); for 40-px food the same offsets put him 16 px right of and below its centre. No speed-bucket change.
4. **Toss** (`0x004fe740`, id 15), every tick there is a target (steering tick or not).

`static`.

## Toss (`0x004fe740`, id 15)

1. **Coins** (`board+0xa8`, list order): the **first** coin with `|px − (c.wx + 36)| < 30` and `|py − (c.wy + 36)| < 30` (doubles, strict), passing the same skip rules as the target search (bomb, 0x12, stars with a starcatcher, collected, `0x004e4f90`, the `app+0x358` test). At most one coin per tick. For it:
   - new kind = kind + 7 if kind ≤ 7 (1 silver → 8, 2 gold → 9, 3 star → 10, 4 diamond → 11, 5 pearl → 12, 6 chest → 13, 7 → 14), else unchanged (8..14 are already the rising variants). Same value (`coin.md#values`).
   - remember `c.wx` and the coin's raw `+0x28`; **remove the coin** (`0x004d5510`, list removal; no RNG since bombs are excluded);
   - **`Board::AddCoin`** (`0x00544430`) at (`c.wx`, `trunc(y − 25)`), the new kind, fall speed default (1.5; the −1.0 argument means "keep"), and the remembered `+0x28` value. `Coin::Coin` draws nothing.
2. **Food** (only when no alien and no Bilaterus exist and `app+0x87c ≠ 5`; runs whether or not a coin was tossed): over `board+0xac` in list order, the **first** pellet with `|px − (f.wx + 20)| < 30`, `|py − (f.wy + 20)| < 30` and motion `+0x18c == 0` (no kind check here, unlike the target search):
   - remember `f.wx` and its food level `+0x188`; **remove it** (`0x004d9570`);
   - **`Board::AddFood`** (`0x00543280`) at (`f.wx`, `trunc(y − 30)`), motion 0, **tossed flag 1**, edible delay 0, the same level. This runs the normal drop path: the food-count limit check (`food.md#dropping-food-player-click`: count against `0x005df58c` plus the "extra" items), then `Food::Food` with its **1 app draw** `(r & 1) + 3`, and `SOUND_DROPFOOD` through `Board::PlaySample` (`0x00538230`). Then return (at most one pellet per tick).

So per tick: at most one coin and at most one pellet; 0 draws for the coin, 1 for the pellet.

`static`.

## Effects on other objects

- **Coins** (replaced in `0x004ff11c`..`0x004ff172`): the new coin is a fresh object (age, animation, floor timer all 0) of a rising kind. `Coin::Update` (`0x004f5050`) moves kinds 5, 8–14 and 0x12 up until their risen byte `+0x168` is set:

  | y | ≥ 150 | 135..150 | 130..135 | 125..130 | 120..125 | < 120 |
  |---|---|---|---|---|---|---|
  | per tick | −8.0 | −2.5 | −1.5 | −1.0 | −0.5 | set risen |

  Once risen it falls and rests like any coin (1.5/tick, 0.8 with Seymour; `coin.md`, `pet-seymour.md`). From Nimbus's height (new y ≥ 295) it climbs to just under 120 in about 25 ticks and falls back in about 165, so as long as he keeps catching a coin it never sits out its floor timer (the "bouncing a coin up and down" of his bio). The coin is appended to `+0xa8`, which updates after `+0xb4`, so it already rises in the toss tick (very likely; not checked).
- **Food** (replaced at `0x004ff276`..`0x004ff2aa`): the new pellet has the tossed byte `+0x190 = 1` (`Food::Food`'s 4th argument). `Food::Update` (`0x004f8aa0`) moves a tossed pellet up with the same table until y < 120, then sets `+0x184` and lets it sink at 1.5/tick as usual. (`food.md` lists `+0x190`/`+0x184` as the "drop from above" variant: Nimbus is its source.) Its fade and floor rules are the normal ones once it sinks.
- **Fish, aliens:** none.

`static`.

## Animation (`0x004dfc50`), id 15

Turning is Itchy's (turn ±20, cel `9 − turn/2` / `turn/2 + 9`). When not turning (branch shared with id 20):

- Swim cycle `+0x1b8` += 1 if the speed bucket `+0x1bc < 2` or the slow byte is set, else += 2; 0 when it exceeds 39.
- **Cel = cycle / 4** (0..9).

Facing follows vx (guppy rule). No RNG.

`static`.

## Drawing (`0x004dfbb0`)

The generic pet draw (`pet-vert.md#drawing-0x004dfbb0`) with `IMAGE_NIMBUS` (global `0x005e8a08`, name at `0x0059c630`; `resources.xml`: 10 × 2 cels of 80 × 80, framedelay 2), src `(cel·80, row·80)`: row 1 while turning (mirrored iff turn > 0), row 0 otherwise (guppy mirror rule). No alien rows. `IMAGE_ZZZ` when the slow byte is set (Virtual Tank only). Type-0 shadow.

`static`.

## Open questions

- Whether a removal by `0x004d9570`/`0x004d5510` takes the object out of its board list at once. If the food stays counted until later, a toss with the food limit already reached would hit the "You can only drop …" path, which also refunds the food price (`0x0053c280`) and drops nothing; that would eat the pellet.
- Whether the new pellet updates in the toss tick (food sits before `+0xb4` in the z-order, but `AddFood` rebuilds the z-order; see `board.md` on objects created during the pass).
- The coin's raw `+0x28` (copied to the new coin) and the app byte `app+0x358` that gates the 2160 test: probably a Virtual Tank/trial tag; never seen set.
- Potions (food kind 2) are never targeted but can be tossed if he passes one while chasing something else.
- Tank 5 skips the gate, so there he only swims (divisor stays at its last value, 0.5 from the ctor).
