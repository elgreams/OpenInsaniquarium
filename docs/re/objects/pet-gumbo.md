# Gumbo the angler (`Sexy::FishTypePet`, pet id 12)

Gumbo swims like a guppy while the tank is calm. While an alien or Bilaterus is in the tank his lantern glows, he **swims to the opposite side of the tank from the nearest alien**, and **every guppy that isn't chasing food steers toward his lantern** instead of running its swim modes. Aliens themselves are not changed: they keep chasing the nearest prey, which is now gathered around Gumbo, away from them.

Same class, update and fields as Itchy (`pet-itchy.md`); only the id-12 differences are listed. Raw `this`-relative offsets.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`: no id-12 branch
- Behaviour gate (slot 82) @ `0x004e9790`, id-12 test at line `iVar1 != 0xc` (`0x004e9840`..`0x004e9858`)
- Chase (slot 84) @ `0x004e02d0`, id-12 branch at `0x004e05ef`; target (slot 85) @ `0x004e5fc0`, id-12 branch at `0x004e678c`; attack (slot 86) @ `0x004fe740` has no id-12 branch
- Guppy side: `Fish::Update` @ `0x004f6970`, lure branch `0x004f6b72`..`0x004f6fbb`, reading the pet counter **`board+0x174`** (`board+0x144 + 4·12`)
- Animation (slot 89) @ `0x004dfc50`, id-12 branch; draw body `0x004f8610` case 12 → `0x004df470`

Confidence: everything here is `static` (no capture with Gumbo yet).

## Fields used by id 12

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), plus `+0x258` (double, glow phase, 0.0 from the ctor). Divisor `+0x180` is the `Fish::Init` draw (2.0, 1.8 or 1.6); bounds are a guppy's (floor 370). `+0x220`/`+0x224` are unused.

`static`.

## Construction

`Board::AddPet(12, -1, -1)` (`0x00544a90`): the same **9 app draws** as Itchy (2 position, 6 `Fish::Init`, 1 type-0 shadow). No id-12 ctor branch, so no extra draw. `board+0x174 += 1` when he is added to `+0xb4` (`sim/board.md`, pet counters).

`static`.

## Update (`0x004f7ad0`), id 12

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the gate (slot 82) for id 12: if an alien (`+0xb8`) or Bilaterus (`+0xf4`) exists (`0x004da780`), call the chase (slot 84, below) and **return true**: the swim modes are skipped. Otherwise return false and the swim modes run. On tank 5 or while paused the gate returns early (false), as for every pet.
- **Step 6:** slot 81 has no id-12 branch.
- **Animation:** own branch (below).

RNG: the re-roll draws only (`r % 10`, then `r % 9 + 1` on 0, every 21 ticks, aliens or not).

`static`.

## Fleeing the alien (chase `0x004e02d0`, id 12)

1. **Target** (`0x004e5fc0`, id-12 branch): best = 10000. Walk the Bilaterus list `+0xf4`, then the alien list `+0xb8`, in list order. For each, with `h` = 40 for a Bilaterus and 80 for an alien: `dx = trunc((x + 40) − (a.wx + h))`, `dy = trunc((y + 40) − (a.wy + h))`, `d = trunc(f32 sqrt(f32(dx² + dy²)))`; it wins if `d < best`. Unlike Itchy's search there is **no `+0x204` filter**.
2. If `mode_timer (+0x1b0) < 5` or there is no target, skip the steering. Otherwise `mode_timer = 0` and steer by the alien's **centre** `acx = a.wx + h`, `acy = a.wy + h` (ints), **not** by Gumbo's own position:
   - vertical: if `acy ≥ 261` and `vy > −8`: `vy −= 2`; else if `acy < 300` and `vy < 8`: `vy += 2`.
   - horizontal: if `acx ≥ 291` and `vx > −8`: `vx −= 2`; else if `acx < 330` and `vx < 8`: `vx += 2`.

   (Literal order: with `acy` in 261..299 and `vy ≤ −8` the second rule fires and `vy` goes back up by 2; the same for x in 291..329 with `vx ≤ −8`. Speeds therefore stay within about ±8 / ±10.) So an alien in the lower half sends him up, in the upper half down; on the right half left, on the left half right: he heads for the far corner. Steering runs every 5th tick. The speed bucket `+0x1bc` is **not** raised (Itchy's is).
3. If there is a target, call the attack (slot 86): it has no id-12 branch and does nothing.

The rest of the update (drift by vx bucket, clamps, edge steer, `x += vx/div`) moves him as usual. No RNG, no sound.

`static`.

## Effect on guppies (`Fish::Update` `0x004f6970`, at `0x004f6b72`)

In `Fish::Update` step 2 (`fish-guppy.md#fishupdate-order-per-tick`), when the fish is **not** sheltered by Wadsworth (`pet-wadsworth.md#effects-on-guppies`, which takes precedence) and `Fish::UpdateHunger` (slot 83) returned false (not chasing food), then **if** an alien or Bilaterus exists (`0x004da780`), **`board+0x174 ≠ 0`** (a Gumbo is in the tank) and the fish's `+0x229` byte is set (guppies of any stage; carnivores and pets clear it), the fish is **lured** this tick instead of running its swim modes:

1. Find the **first pet in `+0xb4` (list order) with id 12**; `gx, gy` = its widget position (ints). With two Gumbos, only the first lures.
2. With `cx = x + 40`, `cy = y + 40` (the guppy's centre, doubles), the first matching rule per axis:
   - x: `cx > gx + 50` → if `vx > −4`: `vx −= 1.3`; `cx < gx + 30` → if `vx < 4`: `vx += 1.3`; `cx > gx + 45` → if `vx > −4`: `vx −= 0.2`; `cx < gx + 35` → if `vx < 4`: `vx += 0.2`; `cx > gx + 40` → if `vx > −4`: `vx −= 0.05`; `cx < gx + 40` → if `vx < 4`: `vx += 0.05`.
   - y: `cy > gy + 25` → if `vy > −3`: `vy −= 1.0`; `cy < gy + 15` → if `vy < 4`: `vy += 1.3`; `cy > gy + 20` → if `vy > −3`: `vy −= 0.5`; `cy < gy + 20` → if `vy < 4`: `vy += 0.7`.

   The aim point is `(gx + 40, gy + 20)`: Gumbo's horizontal centre, 20 px below the top of his 80 × 80 widget, i.e. the lantern. Each rule only fires while the speed is under its cap, so a far guppy accelerates by 1.3 per tick up to ±4.
3. If the fish's widget y `+0x34` ≤ its ceiling `+0x198` and `vy < 0`: `vy = 0`.
4. Speed bucket `+0x1bc += 1` while below 5 (faster swim animation).
5. Continue with the rest of `Fish::Update` (mode/re-roll timers with the **unchanged** re-roll draws, coin drop (none while aliens are in), damping, drift, clamps, edge steer, animation, `x += vx / div`, `y += vy / div`).

The lure runs every tick, has **no RNG** and skips mode 123's draws (the swim modes don't run). A hungry guppy that has food to chase chases it first (`food.md`: with aliens, slot 83 still chases food when hunger < 500 and the first alien isn't kind 4). Gumbo updates after the guppies (`+0xb4` comes later in the z-order), so guppies steer toward his position from the previous tick.

`static`.

## Effect on aliens

None directly. `board+0x174` was looked for in the alien prey search `0x004e8b60`, eat test `0x004e8f70` and prey filter `0x00500ff0`, and isn't read there: aliens target the nearest prey as usual (`alien-sylvester.md#31-nearest-prey-0x004e8b60`). The "luring away" comes from the two rules above: Gumbo heads for the far corner from the alien and the guppies follow him. Gumbo, as a pet, is not prey.

`static`.

## Animation (`0x004dfc50`), id 12

Turning is Itchy's (turn ±20, cel `9 − turn/2` / `turn/2 + 9`); while turning nothing else below runs. When not turning:

- Swim cycle `+0x1b8` += 1 (speed bucket ≤ 1) or 2; set to 0 when it exceeds 19. **Cel = cycle / 2** (as Itchy's).
- **While an alien or Bilaterus exists:** glow phase `+0x258 += 0.1`; if it is then ≥ 1.0, set it to −1.0. (It freezes, not resets, when the tank is calm.)

Facing follows vx (guppy rule). No RNG.

`static`.

## Drawing (`0x004df470`)

`IMAGE_GUMBO` (`0x005e8cb4`, 10 × 2 cels of 80 × 80), src `(cel·80, row·80)`:

| when | row | mirror |
|---|---|---|
| turning (`+0x1c4 ≠ 0`) | 1 | iff turn > 0 |
| otherwise | 0 | guppy rule |

Then, **while an alien (`+0xb8`) or Bilaterus (`+0xf4`) exists**, the same cel and row of `IMAGE_GUMBOLIGHT` (`0x005e8bcc`, 10 × 2) with colourised images, additive mode and colour (255, 255, 0, `trunc(|+0x258| · 255)`): a yellow lantern glow pulsing with a 20-tick period. A type-0 shadow follows him.

`static`.

## Open questions

- Other readers of `board+0x174`: only the files already decompiled were searched (Fish::Update, the alien prey routines). Breeders, Oscars or other alien kinds (`0x004e95f0`, `0x004e9470`, `0x004e8df0`) might read it too.
- Which `Fish::Update` users besides guppies keep `+0x229` set (Ultravore, Gekko, the `+0xf8` fish)?
- The flee rules use the alien's absolute position, not Gumbo's: confirm against a capture (any wave with Gumbo).
