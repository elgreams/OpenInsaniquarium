# Alien: Sylvester (`Sexy::Alien`, type `0x16`, kind 1)

Status: `draft` (static only). Nothing here is CE-confirmed yet. Addresses are inner-PE VAs; offsets are raw `this`-relative.

Sylvester is the first alien. In Adventure he shows up in **tank 1-2**. Tank 1-1 has no alien waves. `Sexy::Alien` is one class for all "simple" aliens; the kind code sits at `alien+0x1ec`. Sylvester is **kind 1**. Kind 2 uses the same sprite with different stats (see the table in §2).

Notation: `rand(n)` = `app MTRand::NextNoAssert() % n` (`[app+0x7b0]`, `0x0040aeb0`). `grand()` = global `Sexy::Rand` (`0x0040f9e0`). "Tick" = one 28 ms logic tick (`framework/timing.md`).

TODO markers flag gaps.

## 1. Wave timing (`Board::Update` @ `0x00547610`, section `0x005481c4`–`0x00548e05`)

### Board fields

| Offset | Type | Meaning |
|---|---|---|
| `+0x2bc` | int | next alien kind (0 = no waves on this level) |
| `+0x2c0` | int | wave countdown in ticks |
| `+0x2c4` | int | post-spawn timer, set to 35 on spawn, decremented each tick while non-zero (purpose TODO, likely blocks clicks/food right after the warp) |
| `+0x2cc`,`+0x2d0` | int | spawn position A (x, y), rolled at the warning |
| `+0x2d4`,`+0x2d8` | int | spawn position B, used only for two-alien kinds 9–12 |
| `+0x45c` | int | waves spawned so far on this level |
| `+0x110` | ptr | boss alien (kind 0x15). Non-null pauses the countdown |

### Level setup (`0x00541200`, called by level start `0x00541a20`)

Level start sets `+0x2c0 = 3000` (`0x00541acf`). Then, in the normal game (`app+0x880 == 0`) for tank 1 (`board+0x3c8 == 1`), a jump table on `board+0x3cc − 1` (`0x00541888`) sets:

| Level | `+0x2bc` kind | `+0x2c0` countdown | `+0x340` (meaning TODO) |
|---|---|---|---|
| 1-1 | 0 (no aliens) | 3000 (unused) | 150 |
| 1-2 | **1 (Sylvester)** | **1750** | 500 |
| 1-3 | 2 | 3000 | 2000 |
| 1-4 | 3 | 3000 | 3000 |
| 1-5 | 3 | 3000 | 5000 |

No RNG on tank 1. Later tanks draw `rand` for some levels, e.g. 2-5 and 3-2: `kind = 4 | (r & 1)`.

TODO(confirm): that `0x00541200` runs after the 3000 write in `0x00541a20` (the 1750 must win).

### Per-tick countdown

This runs after the ambient-bubble draw (`rand(1000)`, see `sim/board.md`) and after `+0x2c4` is decremented.

1. Skip the whole wave block if the board is paused (`+0x94`), or if game mode `app+0x87c == 5` and `+0x4fe == 0`.
2. Skip the whole wave block while any alien exists: `+0xb8` (aliens), `+0xf4` (Bilaterus) or `+0xdc` (missiles) is non-empty. **The countdown is frozen while Sylvester is alive.**
3. If `kind != 0 && board+0x110 == null && byte +0x4f4 == 0`: `countdown −= 1`. If byte `+0x4fe` is set and countdown == 33: play sound `0x143`.
4. If countdown == 30, pet counter `board+0x198` (pet id 21) is non-zero, and tank ≠ 5: "Sneeze of Power" check `0x00543f40`. It is not reachable on 1-2 without that pet, but it does use **`grand()`**: `grand() % 100 < 50` postpones the wave (countdown = 635).
5. Then branch on the countdown value (mode-4 / 2999 branch omitted):
   - **276**: one-time tutorial/tip messages. No RNG.
   - **275 (warning)**: show the warning message, switch to alien music, play sound `0x106`. Then **4 app-RNG draws, in this order**:
     `A.x = rand(450) + 20`, `A.y = rand(195) + 105`, `B.x = rand(450) + 20`, `B.y = rand(195) + 105`.
     (Mode 5 calls `0x00539330` instead.)
   - **1..274**: at 100, a level-specific message (not tank 1). At 1, a music call `0x0054b1a0(1,1,0)`.
   - **≤ 0 (spawn)**:
     1. `+0x2c4 = 35`; bytes `+0x4ec = +0x4ed = 0`.
     2. `Board::AddAlien(kind, A.x, A.y, playSound=1)` (`0x00545620`, §2).
     3. Mode 4 (`app+0x87c == 4`) only: extra aliens with more position draws. Not Adventure.
     4. Kind re-roll for the next wave. On tank 1 only **1-5** re-rolls: `kind = (rand() & 1) ? 3 : 9`. 1-2 to 1-4 keep the same kind and draw nothing.
     5. `+0x45c += 1`; `countdown = 3000`.

**So on 1-2:** the first Sylvester arrives 1750 ticks (49 s) after level start. The warning comes at countdown 275 (tick 1475), and the spawn is on the tick the counter reaches 0. After he dies, the next one comes 3000 ticks (84 s) later, with a warning 2725 ticks after the death. Ticks spent paused or with an alien alive don't count.

TODO(confirm): whether the countdown also stops in the shop/pause states, i.e. any other gate on `+0x4f4`.

## 2. Spawn (`Board::AddAlien` @ `0x00545620`)

1. Global `0x005e8f18 = app+0x484 − 216` (bookkeeping for `0x00503580`; it doesn't matter in normal play, where `0x00503580` always returns true).
2. `new Alien(x, y, kind)` (`0x004ecfa0`, size `0x208`).
3. Warp-in effect `0x004ed370` (all kinds except `0x14`): `new Warp(alien.ix + 30, alien.iy − 40)` (type `0x21`, list `+0xf0`), added and brought to front. TODO: Warp ctor/update RNG.
4. `AddGameObject(alien)` → list `+0xb8`. WidgetManager `AddWidget`.
5. Shadow `0x00544800` → `0x00544370(2, alien)` (kind ≠ `0x14`). TODO: shadow RNG.
6. `0x00539ad0(0)`: clears byte `+0x55` on every guppy, breeder, (pets…) (TODO meaning, likely "alien-free/relaxed").
7. `Board::RebuildZOrder`.
8. Play the arrival sample (kind 1/2/3: `DAT_005e8c6c`).

### Alien constructor RNG (kind 1): **2 app draws**

1. `r = rand()`: even → `vx = −3.0` and facing `+0x1c8 = −1.0`; odd → `vx = +3.0` and facing `+1.0`.
2. (kind-specific: none for kind 1. Kind `0x14` draws `rand(5)` here.)
3. (only if `app+0x880` [virtual tank / screensaver]: `grand()` health override.)
4. `+0x1d0 = rand(10)` (initial value of a timer, TODO meaning).

### Fields (raw offsets) and initial values

| Offset | Type | Init | Meaning |
|---|---|---|---|
| `+0x30`,`+0x34` | int | trunc(x), trunc(y) | widget pos |
| `+0x38`,`+0x3c` | int | 160, 160 | widget size (80×80 for kind `0x14`) |
| `+0x158` | int | 15 | spawn/warp-in counter (sprite scale-in while 1..9 in `Alien::DrawSprite` `0x004dae20`) |
| `+0x160`,`+0x168` | double | x, y | position (top-left). **Alien x/y are at `+0x160/+0x168`, not `+0x158/+0x160` as for Fish** |
| `+0x170`,`+0x178` | double | ±3.0, 0 | vx, vy |
| `+0x180`,`+0x188` | double | 0, 0 | TODO |
| `+0x1a0` | int | 100 | TODO (40 for kind `0x14`) |
| `+0x1a4` | int | 0 | |
| `+0x1a8` | int | 75 | (144 in virtual tank) |
| `+0x1ac` | int | 0 | hit-flash / hurt timer (draw uses it) |
| `+0x1b0` | int | 0 | animation counter |
| `+0x1b4` | int | 0 | turn counter (±10) |
| `+0x1b8` | double | **2.0** | speed |
| `+0x1c0` | int | 1 | sprite cel |
| `+0x1c8` | double | ±1.0 | facing (sign of last non-zero vx) |
| `+0x1d0` | int | `rand(10)` | TODO |
| `+0x1d4`,`+0x1d8` | int | 40, 20 | TODO |
| `+0x1dc`..`+0x1e8` | int | 0, 0, 300, 75 | TODO (saved only for kind `0x15`) |
| `+0x1ec` | int | kind | alien kind |
| `+0x1f0` | double | **50.0** | health |
| `+0x1f8` | double | = health | max health |
| `+0x200` | int | 0 | TODO (30 for kind `0x14`) |
| `+0x204` | byte | 0 | |

Per-kind ctor stats (speed `+0x1b8`, health `+0x1f0`):

| Kind | Speed | Health | Sprite |
|---|---|---|---|
| 1 (Sylvester) | 2.0 | 50 | `DAT_005e8ebc` |
| 2 | 1.6 | 60 | same as 1 |
| 3 | 1.2 | 130 | `DAT_005e8cec` |
| 4 | 1.6 | 100 | `DAT_005e8ae4` |
| 5 | 1.2 | 150 | |
| 6 | 3.5 | 220 | (y forced to 280 outside mode 5) |
| 7 | 0.5 | 260 | |
| `0x14` | 0.8 + rand(5)/10 | 1.0 | 80×80 |

## 3. `Alien::Update` (kind 1)

TODO (in progress)

## 4. Being shot

TODO

## 5. Guppies while an alien is present

TODO

## 6. RNG summary

TODO
