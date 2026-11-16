# Plan: tank 4 (levels 4-1 to 4-5)

Status: plan only. Facts cited below are `static` unless a spec says otherwise.

## What tank 4 adds

From `SetupShop` (`0x00541200`, `sim/shop-bar.md#shop-setup`):

- **Shop:** **breeder** (slot 1, $200 — replaces the guppy), food quality, food quantity, **carnivore** (slot 4, $1000), **ultravore** (slot 9, $10 000), weapon ($5000), egg piece. No guppy buy, no star potion, no Starcatcher, no Guppycruncher/Beetlemuncher. On 4-1 the ultravore and weapon slots are unmapped.
- **Egg prices:** 3000, 25 000, 50 000, 75 000, 99 999.
- **Wave kinds `+0x2bc`:** 4-1 → 3 (Balrog), 4-2 → **kind 8 (Bilaterus)**, 4-3 → 4 (Gus), 4-4 → kind 8, 4-5 → kind 8. No RNG draws in `SetupShop` for tank 4.
- **Post-spawn re-rolls** (`0x548d35`): 4-1 and 4-2 keep their kind every wave; 4-3 re-rolls between kind 4 (Gus, 50 %) and **pair kind 10** (kind 7 + Balrog, 50 %); 4-4 re-rolls between kind 8 (Bilaterus, 50 %) and **pair kind 11** (Destructor + kind 6, 50 %); 4-5 re-rolls `r % 5` → pair kind 10 (40 %), pair kind 11 (40 %), kind 8 (20 %). Kind 12 (Balrog + Bilaterus) only in mode 4. Full detail in `shop-bar.md`.
- **Initial countdown `+0x2c0`:** 3000 (InitLevel default; SetupShop does not override for tank 4).
- **Stage-up reveals** (`0x004f1840` at `0x4f1b48`): when a guppy reaches stage 2, reveals slots 2 (food quality), 3 (food quantity), **4** (carnivore). No messages. Then the purchase chains: buy carnivore on 4-1 → reveal 9 (ultravore), reveal 11 (egg; skips weapon); buy carnivore on 4-2+ → reveal 9, 10 (weapon), 11. Buy ultravore → reveal 10, 11.
- **New objects:**
  - **Breeder** (`Sexy::Breeder`, type `0x0a`, list `+0xc0`): bought fish that produces baby guppies periodically. Replaces the guppy buy for the tank 4 economy (you buy breeders to get guppies). Spec needed: `objects/breeder.md`.
  - **Ultravore** (`Sexy::Ultra`, type `0x06`, list `+0xd4`): high-tier fish, $10 000. Likely eats carnivores and drops high-value coins. Spec needed: `objects/ultravore.md`.
  - **Bilaterus** (`Sexy::Bilaterus`, type `0x17`, list `+0xf4`): the tank-4 alien, kind 8. Its own board list separate from `+0xb8` (regular aliens). The wave countdown freezes while `+0xf4` is non-empty (`alien-sylvester.md` §1 step 2). Spec needed: `objects/alien-bilaterus.md`.
  - **Pair wave kinds** (10, 11): a single wave that spawns two aliens at positions A and B. The pair-kind table in `AddAlien` (`0x00545620`) defines: 10 → kind 7 + Balrog, 11 → Destructor + kind 6. `confirmed` from binary PUSH immediates; `static` until a tank 4 recording.
- **Backdrop:** `FUN_00538a10(n)` in `SetupShop` for tank 4; exact index to be confirmed.
- **Pets hatched** (from `hatch-screen.md#pet-award-table`): 4-1 Nimbus (15), 4-2 Amp (16), 4-3 Gash (17), 4-4 Angie (18), 4-5 **no pet** (id 999, "Evil Alien Mastermind").

`static` for the list itself; each item's confidence is in its own spec.

## Prerequisites

- **Tank 3 complete** (all 7 steps done or in progress). The sim already handles: Guppycruncher/Beetlemuncher/Larva economy, alien kinds 6 and 7 (including kind-1 missiles and submerge), pair kind 9 (Sylvester + Balrog), and the 3-5 re-roll.
- **Bonus screen** after tank 3: same code path as after tanks 1 and 2. `traced`.

## Steps

`static` — same workflow as tanks 2 and 3: spec from the binary → build → record the original playing the level (its own `-record` demo + a CE capture) → replay and fix until exact.

### 1. Ghidra: SetupShop, egg prices, wave kinds, re-roll formulas

**Done** (`static`/`confirmed`).

SetupShop tank-4 branch: shop slots, egg prices, wave kinds, re-roll formulas all recorded in `shop-bar.md`. The pair-kind table and per-level re-roll formulas are `confirmed` from binary analysis at `0x548d35`–`0x548df4` and `AddAlien` at `0x00545620`.

### 2. Breeder spec and 4-1 baseline

`static` (a plan).

4-1 uses only the Balrog (kind 3, already built) and the new shop layout. The Breeder is the core new fish type.

**Ghidra:** spec the Breeder (`Sexy::Breeder`, type `0x0a`). Expected: Fish-based, with a periodic guppy spawn (similar to Prego's pet mechanic but as a bought fish). Identify the spawn interval, RNG draws, interaction with hunger/death.

**Record:** a 4-1 session (Stinky + 2 others) that buys a breeder and exercises the full reveal chain (carnivore → ultravore → egg, without weapon on 4-1), plus a Balrog wave.

**Test:** `tank_4_1` replays the recording exactly.

### 3. Ultravore spec

`static` (a plan).

**Ghidra:** spec the Ultravore (`Sexy::Ultra`, type `0x06`). Expected: eats carnivores/medium-large fish (like Carnivore eats guppies) and drops high-value coins. Identify chase rules, bite box, coin kind, hunger/death.

**Record:** a 4-2 session that buys a carnivore, then an ultravore, and exercises the feeding chain (breeder → guppy → carnivore → ultravore → coin). Also exercises a Bilaterus wave (kind 8).

**Test:** `tank_4_2_ultravore` replays exactly.

### 4. Bilaterus spec (alien kind 8)

`static` (a plan).

The Bilaterus (`Sexy::Bilaterus`, type `0x17`) is the tank-4 boss alien. It has its own board list (`+0xf4`) and likely multi-segment/multi-tentacle mechanics (the warning "FOURTEEN ALIEN SIGNATURES" on 4-4 suggests 14 hit zones). It freezes the wave countdown while alive.

**Ghidra:** spec the Bilaterus: class layout, spawn path in `AddAlien` for kind 8, Update (movement, attack, segments), damage/death, interaction with pets. This is the largest new object on tank 4.

**Record:** a 4-2 (or 4-4) session focused on the Bilaterus: spawn, attack, taking damage, death.

**Test:** `tank_4_bilaterus` replays exactly.

### 5. Pair wave kinds (10, 11)

`static` (a plan).

Pair kinds 10 and 11 spawn two aliens per wave. Kind 10 (kind 7 + Balrog) first appears on 4-3; kind 11 (Destructor + kind 6) on 4-4. The `AddAlien` pair paths are `confirmed` from binary; the re-roll formulas are `confirmed`.

**Build:** the re-roll code for tank 4 in `Board::Update` (the post-spawn re-roll at `0x548d35`).

**Record:** a 4-3 session that gets a pair kind 10 wave (50 % chance per re-roll) and a 4-4 session with a pair kind 11 wave.

**Test:** `tank_4_3_pair` and `tank_4_4_pair` replay exactly.

### 6. 4-5 (re-roll, all wave kinds possible)

`static` (a plan).

The final non-boss level. 4-5 re-rolls `r % 5` after every wave: 40 % pair kind 10, 40 % pair kind 11, 20 % Bilaterus. Egg price is $99 999 (the cap). The hatch screen for 4-5 awards no pet (id 999, "Evil Alien Mastermind").

**Record:** a 4-5 session exercising at least one Bilaterus and one pair wave.

**Test:** `tank_4_5` replays exactly.

### 7. Open static claims and integration

After steps 2–6, remaining `static` claims:

- **Rhubarb flings Ultravores** (`pet-rhubarb.md`): needs a recording with Rhubarb and an Ultravore near the floor.
- **Gumbo lure with Breeders/Ultravores** (`pet-gumbo.md`): whether `board+0x174` affects Breeders or Ultravores (`+0x229` state unknown for these types).
- **Angie revival of Ultravores and Breeders** (`pet-angie.md`): re-creation functions listed but not traced.
- **Seymour's slow-fall on Ultravores/Breeders** (`pet-seymour.md`): whether his coin-fall rate applies to their lists.
- **Blip reticle for kind 8** (`pet-blip.md`): alien health bars and spawn-y pin for Bilaterus.
- **Shrapnel bombs vs Breeders** (`pet-shrapnel.md`): `+0xc0` Breeders in the bomb kill list.

These can be batched into focused recordings and folded into the step that first records the relevant level.

Report back after each step.

`static` (a plan).
