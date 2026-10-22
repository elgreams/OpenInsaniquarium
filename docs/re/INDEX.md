# Reverse-engineering spec index

Every behavior spec lives under `docs/re/<area>/`. See CLAUDE.md → "Clean-room workflow" for conventions.

Status: `todo` · `draft` (static analysis only) · `confirmed` (verified dynamically) · `traced` (covered by a differential test)

| Area | Spec | Status | Notes |
|---|---|---|---|
| framework | `framework/drm-wrapper.md` | confirmed | The exe is a DRM stub; the real game is an unencrypted inner PE |
| framework | `framework/timing.md` | confirmed | **28 ms ticks (~35.7 Hz)**, 200 ms catch-up cap |
| framework | `framework/rng.md` | confirmed (algorithm/seeding) | MT19937 (31-bit output); app stream + global stream + store stream. Implemented: `sexy_core::rand::MtRand` |
| meta | `dynamic-analysis.md` | — | How to attach CE to the Proton game; known crash hazard |
| framework | `framework/dialog.md` | draft | In-tank message boxes (`MoneyDialog`): panel as a 3×3 tiled box, fonts, colours, size/position, the board dialogs' texts |
| framework | `framework/demo-recording.md` | draft | `-record`/`-play` functional; tick-indexed input bitstream |
| objects | `objects/fish-guppy.md` | confirmed (idle swim + animation) | Guppy fields, ctor RNG order (6 draws), swim modes, 21-tick reroll, animation/turn/cel rules, draw-sheet selection. Replays bit-exact vs original |
| objects | `objects/pet-stinky.md` | confirmed (tank 1-2 pre-alien) | Stinky the snail (OtherTypePet id 0): ctor draws, floor wander with 21-tick reroll, coin chase/collect (banks + points-sound draw), shell while aliens, animation |
| objects | `objects/alien-sylvester.md` | confirmed (tank 1-2 replays) | First alien (kind 1, tank 1-2+; none on 1-1): wave countdown (1750 → warning at 275 with 4 position draws), spawn (3 draws), chase/wander/eat, shot damage 3×laser and 9-zone knockback, death debris, DeadAlien; heap-dependent RNG caveats |
| objects | `objects/coin.md` | confirmed | Guppy coin drop (kind = stage, at fish+(5,10)), fall 1.5 to y 370, floor rest 150 (tutorial) / 20, 5-tick expiry, collect glide to (550,30), values, collect-sound RNG, ambient bubble burst draws |
| objects | `objects/food.md` | confirmed (tank 1-1 pellets) | Food drop (click bounds, price, limit, 1 RNG draw), pellet fall/fade, guppy hunger, chase, nearest-food, bite boxes, eat (+700, cap 800, growth), slurp RNG throttle. Replays bit-exact vs original |
| sim | `sim/level-start.md` | confirmed (tank 1-1) | StartLevel → InitLevel → PopulateLevel: Board ctor (3 draws), start bubbles, owned pets, two guppies via AddGuppyAt (growth preset 2). 1-1 start replays exactly |
| sim | `sim/shop-bar.md` | draft (static; guppy purchase confirmed) | Slot/position model, per-level layout and prices (1-1: guppy $100, egg $150), stage-up reveals, mouse-down purchase, TrySpend with pending coins, egg pieces → level complete, button art and money label |
| sim | `sim/board.md` | draft | Type codes → typed vectors; update order = z-order rebuild sequence |
| screens | `screens/hatch-screen.md` | draft (static) | Last egg piece → record/advance profile/unlock pet/save/destroy board → HatchScreen: pet-award table (1-1 → Stinky), 141-tick egg/crack/shard timeline, pet reveal + texts, Continue (t ≥ 170, mouse-up) → NextLevel → StartLevel; 2 app draws per hatch tick |
| assets | `assets/images.md` | draft | Extension probe order, `_x`/`x_` mask lookup, alphaimage/alphagrid, GIF transparency, AnimInfo cel math. Implemented: `sexy_core::image` |
| assets | `assets/music.md` | draft | MO3 via libopenmpt; one module, songs selected by order; tank/alien selection + fades. Implemented: `sexy_core::music` |
| assets | `assets/sprite-variants.md` | draft | Mirroring is draw-time and per cel (`DrawImageMirror`), never baked into textures; `SCL_*` images are shipped pre-scaled files; 4 derived images built at load time (crease tiles, tank/title cut-outs, 40×40 pet thumbnails via box-filter stretch) |
| assets | `assets/overview.md` | draft | Survey: resources.xml, ImageFont, fishsongs, music, saves. Split into per-format specs as each is implemented |
