# Itchy the swordfish (`Sexy::FishTypePet`, pet id 2)

Itchy swims like a guppy while the tank is calm. When an alien is in, he charges it and stabs it: 1 HP per tick while his nose overlaps the alien's centre. He never eats, never drops coins and doesn't count as a fish for the empty-tank check. He is the pet hatched after tank 1-3 and first appears on 1-4.

Class: `Sexy::FishTypePet` (type code `0x15`), vtable `0x00597ec4`, object size `0x268`, list `board+0xb4`. The class also implements pets 3, 4, 6, 8–13, 15–23; only id 2 is described here. It derives from `Sexy::Fish`, so the `+0x158..+0x228` fields are the guppy's (`fish-guppy.md#fields`).

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`
- Update (slot 22) @ `0x004f7ad0`
- Behaviour gate (slot 82) @ `0x004e9790`, chase (slot 84) @ `0x004e02d0`, target (slot 85) @ `0x004e5fc0`, attack (slot 86) @ `0x004fe740`, abilities (slot 81) @ `0x004e5900`
- Animation (slot 89) @ `0x004dfc50`; draw body (slot 83, called by `Fish::Draw` @ `0x004e4a90`) @ `0x004f8610` → `0x004df710`

(Ghidra's `vfunctionN` names are slot N − 1.)

Confidence: `traced` overall, per section below. The replay `tank1_1.rs` `tank_1_4_from_start` compares Itchy (`fish_pets`: position, velocity, modes, timers, animation) and the app RNG every tick against `tank1-4_full_01`: 9,500 ticks, three kind-3 waves, 52 stabs.

## Fields used by id 2

The `Fish` fields (raw offsets, `fish-guppy.md#fields`): x/y `+0x158/+0x160`, vx/vy `+0x168/+0x170`, patrol dir `+0x178`, divisor `+0x180`, facing `+0x188`, bounds `+0x190` (floor 370) / `+0x194` (min x 10) / `+0x198` (ceiling 95) / `+0x19c` (max x 540), mode `+0x1ac`, mode timer `+0x1b0`, reroll timer `+0x1b4`, swim cycle `+0x1b8`, speed bucket `+0x1bc`, cel `+0x1c0`, turn `+0x1c4`, eat timer `+0x1c8`, push timer/velocity `+0x20c/+0x218`, misc timer `+0x210`.

`FishTypePet` adds (raw): `+0x230` byte drop-in flag, `+0x234` pet id, `+0x238` drop-in countdown (360 with the flag, else 0), `+0x23c` and `+0x264` countdowns, `+0x245` byte "slow" (movement ÷ 5·div). For Itchy in Adventure all of these stay 0.

`confirmed` (`tank1-4_full_01` raw `pet_fish`: id 2 at `+0x234`, the bounds above, mode/timers moving as described below).

## Construction

`Board::AddPet(2, -1, -1)` (`0x00544a90`), in order, **app stream**:

1. Position: **x = `r % 265 + 105`**, **y = `r % 520 + 20`** (same two draws as every pet; `pet-stinky.md#construction`).
2. `FishTypePet` ctor → `Fish::Fish` → **`Fish::Init` (`0x004de370`): the guppy's 6 draws** (`fish-guppy.md#construction`): facing (`& 1`), divisor (`[2.0, 1.8, 1.6][r % 3]`), hunger (`% 200 + 400`, never used), growth needed (`% 3 + 4`), mode (`% 10`), coin interval (`% 200 + 150`, never used). Then type `0x15`, id 2, the `FishTypePet` fields above. No id-2 branch draws.
3. Shadow (`0x00544800`): `FishTypePet` with id ≠ 16/20 gets a **type-0 shadow**, whose ctor makes **one draw** (`% 5`), as a guppy's.

So adding Itchy costs **9 app draws**. Pets are added in id order before the two guppies (`level-start.md`), so on 1-4 the order is Stinky, Niko, Itchy.

`traced` (`tank1_1.rs` `level_1_4_start_with_itchy`: Stinky, Niko, Itchy and both guppies built from the pre-populate RNG match the capture's first tick, including the app-RNG position).

## Update order

`+0xb4` comes right after `+0xb0` (`OtherTypePet`) in `RebuildZOrder` (`board.md`): Itchy updates **after the aliens and Stinky**, before missiles, shots and coins. `traced`: his re-roll draws sit after Stinky's in the RNG every 21 ticks of `tank_1_4_from_start`.

## Update (`0x004f7ad0`), id 2, normal game

0. Return if there's no board or it's paused.
1. Common timers (`Fish::PreUpdate` `0x004f22c0`: `+0x108`, `+0x10c`, `+0xf8` count down; the emote branch needs `+0x114 > 0`, never for Itchy).
2. `+0x264` and `+0x23c` count down to 0 (unused for Itchy).
3. **Behaviour.** Slot 82 (`0x004e9790`): for id 2 it **chases when any alien is present** (`board+0xb8` or `+0xf4` non-empty) and returns true, skipping the swim modes; otherwise returns false and the swim modes run.
4. **Swim modes** (no aliens). The guppy's machine (`fish-guppy.md`, step 3) with these differences:
   - There is **no mode 123** and no `+0x208` gate: mode 0 always sets vy = 0.5, modes 1/2 and 5–9 always set vy = −0.5 (−0.1 when y < 115).
   - Mode −1 (never set in Adventure) sets vy = −10.
   - Everything else is the same: the 40-tick steps, the speed bucket `+0x1bc = trunc(|vx|)`, `y −= 0.25/div` (mode 0) or `0.5/div` (modes 1–2), modes 3/4 → mode 0 below y 240, the patrol turn points 250/175.
5. `mode_timer += 1`, `reroll_timer += 1`. When `reroll_timer > 20` (or, for id 21 only, y > floor − 50): reset it, **`r % 10`**, and if 0 **`mode = r % 9 + 1`**. One or two app draws every 21 ticks, whether or not aliens are present.
6. Slot 81 (abilities, `0x004e5900`) runs when no alien is present; it has no id-2 branch. Nothing happens.
7. Push timer `+0x20c`: if non-zero, decrement, `push *= 0.9`, `x += push`. Misc timer `+0x210` −1 while > 0.
8. **Drift by vx bucket** (exact compares): vx == 0 → `y += 1/div`; 1 → `0.75/div`; 2 → `0.5/div`; 3 → `0.25/div`. Unlike a guppy, there is no `y −= 0.25` for a low floor.
9. Drop-in countdown `+0x238` −1 if non-zero.
10. **Clamp**, explicit compares in this order: x > max x → max x; x < min x → min x; y > floor → floor; y < ceiling → ceiling. There are no bubbles (unlike the guppy's step 10).
11. **Edge steer**: x > max x − 5 and vx > 0.1 → `vx −= 0.1`; x < 15 and vx < −0.1 → `vx += 0.1`.
12. **Animation** (below).
13. **Integrate**: `x += vx / div`, `y += vy / div` (the slow byte would use `div · 5`). Widget = `trunc(x), trunc(y)`.

`traced` (`tank_1_4_from_start`). The mode −1 and slow-byte branches are `static` (never taken on 1-4).

## Chasing an alien (`0x004e02d0`)

1. **Target** (`0x004e5fc0`): best = 10000. Walk the Bilaterus list `+0xf4`, then the alien list `+0xb8`, **in list order**. For each: `dx = trunc((a.wx + 80) − (x + 40))`, `dy = trunc((a.wy + 80) − (y + 40))` (doubles truncated to int), `d = trunc(f32 sqrt(f32(dx² + dy²)))`. An alien is a candidate unless its byte `+0x204` is set (a kind-7 state; 0 for kinds 1–3). It wins if `d < best`. The 80 is half the 160-px alien widget; for type `0x17` targets it would be 40.
2. If `mode_timer < 5` or there's no target, skip the steering. Otherwise `mode_timer = 0` and steer, so steering runs **every 5th tick**:
   - `if x + 40 > a.wx + 80: if vx > −10: vx −= 2.5`, `elif x + 40 < a.wx + 80: if vx < 10: vx += 2.5`.
   - `if y + 40 > a.wy + 80: if vy > −4: vy −= 1.5`, `elif y + 40 < a.wy + 80: if vy < 4: vy += 1.5`.
   - Speed bucket `+0x1bc` += 1 while below 5 (faster swim animation).
3. If there is a target, **attack** (every tick, step 2 or not).

The mode timer `+0x1b0` is shared with the swim modes: it doubles as the steering clock here.

`traced` for one alien at a time (`tank_1_4_from_start`). Bilaterus, `+0x204` and several aliens are `static`.

## Attack (`0x004fe740`), id 2

For each Bilaterus (`+0xf4`, not on tank 1), then each alien in **list order**: if `|(x + 40) − (a.wx + 80)| < 20` and `|(y + 40) − (a.wy + 80)| < 20` (doubles) and the alien's `+0x204` is 0:

- `0x004fb5b0`: **alien health −= 1.0** (−0.25 for kinds 4 and 5), and play sample `0x130` with a **throttle of 11 ticks** (`0x00538340(11)` → `0x00538230(0x130, 11)`). No RNG.
- Stop: at most one alien is hit per tick.

There is no cooldown, no knockback and no death check here: the alien dies in its own update on the next tick, when it sees health ≤ 0 (`alien-sylvester.md` §3 step 10). With 130 HP, a kind-3 alien that Itchy keeps overlapping dies in 130 ticks (3.6 s) without help. When his stab finishes an alien, it dies in its own update the next tick, and the corpse updates in that same tick (`board.md`: objects created during the object pass update in it).

`traced` for the 1.0 damage and the kill (`tank_1_4_from_start`: 52 stabs, one kill by a stab at tick 980954). The `PUNCH` throttle is `traced` by the sample table. Kinds 4/5 (0.25) are `static`.

## Animation (`0x004dfc50`), id 2

1. **Turn trigger**: facing > 0 and vx < 0 → turn = 20; facing < 0 and vx > 0 → turn = −20.
2. Turn steps one toward 0; when it reaches 0, the eat timer `+0x1c8` is cleared.
3. While turning, cel = `9 − turn/2` (turn > 0) or `turn/2 + 9` (turn < 0). Otherwise (id 2 takes the default branch) the swim cycle `+0x1b8` advances by 1 (speed bucket ≤ 1) or 2, wraps at 20, and cel = cycle / 2.
4. Facing follows vx when both are non-zero and differ (the guppy rule).

`traced` (`tank_1_4_from_start` compares turn, cycle, cel and facing every tick).

## Drawing (`0x004df710`)

`IMAGE_ITCHY`, 80×80 cels, src `(cel·80, row·80)`:

| | no alien | alien present |
|---|---|---|
| swimming | row 0 | row 2 |
| turning | row 1 | row 3 |

Mirroring: while turning, mirrored iff turn > 0. Otherwise `Fish::Draw` (`0x004e4a90`) passes the guppy's mirror choice (by vx, or facing when vx is 0; `fish-guppy.md#animation-and-drawing`). A type-0 shadow follows him like a guppy's. No hunger fade, no flash overlay (that's for drop-ins and id 21). Drawn in the `+0xb4` slot of the z-order, after the `OtherTypePet`s.

`traced` by frame (`frames.rs` `tank_1_4_calm_frames_match_original` and `tank_1_4_alien_frames_match_original`: rows 0 and 2, both mirror states, and his shadow, pixel-exact). The turning rows 1/3 are `static` (no captured frame mid-turn).

## Not on tank 1-4

The drop-in arrival (`+0x230`, the 360-tick countdown and the white flash), Virtual Tank (mode 5), the cursor-follow override and the other FishTypePet ids.

`static` (listed from the code paths skipped above).
