# Reverse-engineering spec index

Every behavior spec lives under `docs/re/<area>/`. See CLAUDE.md → "Clean-room workflow" for conventions.

Status: `todo` · `draft` (static analysis only) · `confirmed` (verified dynamically) · `traced` (covered by a differential test).

The status column is a summary. **Confidence is marked per section in each spec**, and that is what counts: a `traced` spec can still have `static` sections (other levels, other kinds). `tools/spec_confidence.py` prints the breakdown, and the `spec_confidence` test fails on any untagged section.

| Area | Spec | Status | Notes |
|---|---|---|---|
| framework | `framework/drm-wrapper.md` | confirmed | The exe is a DRM stub; the real game is an unencrypted inner PE |
| framework | `framework/timing.md` | confirmed | **28 ms ticks (~35.7 Hz)**, 200 ms catch-up cap |
| framework | `framework/rng.md` | traced (app stream) | MT19937 (31-bit output); app stream + global stream + store stream. Implemented: `sexy_core::rand::MtRand` |
| meta | `dynamic-analysis.md` | — | How to attach CE to the Proton game; known crash hazard |
| framework | `framework/presentation-capture.md` | draft (sounds confirmed) | Read-only anchors for sound (board sample table), music (BASS interface track map, requested track/order), open dialogs and screen widgets; the harness's presentation comparison |
| framework | `framework/dialog.md` | traced (hint timing); drawing confirmed | In-tank message boxes (`MoneyDialog`): panel as a 3×3 tiled box, fonts, colours, size/position, the board dialogs' texts |
| framework | `framework/demo-recording.md` | draft | `-record`/`-play` functional; tick-indexed input bitstream |
| objects | `objects/fish-guppy.md` | traced (tank 1) | Guppy fields, ctor RNG order (6 draws), swim modes, 21-tick reroll, animation/turn/cel rules, draw-sheet selection; `DeadFish` (sink/fade). Replays bit-exact vs original |
| objects | `objects/pet-stinky.md` | traced (tank 1-2) |
| objects | `objects/pet-niko.md` | traced (tank 1-3) | Niko (pet 1): pearl every ~69 s (timer 1233/1450 + `r % 50`), updates first (behind the overlay), the pearl coin (kind 15, list `+0xec`, $250) | Stinky the snail (OtherTypePet id 0): ctor draws, floor wander with 21-tick reroll, coin chase/collect (banks + points-sound draw), shell while aliens, animation |
| objects | `objects/alien-sylvester.md` | traced (kind 1); drawing static | First alien (kind 1, tank 1-2+; none on 1-1): wave countdown (1750 → warning at 275 with 4 position draws), spawn (3 draws), chase/wander/eat, shot damage 3×laser and 9-zone knockback, death debris, DeadAlien; heap-dependent RNG caveats |
| objects | `objects/coin.md` | traced (kinds 1, 2, 4) | Guppy coin drop (kind = stage, at fish+(5,10)), fall 1.5 to y 370, floor rest 150 (tutorial) / 20, 5-tick expiry, collect glide to (550,30), values, collect-sound RNG, ambient bubble burst draws |
| objects | `objects/carnivore.md` | traced (purchase, diamond, starving); eating static | Carnivore (`Oscar`, type 5, `+0xa4`): buy = 13 app draws (ctor adds hunger `%200+600`), hunts the nearest small guppy (no food), bite box, eat (+900 cap 1300, chomp + gore draws, no growth), diamond every `+0x224` ticks, dies at hunger 0, 1-3 death-hint chain (ids 0x1b/0x1c/0x1d) |
| objects | `objects/food.md` | traced (levels 0–1) | Food drop (click bounds, price, limit, 1 RNG draw), pellet fall/fade, guppy hunger, chase, nearest-food, bite boxes, eat (+700, cap 800, growth), slurp RNG throttle. Replays bit-exact vs original |
| sim | `sim/level-start.md` | traced (tank 1) | StartLevel → InitLevel → PopulateLevel: Board ctor (3 draws), start bubbles, owned pets, two guppies via AddGuppyAt (growth preset 2). 1-1 start replays exactly |
| sim | `sim/shop-bar.md` | traced (1-1/1-2 slots); later levels static | Slot/position model, per-level layout and prices (1-1: guppy $100, egg $150), stage-up reveals, mouse-down purchase, TrySpend with pending coins, egg pieces → level complete, button art and money label |
| sim | `sim/board.md` | traced (tank-1 objects, backdrop, message line) | Type codes → typed vectors; update order = z-order rebuild sequence; backdrop (waves, light bands, cut-out) and the message line (`MessageWidget`, one-time hints) — both confirmed by frame diff |
| screens | `screens/hatch-screen.md` | traced (update/RNG); drawing confirmed | Last egg piece → record/advance profile/unlock pet/save/destroy board → HatchScreen: pet-award table (1-1 → Stinky), 141-tick egg/crack/shard timeline, pet reveal + texts, Continue (t ≥ 170, mouse-up) → NextLevel → StartLevel; 2 app draws per hatch tick |
| assets | `assets/images.md` | confirmed (frame diff) | Extension probe order, `_x`/`x_` mask lookup, alphaimage/alphagrid, GIF transparency, AnimInfo cel math. Implemented: `sexy_core::image` |
| assets | `assets/music.md` | traced (tank 1) | MO3 via libopenmpt; one module, songs selected by order; tank/alien selection + fades. Implemented: `sexy_core::music` |
| assets | `assets/sprite-variants.md` | confirmed (mirroring, cut-out) | Mirroring is draw-time and per cel (`DrawImageMirror`), never baked into textures; `SCL_*` images are shipped pre-scaled files; 4 derived images built at load time (crease tiles, tank/title cut-outs, 40×40 pet thumbnails via box-filter stretch) |
| assets | `assets/overview.md` | draft | Survey: resources.xml, ImageFont, fishsongs, music, saves. Split into per-format specs as each is implemented |
