# Plan: the bonus pets (ids 20–23) and Presto

Status: plan only, no claims about the game. What is already known is cited. `static`.

## What we know

- Ids **20 BRINKLEY the Scuba Diving Elephant**, **21 NOSTRADAMUS the Nose**, **22 STANLEY the Startlingly Small Sea Serpent**, **23 WALTER the Penguin** have hatch-screen names and text but are **not awarded by egg pieces**: they're unlocked by `0x005172e0`: they are **bought with shells on the Bonus screen**, one per purchase in id order, for 20 000 / 25 000 / 30 000 / 35 000 shells (`screens/bonus-screen.md`, `static`).
- All four are `FishTypePet` (AddPet makes an `OtherTypePet` only for ids 0, 1, 5, 7, 14).
- Code paths already seen for them in other pets' specs (all `static`):
  - **20 Brinkley:** board counter `board+0x194`; Zorf drops pellets every 2nd check while he is in (`pet-zorf.md#feeding`); type-1 shadow (`pet-amp.md#construction`); shares Nimbus's animation branch (`pet-nimbus.md#animation`).
  - **21 Nostradamus:** an extra ctor draw `r % 300 + 300` into `+0x224` (`004ef420` id 0x15 branch); slot 81 id 0x15 makes something at (x+15, y+10) via `0x005434e0` every `+0x224` ticks; re-roll also fires when y > floor − 50 (`pet-itchy.md` step 5); Amp's zap removes id-21 pets with `+0x240 > 0` (`pet-amp.md#the-zap`); Update makes him count `+0x1c`/puff like a Presto pet.
  - **22 Stanley:** ctor sets `+0x224 = 100`; gate/abilities id 0x16 branches: a counter `+0x34` (0..10) that rises with aliens, a periodic action through slot 86 (`0x004e55d0`, `0x00544130`, sample `0x122`); probably attacks aliens.
  - **23 Walter:** ctor makes him mouse-visible (`+0x55 = 1`, `+0x28 = 1`, insets 10/15); a punch (slot `+0x20` widget, the "boxing glove") that turns a Presto pet back into Presto (`pet-presto.md`); `+0x238` countdown at 0 sets `+0x28 = 1`.
- Presto (19) is fully specced (`pet-presto.md`, static).

## Steps

`static` (a plan). Same workflow as pets 0–18 (`dynamic-analysis.md#recording-a-pet-zoo-any-pets-any-level`): spec → build → zoo recording replayed from the demo → promote to `traced`. One agent writes and commits each spec as it finishes (`feedback_usage_budget` memory).

1. **Presto (19).** Spec exists. Build:
   - demo replay: right-button events (`button: Right`, codes −1/−2) → a new `Input::RightClick { x, y }`; today they're "unsupported";
   - the PRESTO CHANGE-O dialog (lists owned pets; a pick is a click on the dialog) as a board dialog;
   - the replacement: remove the pet, `AddPet(n, wx, wy, flag 1)` with no position draws, recharge 360, puff 0, the "Presto" label;
   - record: 1-2 with Presto, right-click him, pick 2–3 different pets in turn.
2. ~~Find the unlock~~: done, `screens/bonus-screen.md`. For recordings, the CE profile edit (owned bytes 20–23) is enough.
3. ~~Spec 20–23~~: done (`pet-brinkley.md`, `pet-nostradamus.md`, `pet-stanley.md`, `pet-walter.md`, all `static`). Each spec ends with its own recording plan, which supersedes step 4 below where they differ (e.g. Stanley and Walter need the global MT state captured; Stanley's missile shoot-down needs alien kinds 5/6, a later tank).
4. **Build** them and record one zoo: 1-2 with Brinkley + Nostradamus + Stanley (Walter needs Presto: record him with Presto in a second session). Push timers with CE where an ability is slow, inside one capture chunk (`dynamic-analysis.md`: keep each chunk's header when joining).
5. Promote specs, update `INDEX.md`, `ROADMAP.md`.

## Pitfalls already met

- Chunked captures lose the RNG header: keep each chunk's header or record within one chunk.
- The autoplay buys egg pieces by default (`--no-egg` for zoos), collects coins (`--no-coins`), feeds high (`--feed-low`); `--zap-amp N` clicks a charged Amp.
- The replay adopts recorder (CE) writes to money and to a fish pet's `+0x220` timer; any other CE write needs the same treatment.
- Long sessions run out of memory (earlyoom killed the session at ~10.7 GB): start a fresh session per pet set.

`confirmed` (each was hit in a session on 2026-10-26/27).
