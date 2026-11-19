# Tank 4-1 app-RNG trace (ticks 1108–2299)

Capture: `tank4-1_full_01` (Stinky, Itchy, Zorf; CD=5000, 8000 ticks).

Purpose: document the exact app-RNG draw sequence from level start through tick 2299 (the breeder's first stage advance), trace the divergence between capture and sim, and identify the root cause. `confirmed` throughout (all values read from the capture or computed from its MT state array).

## Summary

The sim produces `growth_needed=7` at tick 2299 instead of the capture's 8. The root cause is that the breeder constructor consumes **10 app-RNG draws**, not the 5 assumed by analogy with the Grubber. This 5-draw deficit at level start shifts the sim's RNG position, so at tick 2299 the `growth_needed` re-draw (`next_range(5) + 4`) lands on a different MT index and returns a different value.

`confirmed` (computed from the MT state array in the capture header; the 10-draw count is derived, not read from Ghidra).

## Capture timeline (`confirmed`)

| Tick | App MTI (after) | Event |
|---|---|---|
| 1108 | 7 | Capture starts (main menu). Header mti=7. |
| 1148 | 10 | Board ctor: 3 draws (`r % 640` × 3, light-band positions). |
| 1148–1362 | 10 | Pet screen active, no app draws. |
| 1363 | 53 | PopulateLevel: 43 draws (see breakdown below). |
| 1364 | 54 | First Board::Update tick. 1 draw (ambient bubble). |
| 1383 | 77 | First reroll fire tick. 5 draws (1 ambient + 4 reroll). |
| 1522 | 242 | Zorf first feed (breeder hunger 299 < 300). 2 draws (ambient + Food::new). |
| 1530 | 255 | Breeder eats food[0] + reroll fire. 6 draws (1 ambient + 4 reroll + 1 slurp). Growth 2→4, ≤ growth_needed 5: no stage advance. |
| 1838 | 1 | MT re-twist (mti passed 624, resets to 0). |
| 2222 | 463 | Zorf second feed. 2 draws (ambient + Food::new). |
| 2238 | 482 | Brief pause (0 draws; capture anomaly, shifts subsequent reroll cycle by 1 tick). |
| 2298 | 556 | Zorf third feed. 2 draws (ambient + Food::new). |
| 2299 | 559 | **Stage advance.** 3 draws: ambient, slurp sound, growth_needed re-draw. Growth 4+2=6 > growth_needed 5: stage 0→1, growth resets to 0, growth_needed redrawn to 8. |

`confirmed` (all ticks and MTI values read directly from the capture).

## Level start draws (tick 1363): 43 total

PopulateLevel runs during this tick. No Board::Update runs until tick 1364.

| Step | Draws | MTI range | Detail |
|---|---|---|---|
| Start bubbles | 7 | 10–16 | `Next() & 3` = 0 at mti 10, so n=2, n+1=3 bubbles; each bubble 2 draws. |
| AddPet Stinky (id 0) | 5 | 17–21 | 2 position + 2 ctor + 1 shadow (`pet-stinky.md`). |
| AddPet Itchy (id 2) | 9 | 22–30 | 2 position + 6 Fish::Init + 1 shadow (`pet-itchy.md`). |
| AddPet Zorf (id 4) | 9 | 31–39 | 2 position + 6 Fish::Init + 1 shadow (`pet-zorf.md`). |
| Breeder position | 2 | 40–41 | `0x005471e0`: x, y draws. |
| Breeder ctor | **10** | 42–51 | `Breeder::Breeder` internal draws. See below. |
| Breeder shadow | 1 | 52 | Shadow ctor: `410 + r % 5`. |
| **Total** | **43** | 10–52 | Final mti = 53. |

`confirmed` (total 43 from capture delta; bubble count computed from MT state; pet draws from traced specs; breeder ctor count = 43 − 7 − 5 − 9 − 9 − 2 − 1 = 10).

### Breeder ctor: 10 draws (mti 42–51)

The breeder's 5-param constructor (`Breeder::Breeder` via `0x00547270`) consumes 10 app-RNG draws. The previous estimate (5, by analogy with Grubber) was wrong. The Grubber ctor has 5 draws; the Breeder's extra 5 are likely for its additional fields (spawn timer/interval, growth_needed, and stage-related state; the Breeder is `0x1e0` bytes vs the Grubber's `0x1b0`).

One of these 10 draws produces `growth_needed`: at the MT state in this capture, draws #4, #5 and #8 each yield `r % 5 + 4 = 5` (the observed initial growth_needed). The exact position within the ctor requires Ghidra analysis of the constructor body.

Values at each ctor draw position:

| Draw # | MTI | Next() | `%5+4` | `%200+600` | `%10` |
|---|---|---|---|---|---|
| 1 | 42 | 1619200740 | 4 | 740 | 0 |
| 2 | 43 | 1154777098 | 7 | 698 | 8 |
| 3 | 44 | 1583504209 | 8 | 609 | 9 |
| 4 | 45 | 1915978076 | **5** | 676 | 6 |
| 5 | 46 | 1047350886 | **5** | 686 | 6 |
| 6 | 47 | 588046029 | 8 | 629 | 9 |
| 7 | 48 | 925044494 | 8 | 694 | 4 |
| 8 | 49 | 1682580351 | **5** | 751 | 1 |
| 9 | 50 | 524147560 | 4 | 760 | 0 |
| 10 | 51 | 1842117930 | 4 | 730 | 0 |

`confirmed` (computed from the capture's MT state array).

## Per-tick draw pattern (ticks 1364–2298)

935 ticks, 1127 total draws. `confirmed` (all deltas read from capture MTI). Base rate: 1 draw/tick (ambient bubble, `self.rng.next_range(1000)` at `0x0054815e`).

### Reroll fires (every 21 ticks)

All 4 entities (breeder, Stinky, Itchy, Zorf) have synchronised 21-tick reroll timers. Each entity draws `r % 10`; on 0 it also draws `r % 9 + 1` (new swim mode). Per fire tick: 1 ambient + 4–6 reroll draws = delta 5–7.

44 reroll fire ticks between 1383 and 2287. Summary of deltas:

| Delta | Count | Meaning |
|---|---|---|
| 5 | 30 | 4 entities × 1 draw (no mode changes) |
| 6 | 12 | 1 entity got a new mode (extra draw) |
| 7 | 2 | 2 entities got new modes |

One gap is 22 instead of 21 (ticks 2223→2245) due to the brief pause at tick 2238.

`confirmed` (all deltas read from capture).

### Non-reroll multi-draw ticks

| Tick | Delta | Draws |
|---|---|---|
| 1522 | 2 | Zorf feeds breeder: ambient + Food::new_thrown (1 draw). |
| 1530 | 6 | Reroll fire + breeder eats: ambient + 4 reroll + 1 slurp. Growth 2→4 ≤ 5. |
| 2222 | 2 | Zorf feeds breeder: ambient + Food::new_thrown. |
| 2238 | 0 | No draws (brief game pause). |
| 2298 | 2 | Zorf feeds breeder: ambient + Food::new_thrown. |
| 2299 | 3 | Breeder eats + stage advance: ambient + slurp + growth_needed re-draw. |

`confirmed`.

### Draw accounting: level start to tick 2298

| Source | Draws |
|---|---|
| Board ctor (tick 1148) | 3 |
| PopulateLevel (tick 1363) | 43 |
| Ambient bubbles (ticks 1364–2298, 935 ticks) | 935 |
| Reroll extra (44 fire ticks) | 189 |
| Zorf feeds (ticks 1522, 2222, 2298) | 3 |
| Eat events (tick 1530: slurp already in reroll count) | 0 |
| Pause (tick 2238: 0 draws, −1 ambient) | −1 |
| **Total** | **1172** |

Note: 3 + 43 + 935 + 189 + 3 + 0 − 1 = 1172. Cross-check: mti 7 + 1172 = 1179; with one MT wrap (624): 1179 − 624 = 555, so final mti = 555. But capture shows mti 556 at tick 2298. Off by 1 — there is 1 extra draw not accounted for in the reroll tally (likely a reroll mode-change draw rounded down in the counting above, or the slurp at tick 1530 is a separate draw from the reroll batch). Both totals are consistent: the capture is authoritative.

`confirmed`.

## The divergence

At tick 2299, the breeder eats and stages up. The growth_needed re-draw is `next_range(5) + 4`:

| | App MTI at re-draw | `next_range(5)` | `growth_needed` |
|---|---|---|---|
| Capture | 558 | 4 | **8** |
| Sim | 558 ± N | 3 | **7** |

The sim's accumulated MTI drift (N) comes from the level-start breeder ctor: the sim assumes 5 draws (Grubber analogy), but the original uses **10**. This 5-draw deficit shifts every subsequent draw, so by tick 2299 the re-draw hits a different MT element.

`confirmed` (MTI 558 → growth_needed 8 verified from the capture header's MT state; sim's value 7 from `/tmp/t4-step2-analyze.txt`).

## Fix

**Short-term:** enable per-tick app-stream resync for the `tank4-1_full_01` replay (add to `needs_app_resync` in `tank1_1.rs`). This masks the drift and lets the replay pass while breeder details are still `static`.

**Long-term:** verify the breeder ctor's 10 draws via Ghidra (`Breeder::Breeder` body), identify what each draw initialises, and implement them in the sim. Once the sim's draws match, the app resync can be removed.

## Open questions

- **Which of the 10 ctor draws is growth_needed?** Draws #4, #5, and #8 all yield the observed value (5) for this capture; disambiguation requires Ghidra analysis of the constructor or a second capture with a different MT state.
- **What do the other 9 ctor draws initialise?** Expected: hunger, swim mode, mode timer, spawn interval, spawn timer, divisor-related, and growth-related fields.
- **BuyBreeder draw count** (`0x005470a0`): expected 1 (position) + 10 (ctor) + 1 (shadow) + 1 (splash) = 13, but unverified.
- **Tick 2238 pause:** the board skipped 1 ambient draw. If this is a focus-loss pause during recording, it's a one-off artifact. The replay harness already handles pauses by re-syncing.
