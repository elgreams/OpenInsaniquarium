# Plan: tank 2 (levels 2-1 to 2-5)

Status: plan only. Facts cited below are `static` unless a spec says otherwise.

## What tank 2 adds

From `SetupShop` (`0x00541200`, `sim/shop-bar.md#shop-setup`):

- **Shop:** guppy, food quality, food quantity, **star potion** (slot 5, $250), **Starcatcher** (slot 6, $750), weapon, egg piece. No carnivore. On 2-1 the Starcatcher and weapon slots are unmapped.
- **Egg prices:** 750, 3000, 5000, 7500, 10000.
- **Wave kinds `+0x2bc`:** 2-1 → 2, 2-2 → 3, 2-3 → 4, 2-4 → 5, 2-5 → 4 or 5 (one app draw). Kinds 2 and 3 are the tank-1 aliens already built (1-3, 1-4: Sylvester variants and Balrog). **Kinds 4 and 5 are new.** Per the alien ctor table (`alien-sylvester.md`): kind 4 divisor 1.6, 100 HP; kind 5 divisor 1.2, 150 HP. The click rule already treats kind 4 specially (a click with kind 4 first in the list may drop food, `alien-sylvester.md#4.1`: the alien lasers can't hurt, the "new breed" of 2-3). Kind 5 fires missiles (kind 0, `pet-stanley.md#open-questions`).
- **New objects:**
  - **Starcatcher** (`Sexy::Penta`, type 0x08, list `+0xd0`): bought fish; eats stars (coin kinds 3/10) and drops a kind-5 coin ($200); specced in `objects/starcatcher.md` (static).
  - **Star potion** (food level 3, `food.md`: a guppy that eats it goes to stage 3 if medium/large, a small one explodes; level-3 food explodes at the bottom; RNG not modelled yet).
  - Stars: stage-3 guppies are star guppies and drop kind-3 stars ($40, falling; `static`).
  - Alien kind 4 and 5 behaviour, kind 5's missiles (Missle kinds 0/1, `+0xdc`), which also makes Stanley's shoot-down testable.
- **Backdrop** 2 (`+0x3e8`), the "Welcome to the second tank!" message (already in `level-start.md`), tank-2 music.

`static` for the list itself; each item's confidence is in its own spec (Starcatcher: `objects/starcatcher.md`).

## Steps

Same workflow as tank 1 and the pets: spec from the binary → build → record the original playing the level (its own `-record` demo + a CE capture) → replay and fix until exact.

1. **2-1**: done. `tank_2_1` replays `tank2-1_full_01` exactly (the whole level to the 4th egg: shop, reveals, two waves, two star potions exploding at the bottom). A guppy eating a star potion (star guppy / explosion) wasn't captured: record it with the Starcatcher (2-2), which needs star guppies anyway.
2. **Starcatcher**: done. `tank_2_2_star_guppies` (`tank2-2_full_01`: star guppies from star potions) and `tank_2_2_starcatcher` (`tank2-2_full_02`: a Starcatcher eating stars, then eaten by the alien) replay exactly. Starvation isn't captured yet.
3. **Alien kind 4 (Gus)**: done. `gus_feeds` and `gus_killed` (`tank2-3_gus_01/02`) replay exactly: he eats pellets and guppies, bursts with no corpse. Star potions and higher food levels aren't captured.
4. **Alien kind 5 (Destructor) and its missiles**: done. `destructor_missiles` (`tank2-4_destructor_01`) replays exactly: volleys, missiles hitting and clicked, the kill with a missile still flying. Stanley's shoot-down: see below.
5. **2-5** (kind 4 or 5 by a draw) and the hatch/pet screens between tank-2 levels: record a full level.

Report back after each step.

`static` (a plan).
