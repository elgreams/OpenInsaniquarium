# Meryl the mermaid (`Sexy::FishTypePet`, pet id 8)

Meryl swims like a guppy. Every 1400 calm ticks (39.2 s) she sings for 100 ticks (2.8 s). While she sings, every guppy that can drop coins drops one as soon as its coin timer reaches 35 instead of its own 150–349 interval: usually one coin at once and one or two more during the song. She is the pet hatched after tank 2-4 (`screens/hatch-screen.md#pet-award-table-adventure-first-play-through`) and can first appear on 2-5.

Same class, update and fields as Itchy (`pet-itchy.md`); only the id-8 differences are listed here. Raw `this`-relative offsets.

- Ctor `FishTypePet::FishTypePet` @ `0x004ef420`, id-8 branch `0x004ef57a`
- Abilities (slot 81) @ `0x004e5900`, id-8 branch `0x004e5de7`
- The song flag: global byte **`0x005e89cc`**, read only by the guppy coin drop `Fish::UpdateCoin` @ `0x004f1400`
- Animation (slot 89) @ `0x004dfc50` (blink draw at `0x004e000c`); draw body `0x004f8610` → `0x004df670`
- The note effect: a kind-16 `Coin` (`0x00544430` → `Coin::Coin` `0x004eead0`, update `0x004f5050` at `0x004f51e6`, draw `0x004dd7a0`)

Confidence per section. `zoo_b_zorf_meryl_wadsworth` (`tank1_1.rs`) replays 1-2 with Zorf, Meryl and Wadsworth from the game's own demo (`zoo_b_01`, 10,400 ticks, 7 songs) and compares the pets, guppies, food, coins, sounds and the app RNG every tick. Sections marked `static` (drawing, the Virtual Tank, Brinkley, Challenge) aren't covered.

**Correction to the brief this was written from:** the Adventure interval is **1400**, not 2700. The ctor stores `4320` in the Virtual Tank (`app+0x87c == 5`) and `4320 − 2920 = 1400` otherwise.

## Fields used by id 8

As Itchy's (`pet-itchy.md#fields-used-by-id-2`), plus:

| Offset | Type | Meaning |
|---|---|---|
| `+0x220` | int | song counter (the guppy coin timer), starts at 0 |
| `+0x224` | int | song interval: **1400** (Virtual Tank 4320) |
| `+0x244` | byte | blink flag. The ctor sets it to 1 for every FishTypePet; only id 8 uses it |

Divisor and bounds are the guppy's (`Fish::Init` draw; floor 370).

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Construction

`Board::AddPet(8, -1, -1)` (`0x00544a90`): the same **9 app draws** as Itchy (2 position, 6 `Fish::Init`, 1 shadow). The id-8 ctor branch only sets the interval `+0x224` (1400, or 4320 in the Virtual Tank), with no RNG. Blink flag `+0x244` = 1.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Update (`0x004f7ad0`), id 8

As Itchy's update (`pet-itchy.md#update-0x004f7ad0-id-2-normal-game`), with:

- **Step 3:** the behaviour gate (slot 82, `0x004e9790`) has no id-8 branch and returns false, so **the swim modes always run**, aliens or not (as Prego).
- **Step 6:** the song (below). Like every slot-81 ability, it is only called when there are no aliens (`+0xb8`) and no Bilaterus (`+0xf4`), and it returns at once on tank 5 (`board+0x3c8 == 5`) or while the board is paused. So the counter **freezes during a wave**.
- **Step 12, animation:** an extra RNG draw for blinking (below).

The re-roll draws (`r % 10`, then `r % 9 + 1` on 0, every 21 ticks) are the same as Itchy's.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## The song (slot 81, `0x004e5900`, id 8)

Each call (calm ticks only), with I = `+0x224`:

1. `counter (+0x220) += 1`.
2. If **`counter == I − 100`** (1300):
   - set the song flag **`0x005e89cc = 1`**;
   - **`AddCoin(wx + 15, wy − 5, kind 16, 0, −1.0, 0)`** (`0x00544430`): the rising music note (below);
   - play **`0x137` `SOUND_SING`** through `Board::PlaySound` (`0x00538230`, gap 3, volume 1.0; no RNG).
3. Else if **`counter ≥ I`** (1400): clear the flag (`0x005e89cc = 0`) and `counter = 0`.

So the first song starts on Meryl's 1300th calm update (36.4 s after she arrives) and the next ones every 1400 calm ticks. **No RNG** in this ability.

The flag is a single global, not per Meryl. With two Meryls (not possible in Adventure: one pet per id), whichever finishes first would clear it.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Effect on guppies (`Fish::UpdateCoin` @ `0x004f1400`)

The only reader of `0x005e89cc` in the game update. It is the guppy's coin drop (`coin.md#guppy-coin-drop-fishupdatecoin-slot-81-0x004f1400`, `fish-guppy.md`): `Sexy::Fish` slot 81, so it applies to **guppies only** (list `+0xa0`). Carnivores, ultravores, breeders and pets have their own slot 81 and ignore the flag (`carnivore.md` notes the same).

At `0x004f1425`: after `coin_timer (+0x220) += 1`, the interval passed to the drop test (`0x004d7100`) is **35 while the flag is set**, else the fish's own `+0x224`. Outside the Virtual Tank the test is just `coin_timer ≥ interval`. Everything else is unchanged:

- Small guppies (stage ≤ 0, `+0xac < 0`) return before the timer and never drop, song or not.
- The coin kind is still chosen by stage (silver for medium, gold for large, the gold-capable table for `+0x22a`), at `(wx + 5, wy + 10)`, with no RNG.
- With aliens present, guppies don't run this code (`coin.md`), so a wave during a song produces no coins. Meryl's counter is frozen too, so the flag stays set through the wave and the song's remaining ticks resume after it.

**Timing within the tick.** Guppies update before the `+0xb4` pets (`board.md#boardrebuildzorder-0x0053aa40-order-back--front`). Meryl sets the flag in her update of tick T and clears it in tick T + 100, so guppies see it on **ticks T + 1 … T + 100**: 100 guppy updates. In those:

- a guppy whose timer is already ≥ 34 drops on tick T + 1 (its timer is incremented first, then compared with 35). Since guppy intervals are 150–349, most guppies are past 34 and **drop a coin immediately when the song starts**;
- after a drop the timer is 0, so it drops again 35 ticks later: at T + 36 and T + 71 for a guppy that dropped at T + 1. **Usually 3 coins per medium/large guppy per song**, fewer for a guppy whose timer was low at the start;
- when the song ends the timer keeps counting toward the guppy's own interval from where it is (< 35). So the next normal coin comes a full interval later.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Other writers of the flag

- `Board::InitLevel` (`0x00541a20`, at `0x00541b12`) clears it, as do the reset routines `0x00537e20` and `0x0053c3f0`. A level never starts mid-song.
- `0x004d6250`, called from the FishTypePet destructor and from slot 74 (removal), clears it when the removed pet is id 8. Meryl leaving mid-song ends the song.
- The board's save/load routine `0x00546240` serialises the flag (`0x00546847`) and, after loading, clears it if the board's Meryl count `board+0x164` (pet counter `+0x144 + 4·8`) is 0.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## The note (`Coin` kind 16)

A `Coin` object used as an effect. `Coin::Coin` (no RNG) gives it the 72×72 widget; for kind 16 only, two widget flags (`+0x55`, `+0x7c`) are cleared where every other kind sets `+0x7c` (most likely the mouse flags, so the note can't be clicked: `guess`). `Board::AddGameObject` puts kind 16 on the **`board+0xe8` list** (`board.md`: "Coin(sub)", z-ordered right after the shadows, far behind the fish), not with the money coins `+0xa8`.

`Coin::Update` (`0x004f5050`), kind-16 path:

1. Animation and age counters as for any coin.
2. `+0x19c += 1` (age on this path). **Removed** when it passes 100, or when `y < 50` (the comparison is done before moving).
3. Otherwise `y −= 1.0`, widget = `trunc(x), trunc(y)`.

So it rises 1 px per tick from `(wx + 15, wy − 5)` for up to 100 ticks, and is cut off at y 50 near the top of the tank. No RNG, no sound, no money.

**Drawing** (`0x004dd7a0`): with `+0x1a0 < 2` (always, the ctor clears it) it draws `IMAGE_MISCITEMS` cel 1 at the widget origin. (Higher `+0x1a0` values switch to a coloured text draw that Meryl never uses.)

`static`.

## Animation (`0x004dfc50`), id 8

Turning is Itchy's. When not turning, id 8 takes the default branch (Itchy's): cycle `+0x1b8` += 1 (speed bucket ≤ 1) or 2, wraps at 20, cel = cycle / 2. Then the **blink check** (`0x004e000c`), only on non-turning ticks:

- If **cel == 0**: if the blink flag `+0x244` is set, clear it (no draw). Otherwise **one app draw `r % 10`** (`MTRand` at `app+0x7b0`, `0x0040aeb0`), and set the flag iff it is 0.

At swim speed 1 the cel stays 0 for two ticks (cycles 0 and 1); at speed 2 for one. So she makes **one or two app draws per swim cycle** whenever the flag is clear, **inside her update, after the re-roll and the song** and before integration. This is a per-tick RNG cost a replay must reproduce. The flag starts at 1, so her first cel-0 tick clears it without drawing.

`traced` (`zoo_b_zorf_meryl_wadsworth`).

## Drawing (`0x004df670`)

`IMAGE_MERYL`, 80×80 cels at `(cel·80, row·80)`, cel = the swim cel:

| when (first match) | row | mirror |
|---|---|---|
| turning (`+0x1c4 ≠ 0`) | 2 | iff turn > 0 |
| singing: `counter > I − 100` | 1 | guppy rule |
| cel < 5 and blink flag set | 3 | guppy rule |
| otherwise | 0 | guppy rule |

Row 1 shows for counter 1301 … 1399 (99 frames; the frame of tick T still shows row 0, since the counter is 1300 then). The blink row 3 covers the first half of a swim cycle after a successful blink draw, and at speed 1 it is cleared again on the next tick (cel still 0), so a blink then lasts one frame. A type-0 shadow follows her like a guppy's. `IMAGE_MERYLBLINK`, `MERYLLIPS` and `MERYLTAIL` are not used by this draw (probably the Virtual Tank or the hatch screen).

`static`.

## Open questions

- Confirm the blink flag clears on the second cel-0 tick at speed 1 (a capture with Meryl will show whether blinks last 1 frame or half a cycle).
- Whether the note is clickable and whether Stinky ever looks at the `+0xe8` list (it shouldn't: it isn't a money coin).
- The interval in the brief (2700) didn't match the code; the 1400/4320 values come from the ctor at `0x004ef57f`.
