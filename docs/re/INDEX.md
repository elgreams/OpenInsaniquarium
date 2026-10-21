# Reverse-engineering spec index

Every behavior spec lives under `docs/re/<area>/`. See CLAUDE.md → "Clean-room workflow" for conventions.

Status: `todo` · `draft` (static analysis only) · `confirmed` (verified dynamically) · `traced` (covered by a differential test)

| Area | Spec | Status | Notes |
|---|---|---|---|
| framework | `framework/drm-wrapper.md` | confirmed | The exe is a DRM stub; the real game is an unencrypted inner PE |
| framework | `framework/timing.md` | confirmed | **28 ms ticks (~35.7 Hz)**, 200 ms catch-up cap |
| framework | `framework/rng.md` | confirmed (algorithm/seeding) | MT19937 (31-bit output); app stream + global stream + store stream. Implemented: `sexy_core::rand::MtRand` |
| meta | `dynamic-analysis.md` | — | How to attach CE to the Proton game; known crash hazard |
| framework | `framework/demo-recording.md` | draft | `-record`/`-play` functional; tick-indexed input bitstream |
| objects | `objects/fish-guppy.md` | confirmed (idle swim + animation) | Guppy fields, ctor RNG order (6 draws), swim modes, 21-tick reroll, animation/turn/cel rules, draw-sheet selection. Replays bit-exact vs original |
| objects | `objects/food.md` | confirmed (tank 1-1 pellets) | Food drop (click bounds, price, limit, 1 RNG draw), pellet fall/fade, guppy hunger, chase, nearest-food, bite boxes, eat (+700, cap 800, growth), slurp RNG throttle. Replays bit-exact vs original |
| sim | `sim/board.md` | draft | Type codes → typed vectors; update order = z-order rebuild sequence |
| assets | `assets/images.md` | draft | Extension probe order, `_x`/`x_` mask lookup, alphaimage/alphagrid, GIF transparency, AnimInfo cel math. Implemented: `sexy_core::image` |
| assets | `assets/music.md` | draft | MO3 via libopenmpt; one module, songs selected by order; tank/alien selection + fades. Implemented: `sexy_core::music` |
| assets | `assets/sprite-variants.md` | draft | Mirroring is draw-time and per cel (`DrawImageMirror`), never baked into textures; `SCL_*` images are shipped pre-scaled files; 4 derived images built at load time (crease tiles, tank/title cut-outs, 40×40 pet thumbnails via box-filter stretch) |
| assets | `assets/overview.md` | draft | Survey: resources.xml, ImageFont, fishsongs, music, saves. Split into per-format specs as each is implemented |
