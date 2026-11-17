# Alien kind 8: Bilaterus (`Sexy::Bilaterus`, type `0x17`, board list `+0xf4`)

Status: `static` (from cached Ghidra decompilation and raw binary analysis; no dynamic verification yet). Bilaterus is not a `Sexy::Alien`; it inherits directly from `GameObject` → `Widget`. Addresses are inner-PE VAs; offsets are raw `this`-relative. See `board.md` for the type code and list, `tank4-plan.md` for context.

Notation: same as `alien-sylvester.md` — `rand(n)` = app MTRand `% n`; "tick" = one 10 ms logic tick.

## 1. Class hierarchy and sub-objects

`static` (from RTTI class-name strings and board.md RTTI tree).

Bilaterus does **not** inherit from `Sexy::Alien`. Its RTTI chain is:

```
Widget → GameObject → Bilaterus
```

Sub-object classes (also Widget → GameObject):
- **`BilaterusHead`** (type descriptor `.?AVBilaterusHead@Sexy@@`): the two "mouths" of the worm. Each is a full GameObject. Board list: none (owned by the parent, not in a typed vector). Vtable at `0x005976cc` (120 slots; dual-vtable layout suggests a secondary interface).
- **`BilaterusBone`** (type descriptor `.?AVBilaterusBone@Sexy@@`): the body segments. Each is a full GameObject. Board list: none. Vtable at `0x0059757c` (81 slots).
- **`BiFish`** (type `0x24`, board list `+0xf8`): the digested-fish display inside the worm. Separate from the Bilaterus itself.
- **`BoxingGlove`** (type descriptor `.?AVBoxingGlove@Sexy@@`): the attack projectile created by each head. Vtable at `0x00598034` (120 slots; dual-vtable). See §5a.

The board stores Bilaterus **segments** (heads + bones) in list `+0xf4`, not the owner. Each segment's `+0x158` points back to the owning Bilaterus.

## 2. Spawn path (`Board::AddAlien` @ `0x00545620`, kind 8 branch)

`static` (from binary cross-reference at `0x00545620` and existing pet-spec references).

When `AddAlien` is called with kind 8 (from the wave countdown logic, §1 of `alien-sylvester.md`):
1. It does **not** call `new Alien(x, y, 8)`. Instead it takes the kind-8 branch, which constructs a Bilaterus.
2. The Bilaterus constructor (§3) handles all sub-object creation internally.
3. The wave countdown freezes while `+0xf4` is non-empty (`alien-sylvester.md` §1 step 2).

**Warning messages** (countdown 275, `static` from string constants):
- Kind 8 alone: **"FOURTEEN ALIEN SIGNATURES DETECTED"** (`0x5a88b4`), then **"JUST KIDDING. ONLY TWO!"** (`0x5a87a8`).
- Pair kinds 9–12 (which include Bilaterus via kind 12 = Balrog + Bilaterus): **"MULTIPLE ALIEN SIGNATURES DETECTED"** (`0x5a87c0`).

The "FOURTEEN" is flavour — the actual number of segments is 8 (2 heads + 6 bones), not 14.

## 3. Constructor (`0x004fe00e` region, vtable written at `0x004f98ea` / `0x004fe03e`)

`static` (from raw hex decoding of the binary at the vtable-write sites).

The Bilaterus vtable is at `0x00598b94` (81 slots). Three call sites write this vtable pointer:
- `0x004f06ef` — destructor.
- `0x004f98ea` — simple constructor (default init).
- `0x004fe03e` — parameterised constructor (used by AddAlien).

The parameterised constructor:
1. Calls the base `GameObject` constructor.
2. Creates **2 `BilaterusHead`** objects.
3. Creates **6 `BilaterusBone`** objects.
4. Sets the warp-in counter to **15** (same as Alien's warp-in).
5. Consumes **1 app RNG draw** (direction or layout; exact use TBD).

Total segments: 8 (2 heads + 6 bones). All are added to board list `+0xf4`. Each segment's `+0x158` → owning Bilaterus.

Segment widgets are **80×80** (from `pet-rufus.md`, `pet-itchy.md`), with centre offset 40.

## 4. Health

`static` (cross-referenced from multiple traced pet specs and cached Ghidra decompilation of missile hit function at `0x004fc070`).

Health is stored at the **first head's `+0x1d8`** (a `double`), not on the owner Bilaterus directly. Both heads share a **single health pool** — all damage paths write to `owner+0x1d8` (via `segment+0x158`).

| Fact | Value | Source | Confidence |
|---|---|---|---|
| Max health | **100.0** | `pet-blip.md`: health bar draws `health / 100`; missile hit: death at ≤ 0 from 100 | `static` |
| Hit flash timer | **10 ticks** | `owner+0x1d0 = 10` on missile/pet hit | `static` |
| Dead flag | `owner+0x164` | Set to 1 when health ≤ 0; prevents double-death | `static` |

## 5. Damage formulas (divisor equivalent)

`static` (from cached Ghidra decompilation of missile collision `0x004fc070` and pet attack `0x004fe740`).

Bilaterus is **not** an `Alien` and does **not** use the Alien laser-damage formula (`health -= 3*laser` for kinds 1–3, `health -= 2*laser+2` for kinds 5–6). Instead:

### 5.1 Player laser/click — no direct damage

Clicking or holding the laser on a Bilaterus segment does **not** reduce health. Instead:
- **Laser hold** (`param_4 < 0` in `0x004fb680`): draws a spark effect at the click position (`FUN_0053bfb0`), returns. No damage.
- **Click** (`param_4 >= 0`, type `0x17`): triggers a **BoxingGlove** attack (§5a). The head punches a nearby fish; the Bilaterus itself takes no damage from the click.

This means the player cannot directly damage Bilaterus with the mouse — only pets and the weapon (Stanley missiles) deal damage.

### 5.2 Stanley missiles (from `0x004fc070`)

| Missile path | Damage | Notes |
|---|---|---|
| **Direct hit (type 1)** on a `+0xf4` segment | **30.0 HP** (`_DAT_005966b0`) | Checks segment bounding box; damages `owner+0x1d8` |
| **Bounceback (type 2)**, weapon level < 4 | **15.0 HP** (literal `0xf` → `double`) | Missile reverses velocity (bounces off); animation frame count 15 |
| **Bounceback (type 2)**, weapon level ≥ 4 | **55.0 HP** (literal `0x37` → `double`) | Stronger bounce; animation frame count 55 |

Bounceback missile bounce speed: **1.5** (level < 4, `_DAT_005c3d58`) or **0.6** (level ≥ 4, `_DAT_005965c8`).

### 5.3 Pet damage (confirmed by traced pet specs and decompiled `0x004fe740`)

| Pet | Damage | Proximity check | Source |
|---|---|---|---|
| **Itchy** (2) | **1.0 HP/tick** (`_DAT_005965a8`) | Centre offset 40 (`+0x28` = 0x28) | `pet-itchy.md` (`traced`) + `0x004fe740` |
| **Rufus** (7) | **2.0 HP/tick** per overlapping segment | Centre offset 40 | `pet-rufus.md` (`traced`) |
| **Gash** (17) | **3.0 HP/hit** (`_DAT_005965a0`) | Centre offset 40; chase within 30.0 radius | `pet-gash.md` (`traced`) + `0x004fe740` |

### 5.4 Effective TTK (time to kill, 100 HP)

| Source | Hits/ticks to kill |
|---|---|
| Stanley direct (30 HP) | ~4 hits |
| Bounceback normal (15 HP) | ~7 hits |
| Bounceback strong (55 HP) | ~2 hits |
| Itchy alone (1 HP/tick) | 100 ticks (~1s) |
| Rufus alone (2 HP/tick/seg) | 50 ticks per segment overlap |
| Gash alone (3 HP/hit) | ~34 hits |

## 5a. Attack mechanism: BoxingGlove creation

`static` (from cached Ghidra decompilation at `0x004fb680`).

Bilaterus attacks fish by **punching** them with a `Sexy::BoxingGlove` projectile. This is entirely different from how regular Aliens eat fish. The same `BoxingGlove` class is shared with Walter (`pet-walter.md`), which documents the full hit/push/draw behaviour from traced data; this section covers the Bilaterus-specific creation path.

**BoxingGlove creation** (from the Bilaterus head's click handler at `0x004fb680`, type `0x17` branch):

Preconditions (all must be true):
- `segment+0x234` == `0x17` (Bilaterus head type)
- `segment+0x1C8` == 0 (idle — no active state)
- `segment+0x24C` == 0 (no active boxing glove)
- `segment+0x264` == 0 (no cooldown timer)
- `segment+0x1C4` == 0 (not in another state)

Direction selection (from velocity):
- If `segment+0x168` (x-velocity double) < 0: glove direction = **left** (param `0`)
- Else: direction = **right** (param `1`)
- More complex: also checks `segment+0x188` (y-velocity) via FPU comparison

After creating the glove:
- `segment+0x24C` = pointer to the new BoxingGlove
- `segment+0x1C0` = 0 (reset state)
- `segment+0x1C8` = `0x28` (40 ticks cooldown before the head can act again)
- Sound `0x136` played at volume 1.0
- Z-order rebuild triggered

**Punch counter** (non-Virtual-Tank only):
- `segment+0x260` increments on each successful punch
- After **5 punches** (`> 4`): head enters cooldown (`segment+0x264` = `DAT_005df034`), counter resets, some cleanup occurs (virtual function call at `vtable+0x98`).

## 5b. BoxingGlove constructor and lifetime

`static` (from raw binary analysis of ctor @ `0x004ef7c0`).

The constructor takes two arguments: `owner` (BilaterusHead pointer) and `direction` (byte: 0 = left, 1 = right). It calls `GameObject::GameObject()` and then:

| Field | Value | Meaning |
|---|---|---|
| `+0x38, +0x3c` | **160, 80** (0xa0, 0x50) | Widget width × height |
| `+0x30, +0x34` | −200, −200 | Initial off-screen position |
| `+0x55` | 0 | Mouse invisible |
| `+0x158` | owner | Owning BilaterusHead |
| `+0x15c` | **30** | Countdown / duration (ticks) |
| `+0x160` | direction | 0 = left, 1 = right |
| `+0x164` | 0 | Sound gap timer |

Object size: `0x168` bytes. The glove is **not** added to any board list, the master set, or the widget manager — the BilaterusHead owns, updates and draws it (same as Walter, `pet-walter.md`).

**Total lifetime: 30 ticks.** The countdown at `+0x15c` decrements by 1 each tick in Update. When it reaches 0, the glove stops acting. The head's dtor or cleanup removes it.

## 5c. BoxingGlove Update (`0x004e9a60`) — hit phases and targeting

`static` (from raw binary analysis).

Each tick the head calls the glove's Update (vtable slot 22). The function:

1. **Null/pause check**: `app = [this+0x88]`, `board = [app+0x730]`; return if null or `board+0x94` (paused) is set.
2. **Owner gate**: if `owner+0x1c8 > 35` (0x23): return without decrementing. The head's punch anim timer at `+0x1c8` starts at 40 and counts down, so the first 5 ticks after creation the timer is 40..36 (> 35), and the glove waits.
3. **Positioning**: each tick, the glove tracks the head:
   - Direction 0 (left): `this.x = head.x − 60`
   - Direction 1 (right): `this.x = head.x − 20`
   - `this.y = head.y`
4. **Countdown**: `this+0x15c` decrements by 1. If ≤ 0: return (expired).
5. **Sound gap**: `this+0x164` decrements by 1 if > 0.
6. **Hit window** (by countdown value after decrement):

| Countdown | Ticks | Reach (px) | Phase |
|---|---|---|---|
| 29–26 | 4 | 0 | Wind-up — no hits |
| 25–21 | 5 | 30 | Narrow reach |
| 20–13 | 8 | 60 | Full reach |
| 12–1 | 12 | — | Retract — no hit logic |
| 0 | 1 | — | Expire |

7. **Fist position** (the collision point, not the widget):
   - Left: `fist_x = widget.x − reach + 80`; Right: `fist_x = widget.x + reach + 80`
   - `fist_y = widget.y + 40`
8. **Target iteration**: walks the master object list at `board+0x104` (a linked list of all board objects). For each object whose `+0xf4` byte is set and that is not the BoxingGlove's own owner (`this+0x158`), dispatches on type code `+0x8c` via a lookup table at `0x004e9c68`:

   | Type code(s) | Class(es) | Action |
   |---|---|---|
   | `0x00`, `0x05`–`0x07`, `0x22`–`0x24` | Guppy, Oscar, Ultra, Gekko, SylvesterFish, BallFish, BiFish | Hit test with `k = 5` |
   | `0x15` | FishTypePet | Sub-dispatch by pet id (see below) |
   | everything else | Alien, Bilaterus, OtherTypePet, coins, food, etc. | Skipped |

   **FishTypePet sub-dispatch** (pet id at `+0x234`):

   | Pet id | Pet | Strength `k` |
   |---|---|---|
   | 3 | **Prego** | **Skipped** (never hit) |
   | 6, 11, 16 | Vert, Shrapnel, Amp | 5 |
   | 21 | Nostradamus | 10 |
   | all others | Itchy, Presto, Stanley, etc. | −1 (no clock change) |

9. **Hit test** `0x004e0ed0(s, k)` per candidate — see §5d.
10. **Sound**: if anything was hit this tick and `this+0x164 == 0`: play **PUNCH** (`0x00538340(3)`) and set `this+0x164 = 5` (cooldown before the next hit sound).

The glove always passes `s = −1` (the push-direction sign) for every target, meaning Bilaterus punches always push fish **leftward**. Walter's glove passes `s = +1` or `−1` depending on the punch direction (`pet-walter.md`); the constant `−1` in BoxingGlove::Update appears to be a Bilaterus-specific quirk (the `xor ecx, ecx; lea ecx, [ecx+ecx−1]` sequence computes −1 regardless of the direction flag).

## 5d. BoxingGlove hit test (`0x004e0ed0`)

`static` (from raw binary analysis). Cross-referenced with `pet-walter.md` (the same function, `traced` there for Walter).

The hit test receives: `ecx` = fist_x, `eax` = fist_y, stack `s` = push sign (−1 from Bilaterus), stack `k` = coin-clock strength.

1. **Already pushed**: if `fish+0x20c > 0` (push timer active), skip. Each fish can only be hit once per glove lifetime (the 50-tick push timer outlasts the 30-tick glove).
2. **Presto shield**: if the target is a FishTypePet with the Presto flag `+0x230` set and `+0x28 < 30` (fewer than 30 widget updates since creation), skip. This protects recently-transformed Presto pets.
3. **Collision**: the fish's centre `(wx + w/2, wy + h/2)` must fall strictly inside the **±40 box** around the fist position: `fist_x − 40 < cx < fist_x + 40` and `fist_y − 40 < cy < fist_y + 40`.
4. **On hit**, in order:
   - If `k > 0`: **coin clock `+0x220 = +0x224 − k`**: the target's next coin/bomb/potion comes `k` ticks later (5 for most fish, 10 for Nostradamus). If `k ≤ 0`: no clock change.
   - **Push timer `+0x20c = 50`**, misc timer `+0x210 = 180`.
   - If `+0x11c ≠ −1`: enqueue a draw via `board+0x90` (1 global draw, the emote removal; `pet-walter.md` §hit).
   - `0x004d6cc0`: if `+0xfc` is set, `+0x100 = 1` (or capped at 10) — the z-order visibility pair.
   - **`grand()`** (global MTRand, 1 draw): even → **push `+0x218 = s · 15.0`**, odd → **`s · 10.0`**.

For Bilaterus (`s = −1`): push velocity is **−15.0** or **−10.0** (always leftward, 50/50 via global RNG). For Walter (`s = ±1`): push direction matches the punch.

**What a hit does to the target** (from `pet-walter.md`, `traced`): while `+0x20c > 0`, the target's own update applies `push *= 0.9; x += push` each tick (damped slide of ~134 px for 15, ~90 px for 10). **No damage, no death.** The fish is pushed but continues swimming normally. A Presto-flagged pet that has been pushed for 10 ticks (`+0x20c` drops below 41) **reverts to Presto** (`pet-presto.md`).

## 6. Head update (`BilaterusHead`, called from `Bilaterus::Update` via `0x004f0ea0`)

`static` (from raw binary analysis of the Update function and partial hex decoding).

Each BilaterusHead is a steered endpoint of the worm. The owning Bilaterus calls the head update function **twice** per tick (once per head).

The head likely steers toward prey (fish) using a chase mechanic, then the bones follow. Movement constants observed in the Update function:
- Velocity increments: **±3.5**, **±4.0**
- Position thresholds: **80.0** (segment size), **30.0**, **50.0**
- **Visibility byte `+0x15e`**: controls whether the head is drawn (`pet-blip.md`).

The two heads are the "mouths" — the worm's attack points. The movement pattern appears to be a slithering motion where the heads lead and the bones trail behind.

## 7. Bone update (`BilaterusBone`)

`static` (inferred from structure; not yet decoded).

Each bone follows the head or the preceding bone, maintaining the worm's body shape. Bones are 80×80 segments that form the body between the two heads. BilaterusBone has an 81-slot vtable at `0x0059757c`; slot 22 is the base `Widget::Update` (`0x0046e310`), meaning bones use the default update (no custom override — they are likely positioned by the owner's Update function).

## 8. Drawing

`static` (from `pet-blip.md` and board.md z-order).

### 8.1 BilaterusHead draw (vtable slot 26 @ `0x004e50b0`)

From `pet-blip.md`:
- The health bar is drawn at **(-20, h-3)** relative to the head widget, where `h` = widget height.
- The bar is **mirrored when a state field == 4** (likely a head-direction state).
- Uses `health / 100` to compute the bar fill fraction (max health 100.0).
- Visibility is gated by byte `+0x15e` on the head.

### 8.2 Z-order

From `board.md`: Bilaterus segments in `+0xf4` are drawn in the z-order rebuild sequence, interleaved with other objects. The segments appear as large in-tank objects.

## 9. Death (`0x004fb4c0`)

`static` (from cached Ghidra decompilation).

Called on a **segment** (head) when health drops to ≤ 0 (via `owner+0x1d8`). The function accesses the owner via `segment+0x158`.

```
FUN_004fb4c0(segment):
    owner = *(segment + 0x158)
    if owner+0x164 != 0:          // already dead
        FUN_004dbfb0()            // alternate cleanup
        return
    
    coin_type = 4                 // gold
    if *(segment + 0x160) != 0:
        coin_type = 5             // diamond
    
    AddCoin(segment.x, segment.y, 0, coin_type)
    FUN_004fb1c0()                // segment cleanup / removal from lists
    
    // remove from widget tree and board lists
    vtable_call(segment+0x10, slot 4)   // widget removal
    vtable_call(segment+0x88, slot 34)  // board list removal
    
    owner+0x164 = 1               // set dead flag
    owner+0x15c = 0               // clear state
    
    if segment.shadow != NULL:
        Shadow::vfunction76(segment.shadow)
    
    PlaySound(0x11d, 3, 1.0)     // death sound 1
    PlaySound(0x11e, 3, 1.0)     // death sound 2
```

The `segment+0x160` field likely distinguishes the two heads (0 for head 0, non-zero for head 1). Only the **first** head to trigger death drops a coin; the second finds `owner+0x164 == 1` and takes the alternate cleanup path.

## 10. `Bilaterus::Update` (vtable slot 22 @ `0x004fe250`)

`static` (from raw binary analysis; call targets and constant references extracted from bytes).

The Update function manages the entire worm each tick:

1. **Null/pause check**: if no board or board paused (`+0x94`), return.
2. **Virtual Tank lifespan**: if mode == 5 (Virtual Tank) AND tick counter (`+0x28`) > **4320** (~43.2s): triggers death/removal via `FUN_00543640` and death function.
3. **Timer update**: calls `FUN_004f22c0` — decrements timers, manages music/animation cues.
4. **Head updates**: calls `FUN_004f0ea0` **twice** (once per head). This function dispatches the head's chase/movement logic.
5. **Bone positioning**: updates bone segment positions using velocity constants **±3.5** and **±4.0**, with position clamp at **80.0** spacing.
6. **Health check**: if `owner+0x1d8 ≤ 0`, calls death function `0x004fb4c0`.

Call targets from the Update function:
| Address | Purpose |
|---|---|
| `0x00543640` (×3) | Position lookup / closest-fish search |
| `0x00538230` (×2) | Sound play |
| `0x004d6830` | Screen shake or visual effect |
| `0x004f22c0` | Timer/state tick (music, counters) |
| `0x004fb1c0` | Segment cleanup |
| `0x004f0ea0` (×2) | Head update (once per head) |
| `0x004edea0` | Bone/segment update |
| `0x004fb4c0` (×2) | Death function (two health-check sites) |

## 11. Pet interactions

`static` unless the pet spec section is `traced` (noted per row).

| Pet | Interaction | Source |
|---|---|---|
| **Rufus** (7) | Targets Bilaterus segments; deals 2.0 HP/tick per overlapping segment; centre offset 40 | `pet-rufus.md` (`traced`) |
| **Itchy** (2) | Targets Bilaterus with centre offset 40 (vs 80 for regular aliens); 1.0 HP/tick | `pet-itchy.md` (`traced`) |
| **Gash** (17) | Chases Bilaterus; 3.0 HP/hit per segment, kills at ≤ 0 immediately | `pet-gash.md` (`traced`) |
| **Blip** (13) | Draws health bar on the BilaterusHead; spawn-y pin to 280 during reticle | `pet-blip.md` (`traced` for core; Bilaterus path `static`) |
| **Wadsworth** (9) | Shelters guppies from Bilaterus (threat = alien OR Bilaterus OR missile) | `pet-wadsworth.md` (`traced`) |
| **Gumbo** (12) | Flees from Bilaterus; targets centre offset 40 | `pet-gumbo.md` (`traced`) |
| **Stinky** (0) | Shells while Bilaterus is present (`+0xf4` non-empty counts as "alien exists") | `pet-stinky.md` (`traced`) |

## 12. Wave freeze

`traced` (via `alien-sylvester.md` §1 step 2, confirmed in multiple tank-1 replays for the `+0xb8`/`+0xf4`/`+0xdc` check).

The wave countdown (`board+0x2c0`) is frozen while **any** of `+0xb8` (aliens), `+0xf4` (Bilaterus segments), or `+0xdc` (missiles) is non-empty. This means the Bilaterus must be fully killed (all segments removed from `+0xf4`) before the next wave can begin counting down.

## 13. Levels

`static` (from `shop-bar.md` and `tank4-plan.md`).

Bilaterus (kind 8) appears on:
- **4-2**: kind 8 is the fixed wave kind.
- **4-4**: re-rolls between kind 8 (50%) and pair kind 11 (Destructor + kind 6, 50%).
- **4-5**: re-rolls `r % 5` → pair kind 10 (40%), pair kind 11 (40%), kind 8 (20%).
- **Pair kind 12** (Balrog + Bilaterus): only in mode 4 on 4-5 when `waves_spawned ≥ 5` and the re-roll gave kind 8.

## 14. BoxingGlove Draw (`0x004e1000`)

`static` (from raw binary analysis).

The draw function is called by the BilaterusHead (or Walter) as part of the owner's draw, **behind** the owner's body. It uses `IMAGE_FIST` (at global `0x005e8e50`; for Walter, this is `IMAGE_WALTER`; for Bilaterus, the image pointer may differ — needs confirmation).

**Animation frame**: `d = |countdown − 15|`, creating a ping-pong. `d = 0` at full extension (countdown 15), `d = 14` at start/end.

**Visibility**: `a = 9 − d`; if `a < 1` (d ≥ 9, i.e. countdown ≤ 6 or ≥ 24): not drawn. If `a ≤ 5`: `a = 6` (capped). The arm cel is `max(a, 6)`, ranging 6–9.

| Countdown | d | a | Drawn? | Cel |
|---|---|---|---|---|
| 29–24 | 14–9 | ≤ 0 | No | — |
| 23 | 8 | 1 → 6 | Yes | 6 |
| 22–16 | 7–1 | 2 → 6 | Yes | 6 |
| 15 | 0 | 9 | Yes | 9 |
| 14 | 1 | 8 | Yes | 8 |
| 13 | 2 | 7 | Yes | 7 |
| 12–8 | 3–7 | 2–6 | Yes | 6 |
| 7 | 8 | 1 → 6 | Yes | 6 |
| 6–1 | 9–14 | ≤ 0 | No | — |

Visible for **17 ticks** (countdown 7–23).

**Two draws per frame**:
1. **Arm/body**: source rect `(cel·80, 160, 80, 80)` from the spritesheet, at dest `(−3, 2)` for left or `(83, 2)` mirrored for right.
2. **Fist overlay**: source rect `(400, 160, 80, 80)` (a fixed frame at row 5), at dest `(6d − 20, 0)` for left or `(100 − 6d, 0)` mirrored for right. This slides with the animation: at full extension (d = 0) the fist is at x = −20 (left) or x = 100 (right); as d increases it retracts toward the centre.

The first 5 ticks after creation the glove is at (−200, −200) off-screen (the owner gate in Update prevents positioning), so nothing is visible even though some of those frames compute a visible `a`.

## 15. Key unknowns (reduced)

`static` — gaps requiring deeper analysis or dynamic verification:

- **Head movement/steering**: exact velocity formulas, fish-chase targeting, bounce/clamp at tank edges. Constants ±3.5, ±4.0, 80.0, 30.0, 50.0 are identified but their roles in the chase algorithm are not yet decoded.
- **BiFish** (`+0xf8`): when and how digested fish appear inside the worm body, their lifecycle.
- **Constructor full RNG**: only 1 draw confirmed; there may be more for sub-object placement or initial head direction.
- **Exact coin types**: types 4 and 5 from the death function need verification against the coin type enum.
- **Head field `+0x160`**: assumed to be head index (0 or 1) determining coin type at death; needs confirmation.
- **BoxingGlove push sign for Bilaterus**: the code always passes `s = −1` (leftward push). If the original Bilaterus can punch rightward and the push should follow the direction, this is a subtle bug in the analysis. Needs dynamic verification.
- **IMAGE_FIST for Bilaterus**: the draw reads from global `0x005e8e50`; for Walter this is `IMAGE_WALTER`. Needs confirmation that the Bilaterus head's draw sets up the correct image before calling the glove's draw.
