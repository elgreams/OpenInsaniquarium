# Blip the porpoise (`Sexy::FishTypePet`, pet id 13)

Blip swims like a guppy and has no ability code of the usual kind. His presence (pet counter **`board+0x178`** = `board+0x144 + 4·13`, non-zero) turns on information displays: a hunger marker over every fish that will soon be hungry, health bars on aliens, and the aim reticle that marks where the next alien will appear. He also reveals every shop button early in the level. In-game text: "BLIP provides you with info ... your guppies from time to time" (`0x005a2da8`). He is the pet hatched after tank 3-4 (`screens/hatch-screen.md#pet-award-table-adventure-first-play-through`).

**He is not purely presentation.** Two things change the simulation: the shop reveal in his update, and the reticle draw, which also pins the alien spawn y to 280 (a gameplay write in the draw path, `alien-sylvester.md#78-danger-reticles`). Neither draws RNG.

Same class, update and fields as Itchy (`pet-itchy.md`); only the id-13 differences are listed. Raw `this`-relative offsets.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`, gate (slot 82) `0x004e9790`, abilities (slot 81) `0x004e5900`: no id-13 branch
- Update `0x004f7ad0`: id-13 shop reveal at `0x004f7b01`..`0x004f7b82` (the only `cmp [reg+0x234], 13` in `.text`)
- Readers of `board+0x178` (a byte scan of `.text` for `[board + 0x178]` loads after an `app+0x730` load finds these three plus the board draw):
  - hunger test `0x004d6980(threshold)` @ `0x004d699a`, used by the body draws of every fish class;
  - alien draw (slot 27) `0x004dae20` @ `0x004db137`;
  - Bilaterus head draw (slot 27) `0x004e50b0` @ `0x004e5155`;
  - board draw `0x0053f5f0` @ `0x0053ffc3` (reticles).
- Animation `0x004dfc50`: default branch; draw body `0x004f8610` case 13 → generic pet draw `0x004dfbb0` with `IMAGE_BLIP`

Confidence: everything here is `static` (read from code only; no capture with Blip yet).

## Fields used by id 13

As Itchy's (`pet-itchy.md#fields-used-by-id-2`). Divisor `+0x180` is the `Fish::Init` draw (2.0, 1.8 or 1.6); bounds are a guppy's (floor 370, ceiling 95). `+0x220`/`+0x224` unused.

The board counter `board+0x178` is incremented by `Board::AddGameObject` (`0x00542ee0`, type 0x15 → `board+0x144 + 4·id`) and decremented by the list removal `0x00541d90`, as for Seymour (`pet-seymour.md`).

`static`.

## Construction

`Board::AddPet(13, -1, -1)` (`0x00544a90`): **9 app draws**, as Itchy (2 position, 6 `Fish::Init`, 1 type-0 shadow). No id-13 ctor code.

`static`.

## Update (`0x004f7ad0`), id 13

As Itchy's update, with:

- **Step 1b, shop reveal** (right after `PreUpdate`, before everything else): if
  - `app+0x87c ≠ 5` (not the Virtual Tank), and
  - `board+0x2e0 < board+0x2dc` (fewer buttons revealed than slots mapped, `sim/shop-bar.md#fields`), and
  - `board+0x3c8 ≠ 5` (not tank 5), and
  - the level tick `board+0x444 > 200`, and
  - the drop-in flag `+0x230` is clear, or the object's raw `+0x28` (a per-object tick counter, presumably its age) is > 720,

  then play `Board::PlaySample(0x13c SONAR, gap 3, volume 1.0)` (`0x00538230`) and call `RevealSlotButton(slot, popAnim = 1)` (`0x005409b0`) for slots 0..11. Unmapped or already revealed slots are skipped by the reveal itself; each new one increments `+0x2e0`, so after this tick the condition is false and it fires **once per level**, on the first update with tick 201 (or when he arrives later). No RNG (`sim/shop-bar.md#reveal`).
- **Step 3:** the gate has no id-13 branch and returns false: the guppy swim modes always run, aliens or not.
- **Step 6:** slot 81 has no id-13 branch.
- **Animation:** the default branch (Itchy's): swim cycle += 1 (speed bucket ≤ 1) or 2, wraps at 20, cel = cycle / 2; turning ±20 as Itchy.

RNG: only the 21-tick re-roll (`r % 10`, then `r % 9 + 1` on 0).

`static`.

## Effects on other objects

All gated on `board+0x178 ≠ 0`; none draws RNG.

1. **Shop buttons** (above): all mapped slots appear at tick 201. This changes what the player can click (and the slot positions' pop-in animation), so it matters for replays of input logs.
2. **Alien spawn position** (board draw `0x0053f5f0`, `0x0053ffaa`..`0x005400c3`): while the wave countdown `board+0x2c0` is in 1..225, and not tank 5 and not the Virtual Tank, the board draw:
   - sets **A.y `+0x2d0` = 280** and draws `IMAGE_CROSSHAIR` (global `0x005e8e1c`, name `0x0059cb34`) additively at (A.x + 40, 320), cel `(board+0x444 / 4) % 5`;
   - for wave kinds 9..12 also draws one at (B.x + 40, B.y + 40), cel `(board+0x444 / 4 + 2) % 5`; for **kind 11 only** it first sets **B.y `+0x2d8` = 280**;
   - when the countdown is exactly 225, plays `SONAR` (`0x13c`, gap 3).

   A.x/B.x are untouched, so the reticle shows the true spawn column; the y write makes the alien appear at y 280 instead of the y rolled at the warning (`alien-sylvester.md`). Because this lives in the draw, the sim must apply it itself: once per tick while the countdown is in 1..225 is enough (225 ticks, many frames). The rolled y is still drawn from the RNG at the warning, so the RNG is unchanged.
3. **Hunger markers** (drawing only): `0x004d6980(t)` is true when the fish's hunger `+0x9c < t`, Blip is present, and (`+0x148 < 3` or `board+0x4ff`). The body draws then add `IMAGE_MISCITEMS` (global `0x005e8bc4`, name `0x0059cbb0`; 3 cels) **cel 2** at (0, −5). Threshold 500 for guppies (`Fish::DrawBody` `0x004dea00`), carnivores (`Oscar` `0x004e2480`), Ultravores (`0x004e3a10`), `BallFish` (`0x004db3b0`), `SylvesterFish` (`0x004e3420`) and the draws at `0x004dd280`, `0x004e15e0`, `0x004e2e30`; **900** in `0x004e19f0`. (`carnivore.md` and `fish-guppy.md` already describe the marker.) Hunger 500 is ~200 ticks before the hungry look at 301.
4. **Alien health bars** (drawing only): the alien draw (`0x004dae20`) adds a bar at (10, widget h − 3) for kind 0x15 always, and for every other kind while Blip is present (not on tank 5): `IMAGE_HEALTHBAR` (`0x005e8e3c`) cropped to width `trunc(barW · health / maxHealth)` (`+0x1f0 / +0x1f8`), clamped to 0..barW, then `IMAGE_HEALTHBARTUBE` (`0x005e8eb8`) over it. The Bilaterus head (`0x004e50b0`, only when its byte `+0x15e` is set) draws the same at (−20, h − 3) with width `trunc(barW · health / 100)`, mirrored (`barW − width`) when one of its state fields equals 4 (Ghidra `BilaterusHead_data.offset_0xc`; raw offset not resolved).

Fish, coins and food are otherwise unaffected.

`static`.

## Drawing (`0x004dfbb0`)

The generic pet draw (`pet-vert.md#drawing-0x004dfbb0`) with `IMAGE_BLIP` (global `0x005e8a14`, name at `0x0059c650`; `resources.xml`: 10 × 2 cels of 80 × 80, framedelay 2): row 1 while turning (mirrored iff turn > 0), row 0 otherwise (guppy mirror rule). No alien rows. `IMAGE_ZZZ` with the slow byte (Virtual Tank only). Type-0 shadow.

`static`.

## Open questions

- Implementation: the spawn-y write is a gameplay effect hidden in a draw; the sim should do it in its tick (document as a deviation in structure only, no behaviour change).
- The raw `+0x28` counter tested against 720 for a dropped-in Blip (likely the object's age; Nimbus's toss copies the same field for coins).
- Which classes own the draws at `0x004dd280`, `0x004e15e0`, `0x004e2e30` (500) and `0x004e19f0` (900); likely Gekko, Penta, Grubber and Breeder.
- `+0x148` and `board+0x4ff` in the hunger-marker test (from stage 3 on the marker needs `board+0x4ff`).
- Whether the shop reveal also shows its pop animation sound per button (`RefreshSlotButton` with popAnim 1).
- First capture: tank 3-5 or later with Blip chosen; check the shop at tick 201 and the alien spawn y (280).
