# Brinkley the scuba diving elephant (`Sexy::FishTypePet`, pet id 20)

Brinkley is a heavy swimmer: on top of the usual vx-bucket drift he sinks by another 0.5–1.0 px every tick, so he stays near the bottom unless he is chasing something. While the tank is calm and any food is in it, he chases the nearest pellet and **eats it** (a slurp, then 45 ticks of digesting before he chases again). **Every 3rd pellet he eats, he spits out a rising coin**: a star ($40), a diamond every 15th pellet ($200), a chest every 45th ($500), and the $2000 coin every 135th. In-game text: "BRINKLEY was a performing elephant … could not escape his own voracious appetite. An appetite for... food!" He is a bonus pet, bought with shells (`screens/bonus-screen.md`). While he is in the tank, Zorf feeds more often (`pet-zorf.md#feeding`).

Same class, update and fields as Itchy (`pet-itchy.md`). Only the id-20 differences are listed here. All offsets are raw `this`-relative (Ghidra's `FishTypePet_data.offset_0xN` = raw `+0x22c + N`, `Fish_data.field_0xN` = raw `+0x154 + N`).

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`: no id-20 branch
- Shadow `0x00544800`: id 20 → type-1 shadow
- Update (slot 22) @ `0x004f7ad0`: the extra sink at `0x004f835c`..`0x004f8404`
- Gate (slot 82) @ `0x004e9790`: id-20 test; chase (slot 84) @ `0x004e02d0`, id-20 steering at `0x004e0c9e`..`0x004e0e0f`; target (slot 85) @ `0x004e5fc0`, id-20 branch at `0x004e6edc`; eat (slot 86) @ `0x004fe740`, id-20 branch at `0x004ff2ec`
- Slurp sound `0x005384c0`; food removal `0x004d9570`; coin `Board::AddCoin` @ `0x00544430`; Virtual Tank coin gate `0x004d70b0`
- Animation (slot 89) @ `0x004dfc50` (branch shared with Nimbus); draw body `0x004f8610` case 0x14 → generic pet draw `0x004dfbb0` with `IMAGE_BRINKLEY`
- Readers of the Brinkley count `board+0x194`: Zorf's feeding (`0x004e5cc3`, `pet-zorf.md#feeding`)

Confidence per section. Captures: `brinkley_01` (1-2 with Zorf, ~9 200 ticks, his count pushed to 44 with Cheat Engine: stars and the chest) and `brinkley_02` (a later chunk of the same session, pushed to 134: the $2000 coin), both replayed exactly from the game's own demo. `traced` where a section says so, else `static`.

## Fields used by id 20

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), with:

| Offset | Type | Meaning for Brinkley | Initial |
|---|---|---|---|
| `+0x180` | double | divisor: `Fish::Init`'s draw (2.0, 1.8 or 1.6), never overwritten | drawn |
| `+0x23c` | int | **digest cooldown**: set to **45** after each pellet (**108** in the Virtual Tank, `app+0x87c == 5`); −1 per tick in the update (Itchy's step 2); he chases only while it is 0 | 0 |
| `+0x240` | int | **pellets eaten** (Ghidra `offset_0x14`), +1 per pellet, never reset | 0 |
| `+0x1b0` | int | mode timer, doubles as the 5-tick steering clock (as Itchy) | |
| `+0x1bc` | int | speed bucket; the chase raises it by 1 per steering tick up to 5 | |

The bounds are a guppy's (floor 370, min x 10, ceiling 95, max x 540). The board keeps his count in the per-id pet counter `board+0x144 + 4·20 = board+0x194` (`board.md#object-lists`).

`traced` by `brinkley_zorf` (`brinkley_01`) and `brinkley_rare_coin` (`brinkley_02`).

## Construction

`Board::AddPet(20, -1, -1)` (`0x00544a90`), app MTRand, in order:

1. Position: `x = r % 265 + 105`, `y = r % 520 + 20` (2 draws).
2. `Fish::Init` (`0x004de370`): the guppy's 6 draws (facing, divisor, hunger, growth, mode, coin interval). The divisor drawn here is kept.
3. Ctor: no id-20 branch, no draw. `+0x23c = 0`, `+0x240 = 0`.
4. Shadow (`0x00544800`): for type code `0x15` the shadow type is 2 for id 16, **1 for id 20**, 0 otherwise. Type 1 is the flat floor shadow the OtherTypePets get (`pet-clyde.md#construction`). The Shadow ctor makes **1 draw** (`% 5`) whatever the type.

So adding Brinkley costs **9 app draws**, as Itchy. Made by Presto (`pet-presto.md#the-change`): the same minus the 2 position draws.

`traced` by `brinkley_zorf` (`brinkley_01`) and `brinkley_rare_coin` (`brinkley_02`) (his draws at level start).

## Update (`0x004f7ad0`), id 20

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 2:** the digest cooldown `+0x23c` counts down by 1 while non-zero.
- **Step 3, gate** (`0x004e9790`; not called at all on tank 5, `board+0x3c8 == 5`, so there he only swims): he **chases** (slot 84, [Chase](#chase)) and returns true, skipping the swim modes, when all of these hold:
  - no alien (`board+0xb8`) and no Bilaterus (`board+0xf4`) is present (`0x004da780` false),
  - the food list `board+0xac` is non-empty (`0x004da800`),
  - the digest cooldown `+0x23c` is 0.

  Otherwise the gate returns false and the guppy swim modes run. Any food counts for the gate (potions, Zorf's thrown pellets, Nimbus's tossed ones).
- **Step 6:** slot 81 has no id-20 branch.
- **Step 8, drift:** first Itchy's exact-match drift (vx == 0 → `y += 1/div`, 1 → `0.75/div`, 2 → `0.5/div`, 3 → `0.25/div`), **then an extra sink, not divided by the divisor**, by `|vx|` (first match):

  | `|vx|` | extra `y +=` |
  |---|---|
  | == 0 | 1.0 |
  | ≤ 1 | 0.75 |
  | ≤ 3 | 0.5 |
  | > 3 | 0 |

  So at rest he sinks `1/div + 1.0` per tick (1.5–1.6), and even while swimming he sinks 0.5–0.75 per tick against the swim modes' −0.5/div lift: he lives on the floor (370). The chase's vy (down to −2.0, divided by the divisor in step 13) is the only thing that lifts him.
- **Animation:** Nimbus's branch (below).

RNG: the 21-tick re-roll (`r % 10`, then `r % 9 + 1` on 0), as Itchy, plus the slurp draw (below). The gate, chase and target search draw nothing.

`traced` by `brinkley_zorf` (`brinkley_01`) and `brinkley_rare_coin` (`brinkley_02`): the gate, the extra sink, the re-roll.

## Chase

`0x004e02d0`, id 20, every tick the gate fires:

1. **Target** (`0x004e5fc0`, id 20): best = 10 000, strict `<` (first minimum wins). Over the food list `board+0xac` in list order, with **no filter** (every kind, any motion, tossed or not): `dx = trunc((x + 40) − (f.wx + 20))`, `dy = trunc((y + 40) − (f.wy + 20))`, `d = trunc(f32 sqrt(f32(dx² + dy²)))`. The nearest pellet wins (no `dx → 50` rule, unlike Nimbus).
2. If `+0x1b0 < 5` or there is no target, no steering. Otherwise `+0x1b0 = 0` and steer, with `cx = x + 40`, `w = target.wx` (int), first matching rule:

   Bands by position, **each speed guard inside its band** (no fall-through to the next band when the guard fails), and the steps are **added** to the velocity:

   | Band (first match) | if | vx |
   |---|---|---|
   | `cx > w + 28` | `vx > −3` | −= 1.0 |
   | `cx < w + 12` | `vx < 3` | += 1.0 |
   | `cx > w + 24` | `vx > −3` | −= 0.1 |
   | `cx < w + 16` | `vx < 3` | += 0.1 |
   | `cx > w + 20` | `vx > −3` | −= 0.05 |
   | `cx < w + 20` | `vx < 3` | += 0.05 |

   Then with `cy = y + 40`, `h = target.wy`:

   | Band (first match) | if | vy |
   |---|---|---|
   | `cy > h + 26` | `vy > −2` | −= 0.6 |
   | `cy < h + 14` | `vy < 3` | += 1.0 |
   | `cy > h + 20` | `vy > −2` | −= 0.3 |
   | `cy < h + 20` | `vy < 3` | += 0.5 |

   (Corrected against `brinkley_01`: the first draft read these as assignments with fall-through and a 2.5 vy guard; the code at `0x004e0bb4` adds, nests, and uses 3.0.)

   Then the speed bucket `+0x1bc += 1` while below 5. The +20 lines his centre up with the pellet's centre (40-px food widget).
3. If there is a target, **eat** (every tick, steering tick or not).

`traced` by `brinkley_zorf` (`brinkley_01`) and `brinkley_rare_coin` (`brinkley_02`).

## Eat (`0x004fe740`, id 20)

Over the food list `board+0xac` in list order, the **first** pellet with (doubles, strict):

- `f.wx + 5 < x + 40 < f.wx + 35` and
- `f.wy < y + 40 < f.wy + 35`

(no kind, motion or alien check). For it, in order:

1. **Slurp** (`0x005384c0(0)`): if sample **`0x138` (`SLURP`)** is off its **3-tick** throttle (the board sample table `0x005381f0`, which stamps `0x138`), **1 app draw `r % 5`**: 0 → `SOUND_SLURP3`, 1 → `SOUND_SLURP2`, 2–4 → `SOUND_SLURP`, played through the app's sound manager. No draw while throttled. (The same shape as Gash's chomp, `pet-gash.md`.)
2. **Remove the pellet** (`0x004d9570`, as Nimbus's toss). No food ctor, no RNG.
3. `+0x240 += 1`; `+0x23c = 45` (108 in the Virtual Tank).
4. If `+0x240 % 3 ≠ 0`, stop. Otherwise, if `0x004d70b0` allows a coin (always outside the Virtual Tank; in it, only when `board+0x500` is set and a coin limit isn't reached, not followed): **`Board::AddCoin(wx + 5, wy − 10, kind, 0, −1.0, 0)`** at his widget position **as of the start of the tick** (the eat runs in the chase, before his move), with

   | `+0x240` | kind | value (`coin.md#values-0x004d53a0-normal-game`) |
   |---|---|---|
   | multiple of 135 | 14 | 2000 |
   | else multiple of 45 | 13 | 500 |
   | else multiple of 15 | 11 | 200 |
   | else (multiple of 3) | 10 | 40 |

   These are the **rising** kinds (Nimbus's toss makes the same ones): the coin flies up to y < 120 and then falls like any coin (`pet-nimbus.md#effects-on-other-objects`). `Coin::Coin` draws nothing. The coin goes to the coin list `+0xa8`. Collecting it (`0x004e51f0`): kind 10 the POINTS family (one draw), 11 `DIAMOND` (`0x116`), **13 the `PEARL` sample (`0x12b`)**, **14 `TREASURE` (`0x142`)** (`BONUSCOLLECT`, `0x10f`, in the Virtual Tank or with `board+0x2a5` set), each with a 3-tick throttle.

At most one pellet per tick. RNG per pellet: the slurp draw only (0 or 1).

Update order (`sim/board.md#tick-order`): food `+0xac` and guppies `+0xa0` update before the FishTypePets `+0xb4`, so a guppy that reaches a pellet in the same tick eats it first, and the pellet Brinkley removes has already moved this tick.

`traced` by `brinkley_zorf` (`brinkley_01`) and `brinkley_rare_coin` (`brinkley_02`): slurp, removal, the count, coin kinds 10, 11, 13 and 14 and their collect sounds. The Virtual Tank gate is `static`.

## Effects on other pets and the board

- **Zorf** (pet 4): while `board+0x194 > 0`, his 65-tick check drops a pellet every 2nd check even when no fish is hungry (every 4th in the Virtual Tank) (`pet-zorf.md#feeding`). Brinkley then eats those pellets, so the pair is a coin machine. No other reader of `board+0x194` was found (a byte scan of `.text` for board-relative `+0x194` reads finds only Zorf's; the other `+0x194` hits are the food and coin kind fields).
- **Guppies:** he takes their food; nothing else.
- **Aliens:** none (he ignores them; the gate stops the chase while one is in).
- **Clicks:** not mouse-visible (`+0x55` = byte `0x005e8f14`, 0) and no id-20 case in the mouse-down `0x004fb680`: clicks pass through to the tank.

`traced` by `brinkley_zorf` (`brinkley_01`) and `brinkley_rare_coin` (`brinkley_02`) for Zorf's every-2nd drop (`board+0x194`). The rest `static`.

## Animation (`0x004dfc50`), id 20

Turning is Itchy's (turn ±20, cel `9 − turn/2` / `turn/2 + 9`). When not turning (branch shared with Nimbus, `pet-nimbus.md#animation-0x004dfc50-id-15`):

- swim cycle `+0x1b8 += 1` if the speed bucket `+0x1bc < 2` or the slow byte `+0x245` is set, else `+= 2`; 0 when it exceeds 39;
- **cel = cycle / 4** (0..9).

Facing follows vx (guppy rule). No RNG.

`traced` by `brinkley_zorf` (`brinkley_01`) and `brinkley_rare_coin` (`brinkley_02`).

## Drawing (`0x004dfbb0`)

The generic pet draw (`pet-vert.md#drawing-0x004dfbb0`) with `IMAGE_BRINKLEY` (global `0x005e8b4c`; `resources.xml` `brink`, 10 × 2 cels of 80 × 80, framedelay 4), src `(cel · 80, row · 80)`: row 1 while turning (mirrored iff turn > 0), row 0 otherwise (guppy mirror rule). `IMAGE_ZZZ` at (30, −10) / (50, −10) mirrored when the slow byte is set (Virtual Tank only). His shadow is type 1. The Presto puff/label apply when he was made by Presto (`pet-presto.md#drawing`).

`static`.

## Open questions

- Whether the food removal `0x004d9570` takes the pellet off `+0xac` at once (the same question as Nimbus's).
- `0x004d70b0` (the Virtual Tank coin gate: `board+0x500`, `board+0x400` against `0x005df590`) is not followed.

`static`.

## Recording

A pet zoo on tank 1-2 with Brinkley (no aliens needed; he stops chasing while one is in, so record between waves). Drop food steadily (the food limit on 1-2 is 1 pellet; he competes with the guppies, so `--feed-low` or drops near the bottom help). A coin every 3rd pellet: about 9 pellets give three stars. For the rarer kinds, push the eaten count `+0x240` with Cheat Engine to **14**, **44** and **134** before a pellet (the next one makes a diamond, a chest, the $2000 coin); the replay must adopt that write as it does for `+0x220`. Adding Zorf (pet 4) to the zoo exercises `board+0x194` (Zorf's every-2nd-check drop) and feeds Brinkley without clicks.

`static` (a plan).
