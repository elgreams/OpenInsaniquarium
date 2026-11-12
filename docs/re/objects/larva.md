# Larva (`Sexy::Larva`, type `0x1d`)

Class `Sexy::Larva`, vtable `0x00596ad4`, board list `+0xc8`, object size `0x178`. A Larva is **not a `Fish`**: it derives from `GameObject` directly (like `Sexy::Penta`, `board.md#class-hierarchy`). It has its own Update (slot 22 @ `0x004f2930`) and Draw (slot 26 @ `0x004d7ea0`).

The Larva is the intermediate currency in the tank-3 economy chain. The **Guppycruncher** drops Larvae on a timer (`guppycruncher.md#larva-drop-timer`). The **Beetlemuncher** eats them and drops $500 coins (`beetlemuncher.md#eat`). Players can also **click** a Larva directly for a smaller reward ($150).

**Slot numbering** is 0-based (Ghidra's `vfunctionN` = slot `N - 1`).

**Offsets** are raw `this`-relative.

RNG: `r` = app stream `MTRand::NextNoAssert` @ `0x0040aeb0`.

`static`.

## Slot overrides

Compared with `GameObject` (vtable `0x00596844`, 81 slots):

| Slot | Larva | Role |
|---|---|---|
| 0 | `0x004f0180` | destructor |
| 22 | `0x004f2930` | **Update** |
| 26 | `0x004d7ea0` | **Draw** |
| 54 | `0x004d7ed0` | **mouse-down** (click to catch) |
| 70 | `0x0051a440` (no-op) | census counter: none |
| 72 | `0x004d68d0` | get position |
| 75 | `0x004ec5f0` | **remove** (jmp `0x004d9570`) |
| 76 | `0x00519120` | no-op |
| 77 | `0x0051a440` (no-op) | eat: none (passive object) |
| 80 | `0x004d7df0` | **sync** (serialise fields) |

`static` (all slot addresses verified against the vtable at `0x00596ad4` in the binary).

## Fields

| Offset | Type | Meaning | Initial value |
|---|---|---|---|
| `+0x30`, `+0x34` | int | widget x / y | `trunc(x)`, `trunc(y)` |
| `+0x38`, `+0x3c` | int | width, height | 72, 72 |
| `+0x55` | byte | widget mouse-visible | 0 (invisible at spawn) |
| `+0x7c` | byte | mouse-interaction flag | 0 |
| `+0x88` | ptr | board pointer | set by `GameObject::GameObject` |
| `+0x8c` | int | type | **`0x1d`** (29) |
| `+0x158` | double | **x** | spawn x |
| `+0x160` | double | **y** | spawn y |
| `+0x168` | double | **rise speed** | **1.0** |
| `+0x170` | int | **animation counter** (cel = counter % 10) | 0 |
| `+0x174` | byte | **caught flag** (0 = rising, 1 = caught/flying off) | 0 |
| `+0x175` | byte | **suppress auto-reveal** | 0 |

`static` (field layout and initial values verified from the ctor bytecode at `0x004ead40` and the sync function at `0x004d7df0`).

## Construction (`Larva::Larva(x, y)` @ `0x004ead40`)

1. `GameObject::GameObject` (`0x004e9cb0`). No RNG.
2. `+0x158` (x) = `(double)arg_x`; `+0x160` (y) = `(double)arg_y`.
3. vtable = `0x00596ad4`.
4. type `+0x8c` = `0x1d` (29).
5. Widget x = `trunc(x)`, widget y = `trunc(y)`.
6. Size 72 x 72 (0x48).
7. `+0x55` = 0 (mouse-invisible); `+0x7c` = 0.
8. **1 phantom app RNG draw** (`MTRand::NextNoAssert`): the result is discarded.
9. `+0x168` = **1.0** (rise speed, loaded via `fld1`).
10. `+0x170` = 0 (animation counter).
11. `+0x174` = 0 (caught flag, byte).
12. `+0x175` = 0 (suppress auto-reveal, byte).

**1 app draw** (phantom). The ctor does not set up a shadow or splash.

`static` (ctor bytecode at `0x004ead40` decoded directly from the PE; `GameObject::GameObject` at `0x004e9cb0`, `MTRand::NextNoAssert` at `0x0040aeb0`, `fld1` for the 1.0 constant all verified).

## Spawn (`Board::AddBeetle` @ `0x005445a0`)

Called from the Guppycruncher's Larva drop timer (`guppycruncher.md#larva-drop-timer`) with `(widget_x + 4, widget_y - 10)`.

1. Allocate **`0x178`** bytes.
2. `new Larva(x, y)`: 1 phantom draw (above).
3. `AddGameObject(larva, 0)` (`0x00542ee0`): appends to `board+0xc8`.
4. Widget manager add (board `+0x10`, slot 3).
5. `RebuildZOrder`.

**No shadow. No splash.** Total: **1 app draw** (the phantom draw from the ctor).

`static` (AddBeetle bytecode at `0x005445a0` decoded; alloc 0x178, Larva ctor call, AddGameObject arg 0, WM add, RebuildZOrder all verified).

## Per-tick update (slot 22 @ `0x004f2930`)

0. Return if the board's app pointer (`[board+0x730]`) is null or paused (`+0x94 != 0`).
1. **PreUpdate** `0x004f22c0` (generic timers).
2. **Auto-reveal**: if `+0x55 == 0` (mouse-invisible) and y < **320.0** (constant at `0x00598f88`) and `+0x175 == 0`: set `+0x55 = 1` (mouse-visible), `+0x7c = 1`.
`static`.

3. Branch on **caught flag** `+0x174`:

### Rising state (`+0x174 == 0`)

4. `y -= rise_speed` (+0x168 = **1.0** px/tick).
5. If `y < 64.0` (constant at `0x005c3d50`): **remove** via `0x004d9570` (WM remove, board remove, list remove) and return.
6. If `x > 530.0` (constant at `0x00598f80`): clamp `x = 530.0`.
7. Fall through to **post-update**.

`static`.

### Caught state (`+0x174 != 0`)

Entered when the player clicks the Larva (see **Mouse-down** below). The Larva drifts toward the upper-right corner and off-screen.

4. **x toward 550.0** (constant at `0x00598f78`):
   - If `x < 550`: `x = x + (550 - x) / 7.0` (lerp, converges from the left).
   - If `x > 550`: `x = x - (550 - x) / 7.0 = x + (x - 550) / 7.0` (accelerates rightward past 550).
   - If `x == 550`: no change.
5. **y toward 30.0** (constant at `0x005966b0`) via `y = y + 7.0 / (30.0 - y)`:
   - When `y > 30`: step is negative (moves up toward 30), magnitude grows near 30.
   - When `y < 30`: step is positive (moves down), so this only applies momentarily after overshooting past 30.
   - When `y == 30`: no change.
6. If `widget_y < 40` (0x28): **remove** via `0x004d9570`, then:
   - Increment `[board->+0x730]+0x4a8` by 1 (Larva-caught counter).
   - Add money: **$150** in normal play, **$7** in Virtual Tank. The amount is returned by `0x004d7e60` (checks `board+0x87c == 5` for VT). Money is added via `0x0053c1e0`.
   - Return.
7. If `widget_y >= 40`: fall through to **post-update**.

`static`.

### Post-update (both states)

8. **Widget position**: truncate x and y to ints, call slot 41 (widget SetPosition).
9. **Animation**: `+0x170 = (+0x170 + 1) % 10` (via `0x004d7e80`).

`static` (Update bytecode at `0x004f2930` fully decoded; constants 320.0, 64.0, 530.0, 550.0, 30.0, 7.0 all read from the PE `.rdata` section at their referenced addresses; the rise-speed 1.0 from `+0x168`, removal threshold at y < 64.0, x clamp at 530.0, caught-state formulas, and the $150/$7 money path all verified).

## Mouse-down (slot 54 @ `0x004d7ed0`)

The Larva starts mouse-invisible (`+0x55 = 0`). Once it rises above y 320 (the auto-reveal check in Update), it becomes clickable.

On click (arg3 >= 0):
1. Set `+0x174 = 1` (caught flag): triggers the caught-state branch in the next Update.
2. Play sound: sample **`0x12c`** (300), gap 3 (via `0x005383d0`).

`static` (mouse-down bytecode at `0x004d7ed0` decoded; `+0x174 = 1` write and `0x005383d0` call verified).

## Remove (slot 75 @ `0x004ec5f0`)

A direct jump to `0x004d9570`:
1. Remove from widget manager (`[board->+0x730]+0x10`, slot 4).
2. Remove from board registry (board vtable slot 34).
3. Remove from the Larva list.

Called by Update (rising past the top, or caught and off-screen) and by the Beetlemuncher's bite check (when a Beetlemuncher eats the Larva).

`static` (slot 75 is `jmp 0x004d9570`; the remove function decoded).

## Drawing (slot 26 @ `0x004d7ea0`)

1. Pre-draw call `0x004d6ae0`.
2. Cel = `+0x170` (animation counter, 0..9).
3. Image from global pointer `[0x005e8e60]` (`IMAGE_BEETLE`).
4. `DrawImage(g, image, 0, 0, cel, 5)`: 5 columns per row, so cels 0..4 are row 0, cels 5..9 are row 1.

The widget is 72 x 72. The sprite sheet has 5 columns and 2 rows (10 cels total, matching the 0..9 counter).

**Never mirrored.** Direction is not shown (the Larva rises vertically).

`static` (Draw bytecode at `0x004d7ea0` decoded; image pointer, cel index, and the 5-column argument verified).

## Sync (slot 80 @ `0x004d7df0`)

Serialises, in order:
1. Base `GameObject` fields (call `0x004d6330`).
2. `+0x158` (x, double).
3. `+0x160` (y, double).
4. `+0x168` (rise speed, double).
5. `+0x170` (animation counter, int).
6. `+0x174` (caught flag).
7. `+0x175` (suppress auto-reveal).

`static` (sync bytecode decoded; field offsets and types verified).

## Interactions

- **Beetlemuncher**: chases and eats Larvae. The Beetlemuncher's FindPrey (slot 85, `beetlemuncher.md#find-prey`) iterates `board+0xc8` and only targets Larvae with `+0x174 == 0` (not caught). On a bite, the Larva is removed (slot 75). No money is awarded for a Beetlemuncher eating a Larva; the reward comes from the Beetlemuncher's coin drop.
- **Player clicks**: set `+0x174 = 1` (caught), worth $150 on removal.
- **Shrapnel**: bombs iterate `board+0xc8` but the Larva has no die function, so Shrapnel likely does not affect Larvae. `static`.
- **Aliens**: Larvae are not prey (type `0x1d` is not in the 5-10 range for `NearestPrey`).

`static` (Shrapnel interaction also `static`).

## RNG summary

| When | Stream | Draws |
|---|---|---|
| Spawn (AddBeetle) | app | 1 phantom draw (ctor), result discarded |
| Rising, caught, click, remove | - | none |

The Larva consumes exactly **1 app draw** on its creation tick and **none** thereafter. This is important for replay fidelity: adding or removing a Larva shifts the RNG by exactly 1.

`static`.

## Lifecycle summary

1. **Spawn**: Guppycruncher's timer fires → `AddBeetle(wx+4, wy-10)` → Larva at the Guppycruncher's y (near floor ~359), rising.
2. **Rise**: y decreases by 1.0/tick. Invisible for the first ~40 ticks (until y < 320).
3. **Visible**: mouse-clickable. Beetlemuncher can chase and eat it.
4. **If eaten by Beetlemuncher**: removed instantly (slot 75). The Beetlemuncher drops a $500 coin on its own timer.
5. **If clicked by player**: `+0x174 = 1`, sound plays, Larva drifts to upper-right, removed when widget_y < 40, player gets $150.
6. **If neither**: continues rising until y < 64 (~295 ticks from a floor spawn), then removed with no reward.

Total visible window: ~256 ticks (y 320 to y 64) at 1.0/tick.
