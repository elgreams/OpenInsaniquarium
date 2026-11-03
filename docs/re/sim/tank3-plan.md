# Plan: tank 3 (levels 3-1 to 3-5)

Status: plan only. Facts cited below are `static` unless a spec says otherwise. Wave-kind assignments and egg prices need Ghidra verification.

## What tank 3 adds

From `SetupShop` (`0x00541200`, `sim/shop-bar.md#shop-setup`):

- **Shop:** guppy, food quality, food quantity, **Guppycruncher** (slot 7, $750), **Beetlemuncher** (slot 8, $2000), weapon ($2000), egg piece. No carnivore, no star potion, no Starcatcher. On 3-1 the Beetlemuncher and weapon slots are unmapped (buying a Guppycruncher on 3-1 reveals slot 11 egg; on 3-2+ it reveals slot 8 Beetlemuncher; buying a Beetlemuncher reveals slots 10 weapon and 11 egg; `shop-bar.md#per-slot-behaviour`).
- **Egg prices:** not yet read from the binary. Tank 2 was 750/3000/5000/7500/10000; tank 3 will differ.
- **Wave kinds `+0x2bc`:** 3-2 and 3-5 each make **one app-`MTRand` draw** to set the wave kind (`shop-bar.md#level-init-and-per-level-layout`); the other levels set it directly. Exact values need a Ghidra read of the tank-3 branch at `0x00541200`. **Kinds 6 and 7 are new.** Per the alien ctor table (`alien-sylvester.md`): kind 6 divisor 3.5, 220 HP, y forced to 280; kind 7 divisor 0.5, 260 HP. Kind 6 fires **kind-1 missiles** (`alien-kind5.md#shared-with-kind-1-tank-3-alien-kind-6`).
- **Stage-up reveals:** the `OnStageUp` branch for tank 3 reveals three slots (not followed; `shop-bar.md#reveal`). By the shop layout and purchase chains, likely 2, 3 and 7 (the initial economy), then the purchase chains above.
- **New objects:**
  - **Guppycruncher** (`Sexy::Grubber`, type `0x09`, list `+0xc4`): bought fish. Eats small guppies like the Carnivore, but instead of dropping diamonds it drops **Beetles** (Larvae). Spec needed: `objects/guppycruncher.md`.
  - **Beetlemuncher** (`Sexy::Gekko`, type `0x07`, list `+0xcc`): bought fish. Eats the Beetles dropped by the Guppycruncher and drops coins. Shares slot 82 (`0x004e1550`) with Oscar (`carnivore.md`). Spec needed: `objects/beetlemuncher.md`.
  - **Beetle / Larva** (`Sexy::Larva`, type `0x1d`, list `+0xc8`): the intermediate currency dropped by the Guppycruncher and eaten by the Beetlemuncher. Not a Fish (inherits directly from GameObject; see `board.md#class-hierarchy`). Spec needed: `objects/larva.md`.
  - **Alien kind 6**: floor walker like the Destructor (y = 280), fires kind-1 missiles. The missiles are **redirectable by clicks** (click changes their heading instead of destroying them), have an additive glow draw, play `0x11f` on removal, get removed when out of bounds, lose the clamp once redirected, and their hit path checks Wadsworth's shelter (`alien-kind5.md#8`). Firing uses `PickTarget(0)`/`(1)`, sample `0x143`, launch points from offsets `+0x46`/`+0x0a` (`pet-stanley.md#open-questions`). Arrival sound: `0x134` ROAR2 (`alien-kind5.md#2`). Spec needed: `objects/alien-kind6.md`.
  - **Alien kind 7**: divisor 0.5 (very fast), 260 HP. Has a state byte at `+0x204` that Itchy, Gash and Stanley skip (`pet-itchy.md#target`, `pet-gash.md#alien-chase`, `pet-stanley.md#target`). Presumably this is a "submerging" or "hiding" state. Almost nothing else is known. Spec needed: `objects/alien-kind7.md`.
  - **Kind-1 missiles** (`Missle` kind 1): fired by alien kind 6. Spec needed in `objects/alien-kind6.md` or a shared missile spec.
- **Backdrop** 3 (`+0x3e8`), tank-3 music (module 2, order 32; `assets/music.md`).
- **Pets hatched** (from `hatch-screen.md#pet-award-table`): 3-1 Seymour (10), 3-2 Shrapnel (11), 3-3 Gumbo (12), 3-4 Blip (13), 3-5 Rhubarb (14).
- **Pair wave kinds** that may appear on 3-5: kind 10 (7+3), kind 11 (5+6), kind 12 (3+8) (`alien-sylvester.md#per-tick-countdown`). Which ones tank 3 actually uses depends on the SetupShop re-roll formula.

`static` for the list itself; Ghidra reads needed for wave kinds, egg prices, reveal slots. Fish class specs are TODO.

## Prerequisites

- **Tank 2 complete** (all 5 steps done, `traced`). The sim already handles: star guppies, Starcatcher, alien kinds 4 and 5, kind-0 missiles, Stanley's shoot-down, the Sneeze of Power, and the 2-5 re-roll.
- **Bonus screen**: done (`traced`). The 2-5 → bonus → tank 3-1 transition is the same code path as 1-5 → bonus → 2-1. The bonus screen after tank 2 sells pets 20–23 and the two upgrades; the exit flow (Continue → pet screen → 3-1) is the same.

## Steps

Same workflow as tank 2: spec from the binary → build → record the original → replay and fix until exact.

### 1. Ghidra read: tank 3 SetupShop and the new fish classes

Before any recording, read the binary for the facts the plan needs:

- **SetupShop tank-3 branch** (`0x00541200`): wave kind per level, egg prices, initial countdown, the 3-2 and 3-5 re-roll formulas. Update this plan with the values.
- **`OnStageUp` tank-3 branch** (`0x004f1840`): which three slots are revealed, and any messages.
- **Grubber** (`0x00545430` spawn, `Grubber` ctor, `Grubber::Update`): what it eats, how it drops Larvae, its ctor draws, hunger, coin-like economy.
- **Gekko** (`0x00545080` spawn, `Gekko` ctor, `Gekko::Update`, slot 82 `0x004e1550`): what it eats (Larvae), how it drops coins, its ctor draws.
- **Larva** (`Larva` ctor at the `0x1d` code path, `Larva::Update`): drop behaviour, lifetime, floor crawl, how the Beetlemuncher catches it.
- **Alien kind 6 firing** (`0x004fdd34`): `PickTarget(0)`/`(1)` meaning, the launch positions, the volley size, the interval.
- **Kind-1 missile** (`Missle::Update` kind-1 branch, click at `0x004d82a0` kind-1 branch): redirect mechanics, out-of-bounds rule, Wadsworth hit check.
- **Alien kind 7** (`Alien::Update` kind-7 branches, `+0x204` logic): what it does, its chase/wander pattern, its special action timer `+0x1ac`.

Write specs: `objects/guppycruncher.md`, `objects/beetlemuncher.md`, `objects/larva.md`, `objects/alien-kind6.md`, `objects/alien-kind7.md`. All initially `static`.

### 2. 3-1: Guppycruncher economy

The first tank 3 level. Shop has guppy, food quality, food quantity, Guppycruncher, and egg piece. The wave kind is from the tank-2 pool (4 or 5, per the progression pattern; verify in step 1). The Guppycruncher + Larva are the new objects to build.

**Record:** a full 3-1 (set profile to 3-1, own all 24 pets, pick 3 pets, push money/countdown). Capture must exercise: buying a Guppycruncher (spawn draws, GROW, PlaySplash, reveal 11), the Guppycruncher eating a small guppy (chomp, Larva dropped), a Larva crawling and being collected or expiring, the alien wave (the kind from step 1), and the egg completion.

**Build:** Grubber, Larva, the tank-3 shop layout, stage-up reveals, backdrop 3, tank-3 music.

**Test:** `tank_3_1` replays `tank3-1_full_01` exactly.

### 3. Beetlemuncher (Gekko) and the full economy chain

3-2 introduces the Beetlemuncher. Buying a Guppycruncher now reveals slot 8 (Beetlemuncher). The economy chain is: guppy → food → growth → Guppycruncher eats small guppy → drops Larva → Beetlemuncher eats Larva → drops coin.

**Record:** a full 3-2 (or a focused zoo on 3-2) that exercises: buying both the Guppycruncher and the Beetlemuncher, the Beetlemuncher eating a Larva, its coin drop, the reveal chain (buy Guppycruncher → reveal 8, buy Beetlemuncher → reveal 10/11), and the 3-2 wave (random kind, 1 draw).

**Build:** Gekko, the slot-8 purchase and reveal chain.

**Test:** `tank_3_2` (or `tank_3_2_beetlemuncher`) replays exactly.

### 4. Alien kind 6 and kind-1 missiles

Kind 6 is a floor walker (y = 280) that fires kind-1 missiles. Kind-1 missiles are **redirectable**: a click changes their heading instead of destroying them, so they can be bounced away from fish. They are removed when they leave the tank bounds. This is the main new combat mechanic on tank 3.

**Depends on:** step 1 (the Ghidra read of kind 6 firing and kind-1 missiles).

**Record:** a level where kind 6 appears (from step 1's wave-kind table). Capture must exercise: the kind-6 spawn (ROAR2, warp, y = 280), a volley of kind-1 missiles, clicking a missile (redirect, not destroy), a redirected missile leaving the tank bounds, a missile hitting a fish, the laser kill (damage `2·laser + 2` as kind 5, sideways-only knockback), Stanley shooting a kind-1 missile, and the death (diamond, floor corpse).

**Build:** alien kind 6 (differences from kind 5 in the firing and the arrival sound), kind-1 missile (redirect on click, additive glow draw, `0x11f` on removal, out-of-bounds removal, Wadsworth shelter check on hit).

**Test:** `alien_kind6_missiles` replays exactly.

### 5. Alien kind 7

Kind 7 is very fast (divisor 0.5, cruise ~3.6 px/tick) with 260 HP. It has a state byte `+0x204` that causes Itchy, Gash and Stanley to skip it as a target. This likely means it has a "hiding" or "submerging" phase. The special-action timer `+0x1ac` is used by kinds 5/6/7 — for kind 7 it may control this state.

**Depends on:** step 1 (the Ghidra read of kind-7 behaviour).

**Record:** a level where kind 7 appears. Capture must exercise: the spawn (ROAR3, `0x135`), the chase, the `+0x204` state transition (pets skip it), the eat (if it eats), and the laser kill.

**Build:** the kind-7 branches in Alien::Update, the `+0x204` transitions, any kind-7-specific death or eating behaviour.

**Test:** `alien_kind7` replays exactly.

### 6. 3-5 (random wave kind, possibly a pair)

The final tank 3 level. 3-5 makes one app draw to pick the wave kind. Tank 1-5 re-rolls to kind 9 (pair 1+3); tank 3-5 may use a pair kind (10 = 7+3, 11 = 5+6, 12 = 3+8). The re-roll after each spawn may also draw.

**Record:** a full 3-5. Capture must exercise: the setup draw, the wave (whichever kind it rolls), and any re-roll.

**Build:** any new re-roll formula or pair wave handling.

**Test:** `tank_3_5` replays exactly. If the draw is random, a second recording with the other outcome would confirm coverage.

### 7. Open static claims and integration

After steps 2–6, remaining `static` claims from the pet specs that mention tank-3 objects:

- **Rhubarb flings** Gekkos and Ultravores (`pet-rhubarb.md`): the snap box checks `board+0xcc` (Gekkos). Needs a tank 3+ recording with Rhubarb chosen and a Beetlemuncher near the floor.
- **Shrapnel bombs** kill Grubbers and Breeders (`pet-shrapnel.md`): `+0xc4` Grubbers → `0x004f2890(1)`. Needs a recording with Shrapnel and a Guppycruncher.
- **Blip reticle** (`pet-blip.md`): first capture should be tank 3-5+ with Blip, checking the shop at tick 201 and the alien spawn y (280).
- **Wadsworth shelter vs. kind-1 missiles**: the hit check on kind-1 missiles tests shelter, unlike kind-0. A recording with Wadsworth on a kind-6 level would confirm.
- **Gumbo lure** with Gekkos/Grubbers: whether `board+0x174` affects fish types beyond guppies.
- **Angie revival** of Gekkos, Grubbers, Breeders (`pet-angie.md`): the re-creation functions at `0x005451c0` (Gekko), `0x005454f0` (Grubber) are listed but not traced.

These can be batched into one or two focused recordings on 3-5 (or a zoo) and folded into the step that first records that level.

Report back after each step.

`static` (a plan).
