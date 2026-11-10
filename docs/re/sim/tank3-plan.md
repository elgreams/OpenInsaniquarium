# Plan: tank 3 (levels 3-1 to 3-5)

Status: plan with `static` values read from the binary (SetupShop @ `0x00541200`, OnStageUp @ `0x004f1840`, InitLevel @ `0x00541a20`). Recording/tracing still needed.

## What tank 3 adds

From `SetupShop` (`0x00541200`, `sim/shop-bar.md#shop-setup`):

- **Shop:** guppy, food quality, food quantity, **Guppycruncher** (slot 7, $750), **Beetlemuncher** (slot 8, $2000), weapon ($2000), egg piece. No carnivore, no star potion, no Starcatcher. On 3-1 the Beetlemuncher and weapon slots are unmapped (buying a Guppycruncher on 3-1 reveals slot 11 egg; on 3-2+ it reveals slot 8 Beetlemuncher; buying a Beetlemuncher reveals slots 10 weapon and 11 egg; `shop-bar.md#per-slot-behaviour`).
- **Egg prices:** `static` (read from SetupShop @ `0x00541200`). 1000 / 5000 / 7500 / 10000 / 15000 by level.
- **Wave kinds `+0x2bc`:** `static` (read from SetupShop). 3-2 and 3-5 each make **one app-`MTRand` draw**; the others set it directly. **Kinds 6 and 7 are new.** Per the alien ctor table (`alien-sylvester.md`): kind 6 divisor 3.5, 220 HP, y forced to 280; kind 7 divisor 0.5, 260 HP. Kind 6 fires **kind-1 missiles** (`alien-kind5.md#shared-with-kind-1-tank-3-alien-kind-6`).

  | Level | Egg price | Wave kind `+0x2bc` | Slots unmapped | RNG |
  |---|---|---|---|---|
  | 3-1 | 1000 | 3 (Balrog) | 8, 10 | – |
  | 3-2 | 5000 | `(r & 1) \| 4`: 4 or 5 | – | 1 app draw |
  | 3-3 | 7500 | 7 | – | – |
  | 3-4 | 10000 | 6 | – | – |
  | 3-5 | 15000 | `(r & 1) \| 6`: 6 or 7 | – | 1 app draw |

- **Initial countdown `+0x2c0`:** 3000 (InitLevel default @ `0x00541a20`; SetupShop does not override it for any tank-3 level). `static`.
- **Stage-up reveals:** `static` (read from OnStageUp @ `0x004f1840` assembly). When a guppy reaches stage 2 (large), reveals slots **2** (food quality), **3** (food quantity), **7** (guppycruncher) with pop animation. No messages. Then the purchase chains above (buy guppycruncher → reveal 8 on 3-2+, or 11 on 3-1; buy beetlemuncher → reveal 10, 11).
- **New objects:**
  - **Guppycruncher** (`Sexy::Grubber`, type `0x09`, list `+0xc4`): bought fish. Eats small guppies like the Carnivore, but instead of dropping diamonds it drops **Beetles** (Larvae). Spec needed: `objects/guppycruncher.md`.
  - **Beetlemuncher** (`Sexy::Gekko`, type `0x07`, list `+0xcc`): bought fish. Eats the Beetles dropped by the Guppycruncher and drops coins. Shares slot 82 (`0x004e1550`) with Oscar (`carnivore.md`). Spec needed: `objects/beetlemuncher.md`.
  - **Beetle / Larva** (`Sexy::Larva`, type `0x1d`, list `+0xc8`): the intermediate currency dropped by the Guppycruncher and eaten by the Beetlemuncher. Not a Fish (inherits directly from GameObject; see `board.md#class-hierarchy`). Spec needed: `objects/larva.md`.
  - **Alien kind 6**: floor walker like the Destructor (y = 280), fires kind-1 missiles. The missiles are **redirectable by clicks** (click changes their heading instead of destroying them), have an additive glow draw, play `0x11f` on removal, get removed when out of bounds, lose the clamp once redirected, and their hit path checks Wadsworth's shelter (`alien-kind5.md#8`). Firing uses `PickTarget(0)`/`(1)`, sample `0x143`, launch points from offsets `+0x46`/`+0x0a` (`pet-stanley.md#open-questions`). Arrival sound: `0x134` ROAR2 (`alien-kind5.md#2`). Spec needed: `objects/alien-kind6.md`.
  - **Alien kind 7**: divisor 0.5 (very fast), 260 HP. Has a state byte at `+0x204` that Itchy, Gash and Stanley skip (`pet-itchy.md#target`, `pet-gash.md#alien-chase`, `pet-stanley.md#target`). Presumably this is a "submerging" or "hiding" state. Almost nothing else is known. Spec needed: `objects/alien-kind7.md`.
  - **Kind-1 missiles** (`Missle` kind 1): fired by alien kind 6. Spec needed in `objects/alien-kind6.md` or a shared missile spec.
- **Backdrop** index 4 (`FUN_00538a10(4)` in SetupShop; `+0x3e8`), tank-3 music (module 2, order 32; `assets/music.md`).
- **Pets hatched** (from `hatch-screen.md#pet-award-table`): 3-1 Seymour (10), 3-2 Shrapnel (11), 3-3 Gumbo (12), 3-4 Blip (13), 3-5 Rhubarb (14).
- **Pair wave kinds** that may appear on 3-5: with `(r & 1) | 6` → kind 6 or 7. Kind 6 pairs to 11 (5+6); kind 7 pairs to 10 (7+3) (`alien-sylvester.md#per-tick-countdown`).

`static` for the list itself (wave kinds, egg prices, reveals all read from the binary). Fish class specs are TODO.

## Prerequisites

- **Tank 2 complete** (all 5 steps done, `traced`). The sim already handles: star guppies, Starcatcher, alien kinds 4 and 5, kind-0 missiles, Stanley's shoot-down, the Sneeze of Power, and the 2-5 re-roll.
- **Bonus screen**: done (`traced`). The 2-5 → bonus → tank 3-1 transition is the same code path as 1-5 → bonus → 2-1. The bonus screen after tank 2 sells pets 20–23 and the two upgrades; the exit flow (Continue → pet screen → 3-1) is the same.

## Steps

`static` — same workflow as tank 2: spec from the binary → build → record the original → replay and fix until exact.

### 1. Ghidra read: tank 3 SetupShop and the new fish classes

**SetupShop and OnStageUp: done** (`static`, read from cached decompilation).

- SetupShop tank-3 branch: wave kinds, egg prices, re-roll formulas, initial countdown all recorded above.
- OnStageUp tank-3 branch: reveals slots 2, 3, 7 with pop animation, no messages.

**Economy chain done** (`confirmed`, `t3-ghidra-b-ecosystem`):

- **Grubber** (`guppycruncher.md`): spawn, ctor (5 draws), Init fields, behaviour (colour fade, hunger countdown, starving check at ≤ 0, chase at < 900), Larva drop timer. All `confirmed`.
- **Gekko** (`beetlemuncher.md`): spawn (12 draws), ctor, Init fields (type 7, stage 7, coin interval), Fish::Update overrides (hunger, prey chase, Larva bite, coin drop kind 6). All `confirmed`.
- **Larva** (`larva.md`): ctor (1 phantom draw), AddBeetle (no shadow/splash), Update (rise at 1.0/tick, removal at y < 64, click sets caught flag → $150 on exit, mouse-visible at y < 320), Draw (10 cels, 5 cols). All `confirmed`.

**Still TODO** (aliens):

- **Alien kind 6 firing** (`0x004fdd34`): `PickTarget(0)`/`(1)` meaning, the launch positions, the volley size, the interval.
- **Kind-1 missile** (`Missle::Update` kind-1 branch, click at `0x004d82a0` kind-1 branch): redirect mechanics, out-of-bounds rule, Wadsworth hit check.
- **Alien kind 7** (`Alien::Update` kind-7 branches, `+0x204` logic): what it does, its chase/wander pattern, its special action timer `+0x1ac`.

Write specs: `objects/alien-kind6.md` (expand from existing `static` draft), `objects/alien-kind7.md`. Existing economy specs (`guppycruncher.md`, `beetlemuncher.md`, `larva.md`) are `confirmed`.

### 2. 3-1: Guppycruncher economy

`traced` (`tank_3_1`).

The first tank 3 level. Shop has guppy ($100), food quality ($200), food quantity ($300), Guppycruncher ($750), and egg piece ($1000). Slots 8 (beetlemuncher) and 10 (weapon) are unmapped. Wave kind 3 (Balrog), countdown 3000. No Guppycruncher bought on this recording (no new objects exercised; that's step 3). The recording exercises the tank-3 shop setup, wave table (kind 3, countdown 3000), stage-up reveals (slots 2, 3, 7 on first large guppy), and a Balrog wave.

**Test:** `tank_3_1` replays `tank3-1_full_01` exactly (Stinky, Niko, Itchy; money pushed to 1500, countdown to 400).

### 3. Beetlemuncher (Gekko) and the full economy chain

`static` (a plan).

3-2 introduces the Beetlemuncher. Buying a Guppycruncher now reveals slot 8 (Beetlemuncher). The economy chain is: guppy → food → growth → Guppycruncher eats small guppy → drops Larva → Beetlemuncher eats Larva → drops coin.

**Record:** a full 3-2 (or a focused zoo on 3-2) that exercises: buying both the Guppycruncher and the Beetlemuncher, the Beetlemuncher eating a Larva, its coin drop, the reveal chain (buy Guppycruncher → reveal 8, buy Beetlemuncher → reveal 10/11), and the 3-2 wave (random kind, 1 draw).

**Build:** Gekko, the slot-8 purchase and reveal chain.

**Test:** `tank_3_2` (or `tank_3_2_beetlemuncher`) replays exactly.

### 4. Alien kind 6 and kind-1 missiles

`traced` (`tank_3_4`).

Kind 6 is a floor walker (y = 280) that fires kind-1 missiles. Kind-1 missiles are **redirectable**: a click changes their heading instead of destroying them, so they can be bounced away from fish. They are removed when they leave the tank bounds. This is the main new combat mechanic on tank 3.

**Test:** `tank_3_4` replays `tank3-4_full_01` exactly. Exercised: kind-6 spawn, firing (volleys of 2), pre-fire animation (interval−15, ping-pong cel), vx manipulation (cel%5), missile update (no smoke trail — corrected from the static claim), death with kind-1 missile retention (`kill_alien` only removes kind-2), and the `focus_paused` fix (Release no longer blindly unpauses).

**Not yet exercised:** click-to-redirect, Wadsworth shelter check, Stanley vs kind-1 missile. These remain `static` in `alien-kind6.md`.

### 5. Alien kind 7

`static` (a plan).

Kind 7 is very fast (divisor 0.5, cruise ~3.6 px/tick) with 260 HP. It has a state byte `+0x204` that causes Itchy, Gash and Stanley to skip it as a target. This likely means it has a "hiding" or "submerging" phase. The special-action timer `+0x1ac` is used by kinds 5/6/7 — for kind 7 it may control this state.

**Depends on:** step 1 (the Ghidra read of kind-7 behaviour).

**Record:** a level where kind 7 appears. Capture must exercise: the spawn (ROAR3, `0x135`), the chase, the `+0x204` state transition (pets skip it), the eat (if it eats), and the laser kill.

**Build:** the kind-7 branches in Alien::Update, the `+0x204` transitions, any kind-7-specific death or eating behaviour.

**Test:** `alien_kind7` replays exactly.

### 6. 3-5 (random wave kind, possibly a pair)

`traced` (`tank_3_5`).

The final tank 3 level. 3-5 makes one app draw to pick the wave kind (`r & 1 | 6`, got 6). The recording exercises the full level with Stinky and Niko as pets, including pearl collection during alien presence (the pearl widget is checked before the shoot path). The fix: the board's click handler now checks `self.pearls` inside the alien-present branch, before regular coins, matching the original's front-most-widget z-order.

**Test:** `tank_3_5` replays `tank3-5_full_01` exactly (5378 snapshots, 95 pearl ticks).

### 7. Open static claims and integration

After steps 2–6, remaining `static` claims from the pet specs that mention tank-3 objects:

- **Rhubarb flings** Gekkos and Ultravores (`pet-rhubarb.md`): the snap box checks `board+0xcc` (Gekkos). Needs a tank 3+ recording with Rhubarb chosen and a Beetlemuncher near the floor.
- **Shrapnel bombs** kill Grubbers and Breeders (`pet-shrapnel.md`): `+0xc4` Grubbers → `0x004f2890(1)`. Needs a recording with Shrapnel and a Guppycruncher.
- ~~**Blip reticle** (`pet-blip.md`): first capture should be tank 3-5+ with Blip, checking the shop at tick 201 and the alien spawn y (280).~~ Done: `tank_3_blip` replays 3-4 with Blip; shop reveal and spawn y 280 both match.
- **Wadsworth shelter vs. kind-1 missiles**: the hit check on kind-1 missiles tests shelter, unlike kind-0. A recording with Wadsworth on a kind-6 level would confirm. (`tank_3_wadsworth` is on 3-1 with a kind-3 alien, so kind-1 missiles are not exercised.)
- **Gumbo lure** with Gekkos/Grubbers: whether `board+0x174` affects fish types beyond guppies.
- **Angie revival** of Gekkos, Grubbers, Breeders (`pet-angie.md`): the re-creation functions at `0x005451c0` (Gekko), `0x005454f0` (Grubber) are listed but not traced.

These can be batched into one or two focused recordings on 3-5 (or a zoo) and folded into the step that first records that level.

Report back after each step.

`static` (a plan).
