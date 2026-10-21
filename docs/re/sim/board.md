# Board & game-object model

Status: `draft` (static). The object/list mapping comes straight from the routing switch. Update order is inferred from the widget framework and needs CE confirmation.
Addresses are inner-PE VAs. Offsets are raw `this`-relative.

## Architecture in one paragraph

Every in-tank entity is a `Sexy::GameObject`, which **is a `Sexy::Widget`**. Objects are added as top-level children of the `WidgetManager` (`board+0x10`), not of the Board. The Board keeps **typed vectors** of object pointers for game logic, plus one master list. Each tick, `WidgetManager` walks its child list and calls `UpdateAll` → `Update` on each widget in list order. That makes **update order = widget z-order (back to front)**. The Board controls that order by rebuilding the z-order from its typed vectors in a fixed sequence (`Board::RebuildZOrder` @ `0x0053aa40`).

## Class hierarchy (from RTTI)

```
Widget
└─ GameObject                     vtable 0x00596844, 81 slots
   ├─ Fish                        vtable 0x00597d54, 91 slots (adds 10 virtuals)
   │  ├─ Oscar, Ultra, Gekko, SylvesterFish, BallFish, BiFish, FishTypePet
   ├─ Alien, Bilaterus (+BilaterusHead, BilaterusBone)
   ├─ Breeder, Grubber, Penta, Larva
   ├─ Coin, Food, Shot, Missle, Warp, Shadow, BoxingGlove
   ├─ DeadFish, DeadAlien
   └─ OtherTypePet
Board : Widget, ButtonListener    vtable 0x005a8404
```

## Object type codes (`GameObject+0x8c`) → Board list

Each constructor stores a type code at `+0x8c`. `Board::AddGameObject` (`0x00542ee0`) routes on it with a switch (jump table `0x005431ec`) and pushes the object into one typed vector:

| Code | Class | Board vector | Likely meaning (TODO confirm) |
|---|---|---|---|
| `0x00` | `Fish` | `+0xa0` | Guppy |
| `0x05` | `Oscar` | `+0xa4` | Carnivore |
| `0x06` | `Ultra` | `+0xd4` | Ultravore |
| `0x07` | `Gekko` | `+0xcc` | Beetlemuncher? |
| `0x08` | `Penta` | `+0xd0` | Starcatcher |
| `0x09` | `Grubber` | `+0xc4` | Guppycruncher |
| `0x0a` | `Breeder` | `+0xc0` | Breeder |
| `0x14` | `OtherTypePet` | `+0xb0` | Non-fish pets; also `++board[+0x144 + 4*petId]` (petId < 24) |
| `0x15` | `FishTypePet` | `+0xb4` | Fish-shaped pets; same pet counter (petId at `+0x234`) |
| `0x16` | `Alien` | `+0xbc` if `alien+0x1ec == 0x14`, else `+0xb8` | Aliens; subtype `0x14` is special (boss?) |
| `0x17` | `Bilaterus` | `+0xf4` | Bilaterus alien |
| `0x19` | `Coin` | `+0xa8` / `+0xe8` / `+0xec` by coin subtype `+0x1a0` | Coins, pearls, treasure… |
| `0x1a` | `DeadAlien` | `+0x9c` | Death animation |
| `0x1b` | `DeadFish` | `+0x98` | Death animation |
| `0x1c` | `Food` | `+0xac` | Pellets/potions |
| `0x1d` | `Larva` | `+0xc8` | Beetle (dropped by Guppycruncher) |
| `0x1e` | `Missle` | `+0xe0` or `+0xdc` (by `Missle::FUN_004d80c0`) | Alien missiles |
| `0x1f` | `Shadow` | `+0xe4` | Floor shadows |
| `0x20` | `Shot` | `+0xd8` | Player/pet shots |
| `0x21` | `Warp` | `+0xf0` | Alien warp-in effect |
| `0x22–0x24` | `SylvesterFish`, `BallFish`, `BiFish` | `+0xf8` | TODO identify |

Codes 1–4 and 0x0b–0x13 have no case (unused or non-board objects).

Every added object is also appended to a master container at `board+0x104` (`0x00540ec0`). Unless the type is 0x1e–0x21 (missile/shadow/shot/warp), adding also sets the flag `board+0x2a7 = 1`.

## Tick order

Per logic tick (28 ms, see `framework/timing.md`):

1. `WidgetManager::UpdateFrame` walks its children in list order. For each, `UpdateAll` runs `Update()` (vtable slot 22, `+0x58`) once per WM tick (it guards on `mLastWMUpdateCount`), then recurses into that widget's own children.
2. **`Board::Update`** (`0x00547610`) runs first among the game widgets (TODO: confirm Board precedes all objects in the WM list). It handles timers, alien-wave countdown, spawns, ambient RNG, mode logic, and finally calls `Board::RebuildZOrder` (`0x0053aa40`).
3. **Each GameObject** then gets its `Update()` in the rebuilt list order.

Removal during iteration is safe: `WidgetContainer` keeps the iteration cursor in the container (`+0x18/+0x1c`) plus a "removed current" flag (`+0x14`).

### `Board::RebuildZOrder` (`0x0053aa40`) order, back → front

`RebuildZOrder` first calls `wm->MarkAllDirty()`, then `wm->BringToFront(obj)` (WM vtable slot 11, `+0x2c`) for each object in this sequence. Each call moves the object to the end of the list, so the first one called ends up furthest back and updates first:

```
board+0xfc (BoardOverlay)   ← TODO: verify it's really at the back
+0xe4 Shadow
+0x98 DeadFish
+0x9c DeadAlien
+0xe8 Coin(sub)
+0xf0 Warp
+0xd0 Penta
+0xc4 Grubber
+0xac Food
+0xa0 Fish (guppy)
+0xa4 Oscar
+0xc8 Larva
+0xcc Gekko
+0xd4 Ultra
+0xc0 Breeder
+0xf8 SylvesterFish/BallFish/BiFish   (special handling when +0x1a0 == 4, and board+0x110)
+0xb8 Alien         (pass 1: only objects with +0xfc == 0 or +0x100 != 0)
+0xf4 Bilaterus     (pass 1)
+0xb0 OtherTypePet  (special case for petId +0x1c4 == 1 with flag +0x1c8)
+0xb4 FishTypePet
+0xdc Missle
+0xe0 Missle(alt)
board+0x110         (single widget; also slot 17)
+0xb8 Alien         (pass 2)
+0xf4 Bilaterus     (pass 2)
+0xbc Alien(sub 0x14)
+0xd8 Shot
+0xa8 Coin
+0xec Coin(sub)
board+0x118 …       (single widgets, HUD overlays)
```

Per-element filter in most loops: an object is brought to front unless `obj+0xfc != 0 && obj+0x100 == 0`. This looks like a "hidden / pending removal" check. TODO.

**Determinism:** within each vector, order is insertion order (`std::vector` push_back). Objects of one type always update before the next type in the sequence above, and **this is the RNG call order**. Our sim must keep the same per-type vectors (or an equivalent ordering) and walk them in this sequence.

## Board fields seen so far

| Offset | Meaning |
|---|---|
| `+0x10` | `WidgetManager*` (parent of all game objects) |
| `+0x98…+0xf8` | Typed object vectors, see table (each `std::vector<GameObject*>*`) |
| `+0xfc` | `BoardOverlay*` |
| `+0x104` | Master object container |
| `+0x138`, `+0x13c` | Resource/name strings (ctor) |
| `+0x144[24]` | Per-pet-ID counters |
| `+0x2a7` | "Objects changed" flag |
| `+0x48c` | High-water mark of guppy count |

The `Board::Update` body (timers, wave countdown `+0x234` from 3000, the alien-type selection per tank/level, ambient bubbles) is substantial. Split it out into `sim/board-update.md` when we implement M3.

## Live observations (CE read-only capture, 2026-10-21)

From `tools/ce_capture.lua`, tank 1-1, two guppies (`traces/original/tank1-1_live_02.trace.jsonl`, 200 consecutive ticks with none missed):

| Fact | Value | Confidence |
|---|---|---|
| Board pointer | `[app+0x730]` | confirmed |
| Money | `board+0x3f0` (int); food pellet costs **5** (180→175→170) | confirmed |
| App-RNG draws per idle tick | **exactly 1** (the `rand()%1000` ambient-bubbles check in `Board::Update`) | confirmed |
| Periodic extra draws | **+2 every 21 ticks** with 2 guppies (ticks 3296, 3317, 3338, …), probably 1 draw per fish on a 21-tick timer | confirmed (count), guess (cause) |
| Food drop | money −5, `food` list +1, extra RNG draws on the drop tick | confirmed |
| Global RNG (`gMTRand`) | untouched during tank play (`mti` stayed 6) | confirmed |
| Shadows | one `Shadow` object per guppy (`board+0xe4`) | confirmed |

Per-tick app-RNG draw counts are a cheap, strong fidelity check. Our sim must reproduce this exact sequence.

## Fish spawn (`Board::AddRandomGuppy` @ `0x00546d70`)

As an example of call order and RNG consumption:
1. `x = rand() % 520 + 20`, `y = rand() % 265 + 105` (app MTRand; x drawn first)
2. `new Fish(x, y)` (0x230 bytes)
3. `fish.speed? (+0x1c) = rand() % 5 + 18` (double)
4. `fish+0xb4 = rand() % 10 + 45`; `fish+0xc = <const 0x005966b0>`; `fish+0x30 = 30`
5. `AddGameObject(fish)`, `wm->AddWidget(fish)`, update the guppy high-water mark, `FUN_00544800(fish)` (shadow creation), `RebuildZOrder()`

So a new guppy costs **4 app-RNG draws, in this order**.

## The global RNG stream in normal play

`Sexy::Rand` (global `gMTRand`) has 74 call sites. On tank 1-1 with guppies, food and coins only, every global draw comes from the **cosmetic bubble systems**:

- `board+0x11c`: the ambient bubble emitter (`0x005110e0`, run from `Board::Update`). Each tick it advances its bubbles (one `Rand() & 1` wobble per bubble, `0x004ffec0`) and removes those above the surface. With probability `emit_pct %` (`Rand() % 100 < +0x28`) it spawns a bubble at a random x in its range (`0x00510fe0`). New bubbles draw 5–6 more (`0x004ffdc0`: size `& 3`, re-rolled once on 3; y `400 + %6`; x `150 + %22`; speed from a 6-entry table). A second list at `+0x4c` (stream bubbles, `0x00511010` / `0x004fff40`) draws colour and position.
- `board+0x140`: the side bubble streams (`0x00511680`), refilled to their quota with `Rand() % 480` positions (`0x005114a0` → `0x00511580`).
- `Board::Update` also draws in level-start/background selection (`0x00547776`) and the surface-splash burst (`0x005478f0`: `5 + %5` splashes, each `%41` and `%10`).

None of these feed back into gameplay. Gameplay sites on the global stream are only: guppy startle exit (`Fish::Update`, mode 123), aliens, pets, missiles, and Virtual Tank fish init. So the sim can treat the global stream as independent of the app stream. The replay harness compares only the app stream's `mti`, and the global stream is resynchronised from each capture's header. **Modelling the bubble systems exactly** (float positions, list order) is needed only before the first global-stream gameplay draw that follows bubble activity, i.e. alien levels and startles. It's tracked as an open item for M3's Sylvester work.
