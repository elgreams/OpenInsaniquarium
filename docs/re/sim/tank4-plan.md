# Plan: tank 4 (levels 4-1 to 4-5 + boss)

Status: plan only. Facts cited below are `static` unless a spec says otherwise. Per-item confidence is in the linked specs.

## What tank 4 adds

From `SetupShop` (`0x00541200`, `sim/shop-bar.md#shop-setup`):

- **Shop:** **breeder** (slot 1, $200 — replaces the guppy), food quality, food quantity, **carnivore** (slot 4, $1000), **ultravore** (slot 9, $10 000), weapon ($5000), egg piece. No guppy buy, no star potion, no Starcatcher, no Guppycruncher/Beetlemuncher. On 4-1 the ultravore and weapon slots are unmapped.
- **Egg prices:** 3000, 25 000, 50 000, 75 000, 99 999.
- **Wave kinds `+0x2bc`:** 4-1 → 3 (Balrog), 4-2 → **kind 8 (Bilaterus)**, 4-3 → 4 (Gus), 4-4 → kind 8, 4-5 → kind 8. No RNG draws in `SetupShop` for tank 4.
- **Post-spawn re-rolls** (`0x548d35`): 4-1 and 4-2 keep their kind every wave; 4-3 re-rolls between kind 4 (Gus, 50 %) and **pair kind 10** (kind 7 + Balrog, 50 %); 4-4 re-rolls between kind 8 (Bilaterus, 50 %) and **pair kind 11** (Destructor + kind 6, 50 %); 4-5 re-rolls `r % 5` → pair kind 10 (40 %), pair kind 11 (40 %), kind 8 (20 %). Kind 12 (Balrog + Bilaterus) only in mode 4. Full detail in `objects/alien-pairs.md` and `shop-bar.md`.
- **Initial countdown `+0x2c0`:** 3000 (InitLevel default; SetupShop does not override for tank 4). `static`.
- **Stage-up reveals** (`0x004f1840` at `0x4f1b48`): when a guppy reaches stage 2, reveals slots **2** (food quality), **3** (food quantity), **4** (carnivore). No messages. Then the purchase chains: buy carnivore on 4-1 → reveal 9 (ultravore), reveal 11 (egg; skips weapon); buy carnivore on 4-2+ → reveal 9, 10 (weapon), 11. Buy ultravore → reveal 10, 11.
- **Economy chain:** breeder → food → guppy → (guppy grows) → carnivore eats small guppy → ultravore eats carnivore → kind-7 coin ($2000). The player buys breeders ($200) to generate guppies, then scales up through carnivores ($1000) to ultravores ($10 000). This is the highest-value chain in the game.
- **New objects:**
  - **Breeder** (`Sexy::Breeder`, type `0x0a`, list `+0xc0`, not a `Fish`): $200, slot 1. Eats food pellets and spawns baby guppies periodically. 80×80 widget, shadow type 0. Hunger at `+0x9c`, stages at `+0x1a0` (dead fish kinds 10/11/12). Zorf feeds hungry breeders. Level start places one breeder (2 position draws + ctor draws). Ghidra-analysed spec: `objects/breeder.md`.
  - **Ultravore** (`Sexy::Ultra`, type `0x06`, list `+0xd4`, is a `Fish`): $10 000, slot 9. Eats carnivores from `board+0xa4` (+900, cap 1300, gore `r%3+2` × 2 draws) and drops kind-7 coins ($2000) every 200–449 ticks. 160×160 widget, shadow type 2, divisor ∈ {2.0, 1.8, 1.6}. 8 ctor draws (Fish::Init 6 + hunger `r%200+600` + coin-interval `r%250+200`). Buy = 13 app draws, `SOUND_SPLASHBIG` (no `PlaySplash`). 3-bubble splash-in branch. Ghidra-analysed spec: `objects/ultravore.md`.
  - **Bilaterus** (`Sexy::Bilaterus`, type `0x17`, list `+0xf4`, not an `Alien`): the tank-4 alien, kind 8. 2 heads + 6 bones = 8 segments, all on `+0xf4`. Health 100.0 at first head's `+0x1d8` (single pool for both heads). Attacks fish with BoxingGlove projectiles. The player **cannot damage it with the laser** — only pets (Itchy 1.0/tick, Rufus 2.0/tick, Gash 3.0/hit) and Stanley missiles (direct 30 HP, bounceback 15/55 HP) deal damage. 1 app draw at construction, no shadow. Wave countdown freezes while `+0xf4` is non-empty. Warning: "FOURTEEN ALIEN SIGNATURES DETECTED" / "JUST KIDDING. ONLY TWO!". Ghidra-analysed spec: `objects/alien-kind8.md`.
  - **Pair wave kinds** (10, 11): a single wave spawning two aliens at positions A and B via recursive `AddAlien`. 10 → kind 7 + Balrog (first on 4-3), 11 → Destructor + kind 6 (first on 4-4). 6 app draws per pair spawn. Warning: "MULTIPLE ALIEN SIGNATURES DETECTED". Full spec: `objects/alien-pairs.md`.
- **Backdrop:** index **5** (`FUN_00538a10(5)` in `SetupShop` tank-4 branch; `+0x3e8`). Full mapping: tank 1 → 1, tank 2 → 2, tank 3 → 4, tank 4 → 5, tank 5 → 6 (backdrop 3 is skipped in Adventure). `static` (cached Ghidra decompilation of `SetupShop` at `0x00541200`). Tank-4 music: module 2, order 45 (`assets/music.md`).
- **Pets hatched** (from `hatch-screen.md#pet-award-table`): 4-1 Nimbus (15), 4-2 Amp (16), 4-3 Gash (17), 4-4 Angie (18), 4-5 **no pet** (id 999, "Evil Alien Mastermind").

`static` for the list itself; each item's confidence is in its own spec. The pair dispatch table and re-roll formulas are `confirmed` from binary analysis (`alien-pairs.md`, `shop-bar.md`). Breeder and Ultravore specs are `static` (Ghidra-analysed, no recordings). Bilaterus spec is `static` with some sections `traced` via pet specs.

## Prerequisites

- **Tank 3 complete** (all 7 steps done). The sim already handles: Guppycruncher/Beetlemuncher/Larva economy, alien kinds 6 and 7 (including kind-1 missiles, submerge/surface, panic dive), and the 3-5 re-roll.
- **Bonus screen** after tank 3: same code path as after tanks 1 and 2. `traced`.

`static` (prerequisites are met).

## Steps

`static` — same workflow as tanks 2 and 3: spec from the binary → build → record the original playing the level → replay and fix until exact.

### 1. Ghidra: SetupShop, egg prices, wave kinds, re-roll formulas

**Done** (`static`/`confirmed`).

SetupShop tank-4 branch: shop slots, egg prices, wave kinds, re-roll formulas all recorded in `shop-bar.md`. The pair-kind table and per-level re-roll formulas are `confirmed` from binary analysis at `0x548d35`–`0x548df4` and `AddAlien` at `0x00545620`. Breeder and Ultravore specs (`objects/breeder.md`, `objects/ultravore.md`) are complete at `static` confidence from Ghidra analysis. Bilaterus spec (`objects/alien-kind8.md`) covers class hierarchy, spawn, health, damage, attack, death; head movement and BiFish details remain `static` gaps.

### 2. Breeder and 4-1 baseline

`static` (a plan). Recording done: `tank4-1_full_01` (2026-11-17).

4-1 uses only the Balrog (kind 3, already built) and the new shop layout. The Breeder is the core new fish type.

**Spec status:** `objects/breeder.md` has the full Ghidra-analysed field table, construction (5-param ctor, 2-param ctor), Die path, death sounds, interactions (aliens, Rhubarb, Zorf, Shrapnel, Angie, missiles), and the starvation check. Open: the Update function address, food chase mechanics, guppy spawn mechanism (interval, timer fields, freeze-during-waves, RNG draws), and drawing.

**Build:** Breeder type (spawn, food chase, hunger, death, guppy birth timer), the tank-4 shop layout (slot 1 replaces guppy), the reveal chain (buy carnivore → reveal ultravore → reveal weapon/egg; 4-1 skips weapon), and the level-start path (`0x005471e0`: one breeder with `+0x1a4 = 2`).

**Record:** ~~a 4-1 session (Stinky + 2 others) that buys a breeder and exercises the full reveal chain, plus a Balrog wave.~~ **Done.** `tank4-1_full_01` (Stinky, Itchy, Zorf; CD=5000, 8000 ticks). Exercises: breeder spawning 4 guppies, guppy growth to stage 2, reveal chain (food quality/quantity/carnivore → carnivore buy at tick 4778 → egg reveal), Balrog wave at tick 5268 (killed by Itchy). Breeder eaten by Balrog. Egg counter confirmed: InitLevel sets `+0x43c = 1`, completes at 4 (3 purchases at $3000 each on 4-1).

**Test:** `tank_4_1` replays the recording. Currently diverges at tick 2299: `growth_needed=7` (expected 8). Root cause is accumulated app-RNG drift from `static`/`guess` breeder ctor draws (5 draws by Grubber analogy). Fix: either enable app-stream resync for the test, or verify the breeder ctor draw count via Ghidra (`0x00547270`). See `/tmp/t4-step2-analyze.txt` for full RNG accounting.

### 3. Ultravore and the full feeding chain

`static` (a plan).

**Spec status:** `objects/ultravore.md` is fully Ghidra-analysed: vtable (19 slot overrides), `Ultra::Init` fields (type 6, stage 6, 160×160, floor 310, ceiling 75, `+0x229 = 0`), `BuyUltravore` at `0x00544e60` (13 draws), prey list (`board+0xa4`, carnivores), bite check (slot 86, inner 160×80 / outer 220×120 boxes), eat (+900 cap 1300, gore), coin drop (kind 7, $2000, at wx+40/wy+90, vy −1.0), hunger (shared `0x004d6ef0`, −1/tick), 3-bubble splash-in, death (kind-6 DeadFish, floor 310). Open: animation slot 89, census slot 70, drawing details.

**Build:** Ultravore type (spawn with splash-in, prey chase of carnivores, bite/eat, coin drop, hunger/death), and the feeding chain: breeder → guppy → carnivore → ultravore → kind-7 coin.

**Record:** a 4-2 session that buys a carnivore, then an ultravore, and exercises the feeding chain. Also exercises the first Bilaterus wave (kind 8).

**Test:** `tank_4_2_ultravore` replays exactly.

### 4. Bilaterus (alien kind 8)

`static` (a plan).

The Bilaterus (`Sexy::Bilaterus`, type `0x17`) is the tank-4 boss alien — not an `Alien` subclass, with its own multi-segment architecture.

**Spec status:** `objects/alien-kind8.md` covers: class hierarchy (Widget → GameObject → Bilaterus; BilaterusHead, BilaterusBone, BiFish, BoxingGlove sub-objects), spawn path (kind-8 branch in `AddAlien`, 1 app draw), health (100.0, single pool, hit flash 10 ticks, dead flag `+0x164`), damage formulas (no laser; Stanley direct 30 / bounceback 15 or 55; Itchy 1.0 / Rufus 2.0 / Gash 3.0), BoxingGlove attack (5-punch limit, 40-tick cooldown, sound `0x136`), Update structure (head updates × 2, bone positioning, VT lifespan 4320), death (coin type 4 or 5 by head, dual death sounds `0x11d`/`0x11e`), pet interactions (7 pets cross-referenced). Open: head steering/chase algorithm (velocity constants ±3.5/±4.0 identified but roles unclear), BoxingGlove target effect (killed? pushed?), BiFish lifecycle, full constructor RNG.

**Build:** Bilaterus type, BoxingGlove, BilaterusHead, BilaterusBone, BiFish. This is the largest new object on tank 4. The head movement algorithm and BoxingGlove hit effect are the main unknowns — may require additional Ghidra work during implementation.

**Record:** a 4-2 (or 4-4) session focused on the Bilaterus: spawn (warp-in), movement (head steering, bone trailing), attack (BoxingGlove hitting a fish), taking damage from pets and Stanley, and death.

**Test:** `tank_4_bilaterus` replays exactly.

### 5. Pair wave kinds (10, 11)

**Done** (`static`/`confirmed`).

Pair-kind mechanics fully analysed from Ghidra: recursive `AddAlien` spawn sequence, pair dispatch table (`confirmed` from PUSH immediates), list insertion and update order, RNG cost per pair (6 draws for kinds 10/11), warning messages, and wave freeze behaviour. Full spec in `objects/alien-pairs.md`. Re-roll formulas for 4-3/4-4/4-5 also recorded in `shop-bar.md`.

Remaining for this step: recording a 4-3 or 4-4 session to turn `static` into `traced`.

### 6. 4-5 (re-roll, all wave kinds possible)

`static` (a plan).

The final non-boss level. 4-5 re-rolls `r % 5` after every wave: 40 % pair kind 10, 40 % pair kind 11, 20 % Bilaterus. Egg price is $99 999 (the cap). The hatch screen for 4-5 awards no pet (id 999, "Evil Alien Mastermind"): `SOUND_EVILLAFF` (0x11c) plays instead of `SOUND_HATCH`, the name "Evil Alien Mastermind" is drawn from T > 170, and a 4-line description replaces the pet text (`hatch-screen.md`).

After the hatch screen, the profile advances to tank 4 level 6, which the advance logic maps to **5-1** (`hatch-screen.md`: "if tank 4 and level == 6: tank = 5, level = 1"). Tank 4 has no bonus round; the player goes straight to the final boss.

**Record:** a 4-5 session exercising at least one Bilaterus wave and one pair wave, plus the hatch screen and its "Evil Alien Mastermind" reveal.

**Test:** `tank_4_5` replays exactly.

### 7. Boss (5-1)

`static` (a plan; all facts from `level-start.md` and `hatch-screen.md`).

Level 5-1 is the final boss fight. It is structurally a normal level with special rules:

**Level start** (`level-start.md`):
- Wave countdown set to **300** (not 3000), so the first wave comes fast.
- **All 18 pets** (ids 0..0x11) are added, regardless of chosen/owned flags.
- No starting fish (no guppies, no breeder).

**Tank 5 special rules** (from pet specs):
- Slot-81 abilities (Prego birth, Vert coins, Meryl song, Shrapnel bombs, Nostradamus potions, Amp charge) **do not run** (`board+0x3c8 == 5` early return).
- Pet chase gates skip on tank 5: Rufus, Gumbo, Gash, Wadsworth never enter their alien-chase or ability modes.
- Blip's reveal and reticle skip on tank 5.
- Clyde only wanders (no coin chase).
- Rhubarb's x-drift is skipped (but reroll still runs).
- Niko's pearl timer skips.
- Brinkley's food chase skips.

**Wave kind:** unknown from SetupShop (tank 5 not covered in the cached decompilation). Expected to be kind 8 (Bilaterus) based on the "Evil Alien Mastermind" theme, but needs verification. The SetupShop tank-5 branch may also set a different shop layout (the egg counter is forced to 3, so only 1 egg piece is needed; `shop-bar.md`). Slot 11 reveals from `Board::Update` once conditions hold, not from stage-up (`shop-bar.md`).

**Hatch screen** after 5-1: pet id **0x13** (Presto), **+5000 shells** (`hatch-screen.md`). The profile advances to 5-2, which sets `profile+0x59 = 1` (game completed) and increments `profile+0x50` (times beaten). After this, Adventure restarts with the completed-game flag and tutorial hints suppressed.

**Record:** a 5-1 session. This requires completing 4-5 first (or setting the profile to 5-1 with `set_profile.py`). The recording exercises the boss wave, all 18 pets active, and the hatch/completion flow.

**Test:** `tank_5_1_boss` replays exactly.

### 8. Open static claims and integration

`static` (a plan).

After steps 2–7, remaining `static` claims from the pet specs that mention tank-4 objects:

- **Rhubarb flings Ultravores** (`pet-rhubarb.md`): walks `board+0xd4`, centre `wx + 80` / `wy + 80`. Needs a recording with Rhubarb and an Ultravore near the floor.
- **Gumbo lure with Breeders/Ultravores** (`pet-gumbo.md`): `board+0x174` steers Ultravores (they are Fish with `+0x229 = 0`, same as Oscar). Whether it steers Breeders (not a Fish, no `+0x229`) is unknown.
- **Angie revival of Ultravores and Breeders** (`pet-angie.md`): Ultravore via `0x00544fe0(wx, wy, facing)` → 3-param ctor at `0x004f0310`; Breeder via `0x00547270(wx, wy, kind − 10, facing)` → 5-param ctor. Functions identified in Ghidra but not traced.
- **Seymour's slow-fall on Ultravore coins** (`pet-seymour.md`): whether his coin-fall rate (0.8/tick, rest 200) applies to kind-7 coins (`+0xd4` list).
- **Blip reticle for kind 8** (`pet-blip.md`): alien health bars and spawn-y pin for Bilaterus segments.
- **Shrapnel bombs vs Breeders** (`pet-shrapnel.md`): bomb kill list includes `+0xc0` Breeders (`0x004f2890(1)`). Path identified but not traced.
- **Walter punches Ultravores** (`pet-walter.md`): Ultravores (type 6) are in the punch target list but not traced with an Ultravore present.
- **Bilaterus BoxingGlove target effect** (`alien-kind8.md`): what happens to the fish hit by the glove — killed, eaten, or pushed. This is the highest-priority unknown for step 4.
- **Bilaterus head movement** (`alien-kind8.md`): the chase/steering algorithm for the two heads. Velocity constants identified (±3.5, ±4.0, 80.0 spacing) but roles not fully decoded.

These can be batched into focused recordings on 4-2/4-4 (Bilaterus-heavy levels) or a tank-4 zoo, and folded into the step that first records the relevant level.

Report back after each step.

`static` (a plan).
