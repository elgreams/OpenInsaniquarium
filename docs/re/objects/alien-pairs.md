# Pair-kind aliens (wave kinds 9–12)

Status: per section below. The pair dispatch table is `confirmed` from binary analysis. Spawn mechanics, list insertion, update order and RNG cost are `static` (from cached Ghidra decompilation of `AddAlien` and `Board::AddGameObject`; not dynamically verified). Re-roll formulas for 4-3/4-4/4-5 are `static` (assembly-verified, not yet under test).

Notation: same as `alien-sylvester.md`. Addresses are inner-PE VAs.

## 1. Pair dispatch table (`Board::AddAlien` @ `0x00545620`)

`confirmed` — component kinds verified from x86 PUSH immediates at the recursive call sites in `AddAlien`. Kind 9 is `traced` (`tank_1_5_won`); kinds 10–12 are `static`.

A "pair kind" is a wave kind that spawns two aliens in a single wave. `AddAlien(pair_kind, A.x, A.y, sound)` calls itself **recursively** twice:

1. `AddAlien(component_A_kind, A.x, A.y, sound)` — position A from `board+0x2cc/+0x2d0` (rolled at the warning).
2. `AddAlien(component_B_kind, B.x, B.y, sound)` — position B from `board+0x2d4/+0x2d8` (rolled at the same warning).

| Pair kind | Component A (pos A) | Component B (pos B) | First Adventure appearance | PUSH addresses |
|---|---|---|---|---|
| 9 | 1 (Sylvester) | 3 (Balrog) | 1-5 (`traced`) | `0x545673`/`0x54568b` |
| 10 | 7 (kind 7) | 3 (Balrog) | 4-3 | `0x5456a7`/`0x5456bf` |
| 11 | 5 (Destructor) | 6 (kind 6) | 4-4 | `0x5456db`/`0x5456f3` |
| 12 | 3 (Balrog) | 8 (Bilaterus) | mode 4 only | `0x54570f`/`0x545727` |

Each component call goes through the full individual creation path (§2). No other pair kinds exist; kinds 1–8 and 0x15 go through individual creation directly.

## 2. Per-component spawn sequence

`static` (from cached decompilation of `AddAlien` and `Board::AddGameObject`).

Each recursive call to `AddAlien(kind, x, y, sound)` for a single alien executes:

### Normal aliens (kinds 1–7)

1. `new Alien(x, y, kind)` @ `0x004ecfa0` (0x208 bytes) — **2 app draws**: `rand()` for direction, `rand(10)` for wander mode.
2. Warp-in effect @ `0x004ed370`: creates a `Warp` object (type `0x21`, list `+0xf0`) at `(alien.ix + 30, alien.iy − 40)`. **No RNG.**
3. `Board::AddGameObject(alien, 1)` @ `0x00542ee0` — adds alien to list `+0xb8` (type `0x16`, via switch at `0x00543171`).
4. `WidgetManager::AddWidget(alien)` — registers the widget for Update/Draw.
5. `Board::AddShadow(alien)` @ `0x00544800` — **1 app draw**: `rand(5) + 410` for the shadow y.
6. `FUN_00539ad0(0)` — clears the "no alien present" byte on all fish/breeders/pets.
7. `Board::RebuildZOrder` @ `0x0053aa40`.
8. Sound: kind-specific roar (see §4).

**Cost: 3 app draws per normal alien.**

### Bilaterus (kind 8)

1. `new Bilaterus(x, y)` @ `0x004fe000` (0x178 bytes) — **1 app draw** (direction; `alien-kind8.md` §3).
2. `FUN_004edc70` on the Bilaterus — post-construction setup (likely adds segments to the board; exact function not decompiled, `static`).
3. `WidgetManager::AddWidget(bilaterus)` — note: this is called **before** `AddGameObject`, reversed from normal aliens.
4. `Board::AddGameObject(bilaterus, 1)` — adds to list `+0xf4` (type `0x17`, via switch at `0x00542fae`).
5. **No Warp object** created (uses `FUN_004edc70` instead of `0x004ed370`).
6. **No shadow** created (no call to `0x00544800`).
7. `FUN_00539ad0(0)` + `Board::RebuildZOrder`.
8. No sound from the kind-8 path (kind 8 is not in the roar sound switch; see §4).

**Cost: 1 app draw for Bilaterus.**

## 3. Full pair spawn sequence

`static`.

When a pair kind fires (countdown reaches 0), `Board::Update` calls `AddAlien(pair_kind, A.x, A.y, 1)`. The function:

1. Calls `AddAlien(component_A, A.x, A.y, 1)`:
   - Full creation sequence (§2), including `RebuildZOrder` and roar sound.
2. Calls `AddAlien(component_B, B.x, B.y, 1)`:
   - Full creation sequence (§2), including `RebuildZOrder` and roar sound.
3. The outer pair-kind call runs `FUN_00539ad0(0)` + `RebuildZOrder` a third time (redundant but harmless).
4. The outer call does **not** play a sound (pair kind numbers 9–12 fall through the sound switch without matching any case).

### List insertion order

Component A is inserted into its board list first, then component B. For pair kinds 9–11, both aliens are type `0x16` (Alien) and go to list `+0xb8` in order: A first, B second. For pair kind 12, the Balrog (type `0x16`) goes to `+0xb8` and the Bilaterus (type `0x17`) goes to `+0xf4` — they are on separate lists.

### Update order

Update order follows the z-order rebuild sequence (`board.md`). The rebuild walks `+0xb8` before `+0xf4`, so:

- **Pair kinds 9–11** (both on `+0xb8`): component A updates first (earlier in list), then component B.
- **Pair kind 12** (Balrog on `+0xb8`, Bilaterus on `+0xf4`): the Balrog updates first.

Within the same list, insertion order is preserved: A is earlier than B.

### RNG cost per pair spawn

| Pair kind | Component A draws | Component B draws | Total app draws |
|---|---|---|---|
| 9 (1 + 3) | 3 (ctor 2 + shadow 1) | 3 (ctor 2 + shadow 1) | **6** |
| 10 (7 + 3) | 3 | 3 | **6** |
| 11 (5 + 6) | 3 | 3 | **6** |
| 12 (3 + 8) | 3 (Balrog) | 1 (Bilaterus, no shadow) | **4** |

These are in addition to the 4 warning draws at countdown 275 (`rand(450)`, `rand(195)`, `rand(450)`, `rand(195)` for positions A and B).

## 4. Sound per pair kind

`static` (from the sound switch at `0x00545866`–`0x0054593e`).

Each component alien plays its own roar during its recursive `AddAlien` call. The outer pair-kind call plays nothing (kinds 9–12 don't match the switch). Sound ids from DAT globals:

| Pair kind | Component A sound | Component B sound |
|---|---|---|
| 9 | ROAR (`DAT_005e8c6c`, kinds 1/2/3) | ROAR (same) |
| 10 | ROAR3 (`DAT_005e89fc`, kind 7) | ROAR (kind 3) |
| 11 | INTERFER (`DAT_005e8bc8`, kind 5) | ROAR2 (`DAT_005e8b6c`, kind 6) |
| 12 | ROAR (kind 3) | *none* (kind 8 not in switch) |

## 5. Re-roll formulas for tank 4

`static` (from assembly at `0x548d35`–`0x548dea` in `Board::Update`; verified against the x86 encoding, matches `shop-bar.md`).

After each wave spawns, `Board::Update` re-rolls the kind for the next wave. 4-1 and 4-2 keep their kind. Tank 4 levels 3–5:

| Level | Assembly | Formula | Possible kinds |
|---|---|---|---|
| 4-3 | `0x548d35` | `(r & 1) ? 4 : 10` | kind 4 (Gus, 50 %) or **pair 10** (50 %) |
| 4-4 | `0x548d67` | `(r & 1) ? 8 : 11` | kind 8 (Bilaterus, 50 %) or **pair 11** (50 %) |
| 4-5 | `0x548d92` | `r % 5` → see below | pair 10 (40 %), pair 11 (40 %), kind 8 (20 %) |

### 4-5 detail

`r % 5`:
- 0, 1 → kind 10 (pair: kind 7 + Balrog)
- 2, 3 → kind 11 (pair: Destructor + kind 6)
- 4 → kind 8 (Bilaterus)

**Mode 4 override** (`0x548dea`): on 4-5 in Challenge mode (`app+0x87c == 4`), if `waves_spawned >= 5` and the re-roll gave kind 8, it becomes **kind 12** (Balrog + Bilaterus). This never occurs in Adventure.

Each re-roll draws **1 app draw** from MTRand.

## 6. Warning messages

`static` (from string addresses pushed at countdown 275/276 in `Board::Update`).

At countdown 276, one-time hint messages may appear. At countdown 275, the warning message depends on the wave kind:

- **Kinds 1–3**: kind-specific warning (e.g. "AN ALIEN IS APPROACHING!" for kind 1).
- **Kind 8 alone**: "FOURTEEN ALIEN SIGNATURES DETECTED" (`0x5a8810`), followed by "JUST KIDDING. ONLY TWO!" (`0x5a87a8`).
- **Pair kinds 9–12**: "MULTIPLE ALIEN SIGNATURES DETECTED" (`0x5a88b4`).
- **Kinds 5, 6, 7**: kind-specific messages at their own string addresses.

The warning also plays sound `0x106` (AWOOGA) and triggers alien music. **No RNG** at countdown 275/276 beyond the 4 position draws.

## 7. Position draws at the warning

`traced` for kind 9 (`tank_1_5_won`); `static` for other pair kinds.

At countdown 275, `Board::Update` draws **4 app values** in order:

1. `A.x = rand(450) + 20`
2. `A.y = rand(195) + 105`
3. `B.x = rand(450) + 20`
4. `B.y = rand(195) + 105`

For single-alien kinds (1–8, 0x15), only position A is used. For pair kinds (9–12), both A and B are used: component A spawns at A, component B at B. The positions are stored at `board+0x2cc/+0x2d0` (A) and `board+0x2d4/+0x2d8` (B). The B draws always occur even for single-alien kinds; they're just unused.

With `board+0x178` set (Blip reticle), the draw code at `0x0053ffa2` overrides `A.y = 280` for all kinds, and also `B.y = 280` for kind 11. The reticle is drawn at both A and B positions for kinds 9–12.

## 8. Wave freeze with pairs

`static` (follows from `alien-sylvester.md` §1 step 2 and `alien-kind8.md` §12).

The countdown is frozen while any of `+0xb8`, `+0xf4` or `+0xdc` is non-empty. For a pair of two regular aliens (kinds 9–11), both are on `+0xb8`: the countdown resumes only when **both** are dead and all missiles are gone. For pair kind 12, the Balrog is on `+0xb8` and the Bilaterus segments are on `+0xf4`: both lists must be empty.

## Deviations

None identified for pair-kind mechanics. The heap-dependent effect-shot delay deviation from `alien-sylvester.md` applies to each individual alien's death debris. For pairs, two separate deaths produce two independent debris sequences.

## Open questions

- **`FUN_004edc70`** (Bilaterus post-ctor setup in `AddAlien`): exact purpose not decompiled. It is called with the Bilaterus as `this` after construction, before widget registration. It may add sub-objects (heads, bones) to the board lists and widget manager. The Ghidra MCP was offline during this analysis.
- **Warning message conditions**: the pair-kind message at countdown 275 appears to be gated by level/tank conditions (possibly only on specific hint-byte states). Need CE verification.
- **Bilaterus Warp equivalent**: normal aliens get a `Warp` object (visual warp-in hole). Bilaterus uses `0x004edc70` instead; the visual warp-in may be internal to the Bilaterus (its `+0x158` counter starts at 15, same as Alien's warp-in).
