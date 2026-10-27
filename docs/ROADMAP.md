# Roadmap

Guiding principle: **get one tank provably 1:1 end-to-end before going wide.** The hard, risky parts (timing, RNG, update order, trace capture) are proven on a small slice; everything after that is mostly content.

Two tracks run in parallel throughout: **RE** (specs in `docs/re/`) stays roughly one milestone ahead of **implementation**.

Each milestone has an exit criterion. Don't start the next one until it's met.

---

## M0 — Foundations & recon

Answer the questions that shape everything else.

- [x] Commit CLAUDE.md, .gitignore; scaffold the Cargo workspace (empty crates, CI with fmt/clippy/test).
- [x] Ghidra: import `Insaniquarium.exe`, apply RTTI class names, recover vtables for `GameObject` and its subclasses. *(The exe is a DRM stub; the inner PE was extracted, see `framework/drm-wrapper.md`.)*
- [x] **Timing:** find the main loop / `SexyAppBase::UpdateApp`; confirm tick rate and how many updates run per frame. → `docs/re/framework/timing.md` *(28 ms, confirmed live)*
- [x] **RNG:** identify the algorithm (MSVC `rand()` vs. `MTRand`), seeding, number of streams, where the state lives. → `docs/re/framework/rng.md`
- [x] **Demo recording:** `-record` works and the format is parsed (`harness::demo`). Original `-play` is broken in this build (`framework/demo-recording.md`); capture uses CE polling instead.
- [x] **Object model:** map `Board` and its object lists (fish, coins, food, aliens, pets, shots); find `Board::Update` and record the update order. → `docs/re/sim/board.md`
- [x] Survey of the asset formats in `resources.xml`, the fonts in `data/*.txt`, `fishsongs/*.txt`, the save/profile files, and `.mo3`. → `docs/re/assets/`

**Exit:** we know the tick rate and the RNG algorithm, and we've chosen a trace capture strategy (CE polling, demo files, or both).

## M1 — Asset pipeline & viewer

Load the user's install and show things on screen. Mostly framework, no gameplay.

- [x] `sexy-core`: game-data path resolution (`--data` / env), install validation, case-insensitive lookup. *(Config file deferred until we have config.)*
- [x] `resources.xml` parser: groups, `SetDefaults`, ids, and load-time attributes.
- [x] Image loading with alpha-mask compositing (`foo.gif` + `_foo.gif`), sprite-sheet cel layout (rows/cols). Cel animation (`AnimInfo`).
- [x] ImageFont descriptor parser + text rendering (kerning, line spacing, outline variants). *(Single-layer layout; multi-layer draw order TODO when needed.)*
- [x] Audio: `.ogg`, `.au`, `.mo3` (libopenmpt loaded at runtime; optional).
- [x] Asset browser (`cargo run --bin asset-browser -- --data <install>`; `--shot ID out.png` for headless screenshots).

**Exit:** every resource in `resources.xml` loads and displays correctly in the browser, matching the original by eye.

## M2 — Differential harness

Build the proof machinery before the gameplay it will prove.

- [x] Versioned trace format (initial state, input events, per-tick snapshots). *(`crates/harness/README.md`; inputs are inferred from captured state.)*
- [x] Capture tool: CE-driven field polling per tick (`tools/ce_capture.lua`, raw object bytes + RNG state).
- [x] Replay runner and field-by-field diff that reports the first divergent tick and field.
- [x] RNG unit tests: our RNG reproduces sequences captured from the original.

**Exit:** a capture of the original's RNG/state can be replayed and diffed by our sim, even if all that's in the sim so far is the RNG and the clock.

## M3 — Vertical slice: Tank 1-1

The smallest real game, provably 1:1.

- [x] Sim: `Board`, guppy (hunger, growth stages, coin drops, death), food (drop, sink), coins (types, fall, collect), money, shop (guppy, egg pieces), mercy guppy. *(Food-quality/quantity upgrades and DeadFish visuals deferred: not on 1-1.)*
- [x] Sim: the first alien (Sylvester): spawn timing, targeting, health, click damage. *(Tank 1-2, with Stinky and the food-quality upgrade. `full_tank_1_2_replays`: `tank1-2_full_02`, level start to the third egg piece, ~6200 ticks, two waves, bit-exact apart from the documented effect-shot deviation.)*
- [x] App: tank render, click-to-feed/collect, shop bar, money counter, help arrows, sounds, music, win condition (egg pieces). *(`cargo run --bin tank`. Hatch screen is M4.)*
- [x] Trace-based tests for all of the above. *(`crates/harness/tests/tank1_1.rs`: 6 captures, ~16k ticks, incl. a full level.)*

**Exit:** a recorded play of Tank 1-1 on the original replays in our sim with zero divergence for its full length. ✅ *`tank1-1_full_01`: 5222 ticks from level start (after the mercy guppy) to the third egg piece, 48 inferred inputs, bit-exact (app RNG, all guppy/food/coin fields, money, shop). Played by `tools/autoplay_tank11.py`.*

## M4 — Adventure mode complete

Mostly content now; the hard problems are solved.

- [ ] All fish types: Carnivore, Ultravore, Starcatcher, Guppycruncher, Beetlemuncher, Breeder.
- [ ] All aliens + Cyrax boss; shots/missiles; boss-specific mechanics.
- [ ] All 24 pets and their unlock flow. *(Status 2026-10-27: 19 of the 20 Adventure pets built and replayed exactly against the original (`docs/re/INDEX.md`, pet zoos `zoo_a`..`zoo_e`, `gash_snack`, `amp_zap`); Presto (19) is specced, not built. Bonus pets 20–23 not specced; plan in `docs/re/objects/bonus-pets-plan.md`.)*
- [ ] Store, upgrades, egg pieces, per-level config for tanks 1–4.
- [ ] Screens: title, menu, story/interlude, hatch, bonus, game over, help, options.
- [ ] Profiles and save data — format compatible with the original if feasible.

**Exit:** Adventure is playable start to finish; trace tests cover every fish, alien, and pet.

## M5 — Other modes

- [ ] Time Trial, Challenge, high scores.
- [ ] Virtual Tank: shells, fish/decoration store, sim-tank behavior (`SimFishScreen`, `SimSetupScreen`), fish songs (`fishsongs/*.txt` player).
- [ ] Hidden extras and secret codes.

**Exit:** feature parity with the original's main menu.

## M6 — Screensaver

- [ ] `--screensaver` mode: Virtual Tank fullscreen, exit on input, multi-monitor behavior per the original.
- [ ] Settings parity with `ScreenSaverDialog`.
- [ ] Windows `.scr` launcher (`/s`, `/c`, `/p`); `/p` preview pane is a stretch goal.
- [ ] Linux: xscreensaver / generic idle-hook integration.

**Exit:** installs and runs as a screensaver on Windows and Linux.

## M7 — Modding platform

Turn the faithful engine into a foundation for other games. Modding works in layers. Forking the engine works from day one (MIT/Apache, clean code); data packs come next. Scripting is optional.

- [ ] **Forking:** keep game rules (`insaniquarium-sim`) separate from engine plumbing (`sexy-core`, app) so a fork can swap rules without touching the rest. Document how to build your own game on the engine.
- [ ] **Data packs (main mod path):** move hardcoded fish/alien/pet/level/shop definitions into data files (TOML/RON). The original becomes the "base" content pack. Content folder with an OpenMW-style load order, so mods override files and stack. User-asset overrides (sprites, sounds).
- [ ] **Scripting (optional, decide later):** only if data packs plus forking leave a real gap (stackable new *behaviours*). Lua is the likely choice if we do it; Rhai is the alternative. Sandboxed, with a stable API.
- [ ] Remove limits as opt-in options: resolution and widescreen, tank size, object caps.
- [ ] Example mod + modding docs.

**Exit:** a non-trivial new fish and level ship as a mod without touching engine code, and the base game still passes all trace tests.

## M8 — Release

- [ ] Packaging for Windows, Linux, and macOS; first-run setup that points the engine at a user install.
- [x] Decide the license: MIT OR Apache-2.0, like Bevy.
- [ ] User docs, contributor docs, a public spec index.

---

## Open questions

- Tick rate and RNG algorithm (M0).
- Is the demo recording functional? If not, how much CE instrumentation will capture need?
- Does the screensaver build's sim differ from `Insaniquarium.exe`, or is it the same code?
- Save-file compatibility: worth it, or import-only?
- Scripting for mods: maybe never needed (see M7). Lua if so.
