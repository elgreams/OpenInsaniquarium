# Wadsworth the whale (`Sexy::FishTypePet`, pet id 9)

Wadsworth swims like a slow guppy. While any alien, Bilaterus or alien missile is in the tank he **shelters**: a global flag tells every small and medium guppy (stage 0 and 1) to stop what it is doing and home in on his position, guppies sitting on top of him are not drawn (they are "inside" him), and **no alien can target or eat a stage-0/1 guppy anywhere in the tank**, whether it has reached him yet or not. Large guppies and other fish get no protection. When the threat is gone he releases them: two bubbles, and the guppies go back to swimming. He is the pet hatched after tank 2-5 (`screens/hatch-screen.md#pet-award-table-adventure-first-play-through`).

Same class, update and fields as Itchy (`pet-itchy.md`). Only the id-9 differences are listed here. All offsets are raw `this`-relative.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`, id-9 branch
- Behaviour gate (slot 82) @ `0x004e9790`, id-9 branch `0x004e9926`..`0x004e9a5b`
- "Guppy still outside" test `0x004e7010`; Challenge refusal test `0x004e7130`; missile-list test `0x004e3ea0` (board `+0xdc` non-empty)
- Shelter globals: **`0x005e89d0`** (int, number of sheltering whales), **`0x005e89d4` / `0x005e89d8`** (int, his widget x / y)
- Readers: `Fish::Update` @ `0x004f6970` (`0x004f69a2`), `Fish::Draw` @ `0x004e4a90`, the prey filter `0x00500ff0`, the alien-missile hit `0x004fc070` (`0x004fc8b6`)
- Draw body `0x004f8610` case 9 → `0x004e4e40`

Confidence per section. `zoo_b_zorf_meryl_wadsworth` (`tank1_1.rs`) replays 1-2 with Zorf, Meryl and Wadsworth from the game's own demo (`zoo_b_01`, 10,400 ticks, three waves, 270 sheltering ticks) and compares the pets, guppies, food, coins, sounds and the app RNG every tick. Sections marked `static` (drawing, the Virtual Tank, Brinkley, Challenge) aren't covered.

## Fields used by id 9

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), plus:

| Offset | Type | Meaning |
|---|---|---|
| `+0x180` | double | divisor: **4.0** (set by the ctor after `Fish::Init`'s divisor draw) |
| `+0x220` | int | mouth timer (draw only), counts down to 0 |
| `+0x224` | int | mouth-open length on shelter start: **120** |
| `+0x250` | byte | **sheltering** flag (Ghidra `FishTypePet_data.offset_0x24`) |

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Construction

`Board::AddPet(9, -1, -1)` (`0x00544a90`): the same **9 app draws** as Itchy, in order: 2 position (`% 265 + 105`, `% 520 + 20`), 6 `Fish::Init` (facing, divisor, hunger, growth, mode, coin interval; the divisor draw is consumed and then overwritten), 1 shadow (`% 5`, type 0: every FishTypePet except ids 16 and 20). The id-9 ctor branch sets `+0x224 = 120` and divisor `+0x180 = 4.0` (`0x00596588`), with no RNG. `+0x250 = 0`. `+0x229` (the "can be sheltered" byte, `Fish::Init` sets it to 1) is cleared by the FishTypePet ctor for every pet, so pets are never sheltered or hidden.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Update (`0x004f7ad0`), id 9

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the gate (below) **always returns false** for id 9, so the swim modes always run; he never chases anything. On tank 5 (`board+0x3c8 == 5`) the gate isn't called at all, so he never shelters there.
- **Step 6:** slot 81 has no id-9 branch.
- Everything else (re-roll `r % 10` / `r % 9 + 1` every 21 ticks, drift, clamps, Itchy's default animation) is unchanged, with divisor 4.0: half a guppy's speed.

RNG: the re-roll draws only (app MTRand). The release bubbles use the global stream (below).

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## The gate (slot 82, `0x004e9790`), id 9

Each call, in order:

1. **Mouth timer** `+0x220`: if > 0, decrement. Else, if sheltering and `0x004e7010` is true, set it to **100**.
   - `0x004e7010`: false if the shelter count `0x005e89d0` > 1. Otherwise true if some guppy (`board+0xa0`, list order) with stage `+0x1a0 < 2` has **both** `|(x + 40) − (g.wx + 40)| > 10` **and** `|(y + 40) − (g.wy + 40)| > 10` (x, y his doubles; g.wx/wy the guppy's widget ints). (Both axes must be off: a guppy level with him but 50 px above doesn't count. Probably meant as "either"; it only affects the mouth animation.)
2. **Threat test.** "Threat" = an alien (`board+0xb8`) or Bilaterus (`+0xf4`) exists (`0x004da780`), **or** an alien missile exists (`board+0xdc`, `0x004e3ea0`).
   - **Challenge refusal:** with a threat, if the app mode is Challenge (`app+0x87c == 4`), more than one wave has spawned on the level (`board+0x45c > 1`), and `0x004e7130` is true, he treats it as no threat. `0x004e7130` is true when the tank holds no Oscars (`+0xa4`), Penta/starcatchers (`+0xd0`), Grubbers (`+0xc4`), Gekkos (`+0xcc`), Breeders (`+0xc0`) or Ultravores (`+0xd4`), and no guppy with stage > 1. I.e. in Challenge, from the second wave on, he won't protect a tank that has nothing else for the aliens to eat.
3. **No threat (or refused):** if sheltering: clear `+0x250`; `0x005e89d0 −= 1` (floored at 0); `+0x220 = 30`; two bubbles `Board::AddBubble` (`0x00538ac0`) at (wx + 11, wy + 5), then (wx + 4, wy + 2). Each bubble (only while there are fewer than 30) costs 4–5 **global** `grand()` draws (`alien-sylvester.md`, bubble). Return false.
4. **Threat:** if not sheltering: `0x005e89d0 += 1`, `+0x250 = 1`, `+0x220 = +0x224` (120). Then, **every threat tick**: `0x005e89d4 = wx`, `0x005e89d8 = wy` (his widget position, i.e. from his previous move). Return false.

So the count is 1 from his first update with an alien in the tank until his first update after the last alien, Bilaterus and missile are gone. Since the `+0xb4` pets update after the guppies and aliens (`board.md`), guppies and aliens see the new count from the next tick, and the published position lags his real one by one tick.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Effects on guppies

### `Fish::Update` (`0x004f6970`, at `0x004f69a2`)

Right after `PreUpdate`, if `0x005e89d0 ≠ 0`, the fish's `+0x229` byte is set (guppies; carnivores and pets clear it) and stage `+0x1a0 < 2`, the fish is **sheltered** this tick instead of running hunger and swim modes:

1. If the push timer `+0x20c` is 0: `vx = +0.5` if vx ≥ 0, else `−0.5`.
2. If `+0x208` is 0: `vy = 0`.
3. With `wx, wy` = the published whale position (ints) and `x, y` the guppy's doubles (both are 80 × 80 widget top-lefts, so this lines the guppy up on the whale):
   - x: `x > wx + 50` → `x −= 5`; else `x < wx − 50` → `x += 5`; else `x > wx + 5` → `x −= 3`; else `x < wx − 5` → `x += 3`.
   - y: the same with **4** for the far step: `y > wy + 50` → `y −= 4`; `y < wy − 50` → `y += 4`; `y > wy + 5` → `y −= 3`; `y < wy − 5` → `y += 3`.
4. Skip `Fish::UpdateHunger` (`0x004f15c0`, `fish-guppy.md#hunger`) and the swim modes, and continue with the rest of the update: re-roll (`r % 10`, `r % 9 + 1`: **the RNG cost is unchanged**), coin drop (only without aliens), damping timers, vx-bucket drift (never matches: vx is ±0.5), clamps, edge steer, animation, and `x += vx / div`, `y += vy / div`.

So a sheltered guppy jumps up to 5 px per tick toward him, regardless of its own speed, and doesn't chase food. (Hunger doesn't drop while aliens are present anyway; the shelter only adds the tick after the last alien leaves and before his next update, when the count is still set.)

`traced` (`zoo_b_zorf_meryl_wadsworth`).

### `Fish::Draw` (`0x004e4a90`)

If `0x005e89d0 ≠ 0`, `wx − 8 < x < wx + 8`, `wy − 8 < y < wy + 8` (strict, doubles against the published ints), `+0x229` set and stage < 2: the fish **is not drawn**, and its shadow's scale (`shadow+0x168`) is set to 0, hiding the shadow too. Otherwise the shadow scale is set back to 1.0 and the fish draws normally. So guppies vanish once they are within 8 px of his position, and reappear as they drift out.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Effects on aliens

The shared prey filter **`0x00500ff0`** returns "protected" for an object of type 0 (guppy) when `0x005e89d0 ≠ 0` and its stage `+0x1a0 < 2`. It is called by:

- Sylvester's nearest-prey search `0x004e8b60` and eat test `0x004e8f70` (`alien-sylvester.md#31-nearest-prey-0x004e8b60`: protected guppies are not candidates), and by `0x004e95f0`, `0x004e9470`, `0x004e8df0` (the other alien kinds' target/eat routines; not followed).
- The alien missile's guppy hit test `0x004fc070` (at `0x004fc8b6`) skips stage-0/1 guppies while the count is non-zero.

The protection depends only on the global count, **not** on the guppy's distance to Wadsworth. Aliens then pick from what's left (large guppies, carnivores and other fish, or pets while a boss is present); with nothing left they have no prey.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Count bookkeeping

- `Board::InitLevel` (`0x00541a20`, at `0x00541b18`) and the reset routines `0x00537e20`, `0x0053c3f0` set `0x005e89d0 = 0`.
- `0x004d6250` (FishTypePet destructor and slot 74, removal): if the removed pet is id 9 and sheltering, `count −= 1` (floored at 0). Selling or losing him mid-wave releases the guppies.
- Save/load `0x00546240` serialises `0x005e89d0`, `d4`, `d8`; after loading it zeroes the count if it is negative, or positive with no alien and no Bilaterus.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Drawing (`0x004e4e40`)

`IMAGE_WADSWORTH` (`0x005e8b30`, `resources.xml`: 10 × 3 cels of 80 × 80), source `(cel · 80, row · 80)`, mirrored per `Fish::Draw`'s guppy rule unless turning:

| when (first match) | row | cel | mirror |
|---|---|---|---|
| mouth timer `+0x220 > 0` | 2 | see below | guppy rule |
| turning (`+0x1c4 ≠ 0`) | 1 | swim cel `+0x1c0` | iff turn > 0 |
| otherwise | 0 | swim cel `+0x1c0` | guppy rule |

Mouth cels for timer t (C integer division):
- sheltering and `t > +0x224 − 10` (111..120): `9 − ((120 − t) − 1) / 2` (9 down to 5): opening;
- not sheltering and `t > 20` (21..30): `9 − (29 − t) / 2` (9 down to 5);
- otherwise `t < 11`: `(t − 1) / 2` (4 down to 0): closing; else 5 (held open).

So on shelter start the mouth opens and stays open (cel 5) while the 100-tick refresh sees guppies outside, then closes; on release it opens for 30 ticks to let them out.

Then, if **not** sheltering while a threat exists (alien, Bilaterus or missile) and the tank isn't 5: `IMAGE_ZZZ` (`0x005e8b44`) at (40, −15). This only happens under the Challenge refusal (or in the first frame of a wave before his update). A type-0 shadow follows him.

`static`.

## Summary / open questions

- **9 app draws** on AddPet (as Itchy); then only the 21-tick re-roll. Release costs 2 bubbles on the **global** stream.
- While count ≠ 0: every stage-0/1 guppy homes on him (±5/±4 far, ±3 near, 5-px deadzone), skips hunger and swim modes, is hidden within 8 px, and is immune to aliens and missiles everywhere.
- Open: the `0x004e7010` both-axes test (intended "either"?); which alien kinds use `0x004e95f0`/`0x004e9470`/`0x004e8df0`; first capture should be tank 3-1 with a wave.
