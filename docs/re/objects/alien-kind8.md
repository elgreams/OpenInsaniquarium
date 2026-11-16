# Alien kind 8: Bilaterus (`Sexy::Bilaterus`, type `0x17`, board list `+0xf4`)

Status: `draft` (static analysis only). Bilaterus is not a `Sexy::Alien`; it inherits directly from `GameObject` → `Widget`. Addresses are inner-PE VAs; offsets are raw `this`-relative. See `board.md` for the type code and list, `tank4-plan.md` for context.

Notation: same as `alien-sylvester.md` — `rand(n)` = app MTRand `% n`; "tick" = one 28 ms logic tick.

## 1. Class hierarchy and sub-objects

`static` (from RTTI class-name strings and board.md RTTI tree).

Bilaterus does **not** inherit from `Sexy::Alien`. Its RTTI chain is:

```
Widget → GameObject → Bilaterus
```

Sub-object classes (also Widget → GameObject):
- **`BilaterusHead`** (type descriptor `.?AVBilaterusHead@Sexy@@`): the two "mouths" of the worm. Each is a full GameObject (0x208 bytes). Board list: none (owned by the parent, not in a typed vector).
- **`BilaterusBone`** (type descriptor `.?AVBilaterusBone@Sexy@@`): the body segments. Each is a full GameObject (0x1f0 bytes). Board list: none.
- **`BiFish`** (type `0x24`, board list `+0xf8`): the digested-fish display inside the worm. Separate from the Bilaterus itself.

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

The Bilaterus vtable is at `0x00598b94`. Three call sites write this vtable pointer:
- `0x004f06ef` — destructor.
- `0x004f98ea` — simple constructor (default init).
- `0x004fe03e` — parameterised constructor (used by AddAlien).

The parameterised constructor:
1. Calls the base `GameObject` constructor.
2. Creates **2 `BilaterusHead`** objects (each 0x208 bytes).
3. Creates **6 `BilaterusBone`** objects (each 0x1f0 bytes).
4. Sets the warp-in counter to **15** (same as Alien's warp-in).
5. Consumes **1 app RNG draw** (direction or layout; exact use TBD with Ghidra).

Total segments: 8 (2 heads + 6 bones). All are added to board list `+0xf4`. Each segment's `+0x158` → owning Bilaterus.

Segment widgets are **80×80** (from `pet-rufus.md`, `pet-itchy.md`), with centre offset 40.

## 4. Health

`static` (cross-referenced from multiple traced pet specs).

Health is stored at the **first head's `+0x1d8`** (a `double`), not on the owner Bilaterus directly.

| Fact | Source | Confidence |
|---|---|---|
| Max health = **100.0** | `pet-blip.md`: health bar draws `health / 100` | `static` (Blip spec is `traced` but the Bilaterus path in it is `static`) |
| Rufus deals **2.0 HP/tick** per overlapping segment | `pet-rufus.md` §3 | `traced` (Rufus spec) |
| Gash deals **3.0 HP/hit** per segment, kills at ≤ 0 via `0x004fb4c0` | `pet-gash.md` §3 | `traced` (Gash spec) |
| Itchy deals **1.0 HP/tick** (default rate) | `pet-itchy.md` §3 | `traced` (Itchy spec) |
| Death function: `0x004fb4c0` called on `seg+0x158` (the owner) | `pet-gash.md` §3 | `static` |

The divisor for laser/click damage is not yet known from cross-references alone. The Alien class uses `health -= 3 * laser` for kinds 1–3 and `health -= 2 * laser + 2` for kinds 5–6 (`alien-sylvester.md` §4.2, `alien-kind5.md`). Bilaterus likely has its own formula since it is a different class. **Needs Ghidra** to read the Hit function.

## 5. `Bilaterus::Update` (vtable slot 22 @ `0x004fe250`)

`static` (from raw hex decoding of the function at `0x004fe250`).

The Update function:
1. Returns if no board or board is paused.
2. **Warp-in** (`+0x158`, starts at 15): decrements each tick; while non-zero, the segments are not fully active. Same 15-tick mechanic as `Alien` (§3.1 of `alien-sylvester.md`).
3. Updates all **heads** (calls each head's update function, §6).
4. Updates all **bones** (calls each bone's update function, §7).
5. Manages an **attack timer** (~1000 ticks observed in the hex; exact threshold TBD). This likely gates the worm's attacks (eating fish or firing projectiles).
6. **Health check**: if the first head's health (`+0x1d8`) ≤ 0, calls the death function `0x004fb4c0`.

## 6. Head update (`BilaterusHead`, function near `0x004f0ea2`)

`static` (from partial hex decoding; the function uses FPU comparisons and velocity adjustments).

Each BilaterusHead is a steered endpoint of the worm. The update function:
- Reads and writes position doubles at offsets `+0x78`, `+0x80` (or similar; exact mapping needs Ghidra).
- Applies velocity adjustments (`+0x88`, `+0x90`, `+0x98`) with FPU compare-and-branch patterns.
- The head likely steers toward prey (fish) using a chase mechanic, then the bones follow.
- **Visibility byte `+0x15e`**: controls whether the head is drawn (`pet-blip.md`).

The two heads are the "mouths" — the worm's attack points. The movement pattern appears to be a slithering motion where the heads lead and the bones trail behind, forming the segmented body.

## 7. Bone update (`BilaterusBone`)

`static` (inferred from structure; not yet decoded).

Each bone follows the head or the preceding bone, maintaining the worm's body shape. Bones are 80×80 segments that form the body between the two heads.

## 8. Drawing

`static` (from `pet-blip.md` and board.md z-order).

### 8.1 BilaterusHead draw (vtable slot 27 @ `0x004e50b0`)

From `pet-blip.md`:
- The health bar is drawn at **(-20, h-3)** relative to the head widget, where `h` = widget height.
- The bar is **mirrored when a state field == 4** (likely a head-direction state).
- Uses `health / 100` to compute the bar fill fraction (max health 100.0).
- Visibility is gated by byte `+0x15e` on the head.

### 8.2 Z-order

From `board.md`: Bilaterus segments in `+0xf4` are drawn in the z-order rebuild sequence, interleaved with other objects. The segments appear as large in-tank objects.

## 9. Death (`0x004fb4c0`)

`static` (address from `pet-gash.md`; behaviour not yet decoded).

Called on the **owner Bilaterus** (via `seg+0x158`) when health drops to ≤ 0. Expected to:
1. Remove all segments from `+0xf4`.
2. Drop a reward (diamond or similar).
3. Trigger death effects (debris, sound).
4. Allow the wave countdown to resume (since `+0xf4` becomes empty).

The exact RNG draws, debris pattern, and reward kind need Ghidra analysis.

## 10. Pet interactions

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

## 11. Wave freeze

`traced` (via `alien-sylvester.md` §1 step 2, confirmed in multiple tank-1 replays for the `+0xb8`/`+0xf4`/`+0xdc` check).

The wave countdown (`board+0x2c0`) is frozen while **any** of `+0xb8` (aliens), `+0xf4` (Bilaterus segments), or `+0xdc` (missiles) is non-empty. This means the Bilaterus must be fully killed (all segments removed from `+0xf4`) before the next wave can begin counting down.

## 12. Levels

`static` (from `shop-bar.md` and `tank4-plan.md`).

Bilaterus (kind 8) appears on:
- **4-2**: kind 8 is the fixed wave kind.
- **4-4**: re-rolls between kind 8 (50%) and pair kind 11 (Destructor + kind 6, 50%).
- **4-5**: re-rolls `r % 5` → pair kind 10 (40%), pair kind 11 (40%), kind 8 (20%).
- **Pair kind 12** (Balrog + Bilaterus): only in mode 4 on 4-5 when `waves_spawned ≥ 5` and the re-roll gave kind 8.

## 13. Key unknowns (need Ghidra)

`static` — these are gaps that require deeper binary analysis:

- **Bilaterus constructor full RNG order**: only 1 draw confirmed; there may be more for sub-object placement.
- **Movement mechanics**: how the worm slithers (head-leads-bones chain, exact velocity formulas, bounce/clamp).
- **Attack pattern**: whether the worm eats fish directly (like Alien kinds 1–3) or uses a different attack.
- **Laser/click damage formula**: the Hit function on Bilaterus (not `Alien::Hit`).
- **Death details**: debris RNG, reward coin kind, DeadAlien equivalent (or lack of one).
- **Exact field layout**: the offsets on the Bilaterus owner itself (beyond health at `+0x1d8` via first head).
- **BilaterusBone update function address** and its following-chain logic.
- **BiFish** (`+0xf8`): what digested fish look like inside the worm and how they're created.
