# CLAUDE.md

Guidance for Claude (and humans) working in this repository.

## What this project is

An **OpenMW-style, clean-room reimplementation of Insaniquarium Deluxe 1.1** in Rust, using Bevy for presentation. The goal is a faithful engine that plays *identically* to the original (same inputs → same frames, same outcomes), which then serves as a foundation for mods and entirely new games that aren't bound by the original's limits.

**Scope:** main game (Adventure, Time Trial, Challenge), Virtual Tank, and the screensaver.

**Non-goals:**
- Byte-matching or instruction-matching decompilation. We match *behavior*, not code.
- Shipping, embedding, or redistributing any original game content.
- Building for PvZ now. A future PvZ engine may reuse the `sexy-*` framework crates (both games are built on PopCap's SexyApp framework), so keep those crates game-agnostic — but don't add speculative PvZ features.

**License:** not yet decided. Do not add license headers or a LICENSE file until it is.

## Bring your own assets (hard rule)

Users must supply their own legally obtained copy of Insaniquarium Deluxe. The engine reads assets from that install at runtime.

- **Never commit** anything from the original game: images, sounds, music, fonts, `resources.xml`, fishsongs, executables, or derived/converted versions of them.
- **Never commit** raw Ghidra decompiler output, disassembly dumps, Ghidra project files, or Cheat Engine tables. These live in gitignored dirs (`re/ghidra/`, `re/ce/`, `traces/`).
- Test fixtures that need assets must locate them via the game-data path (see below) and **skip** when it's absent — never vendor them.
- Synthetic test data we author ourselves (e.g. a hand-written tiny `resources.xml`, a 4×4 test PNG) is fine to commit.

The local reference install lives at `original Game/` (gitignored). The engine locates game data via, in order: `--data <path>` CLI flag → `INSANIQUARIUM_DATA` env var → config file. Do not hardcode `original Game/`.

## Reference target

All RE work targets this exact build. If a finding came from a different build, say so.

| File | SHA-256 |
|---|---|
| `Insaniquarium.exe` (= `InsaniquariumDeluxe.exe`, byte-identical) | `f54c2c6ee54b00ae6da7f4bde15feb90d0a867aba6b4151b0277c1f81da3ff66` |
| `WinFish_Scr.exe` (screensaver build of the game) | `c904b14cfbf985043aca01b7d77bc06d0fd24101800e49fa6fc374ce5b92b878` |
| `Insaniquarium.scr` (screensaver launcher stub) | `beceb69377a570d5aa07ccc1784f2e1768e05ca21abdf59d68c02610b23a0bf8` |

Known facts about the binary:
- PE32, i386, MSVC. `.text` at `0x401000`, `.rdata` at `0x516000`, `.data` at `0x554000`. Not packed.
- **RTTI is intact** (~560 classes). Class names like `Sexy::Board`, `Sexy::Fish`, `Sexy::Alien`, `Sexy::Gekko`, `Sexy::Ultra` are recoverable — use them.
- Framework: PopCap SexyApp (`Sexy::` namespace). Audio via BASS (`bass.dll`); music is `.mo3`.
- Embedded Lua 5.1 is used by the **PopCap DRM wrapper only** (`drm/common/scripts/main.lua`), not by gameplay. DRM is out of scope — don't reimplement it.
- SexyApp demo record/playback strings exist (`-record`, `-demofile`, `-recnum`, `-playnum`). **Unverified** whether functional in this build — if so, it's a major lever for differential testing.

## Clean-room workflow

The pipeline is **binary → spec → code**. Code is written from specs, not from decompiler output.

1. **Investigate** with Ghidra (static) and Cheat Engine (dynamic). MCP tools for both are available (`mcp__ghidra__*`, `mcp__cheatengine__*`).
2. **Write a spec** in `docs/re/<area>/<topic>.md` describing *behavior* in our own words: state fields and their meaning, update order, constants, formulas, timers, RNG calls, edge cases. Pseudocode is fine; pasted/lightly-edited decompiler output is not.
3. **Implement** in Rust from the spec. Idiomatic Rust, our own structure and names.
4. **Verify** against the original via the differential harness (below).

Spec conventions:
- Cite original addresses for traceability: `Fish::Update @ 0x0041A2B0`. Addresses are facts, not copyrightable expression.
- Give exact constants with units and the tick basis (e.g. "hunger decrements by 1 per tick; starves at 0, initial 500").
- Note every RNG call site and its order within the tick — this is what makes or breaks determinism.
- Mark confidence: `confirmed` (verified dynamically), `static` (read from code only), `guess`.
- One spec per class/system. Keep `docs/re/INDEX.md` listing every spec and its status.

Ghidra conventions:
- Rename functions as `Class::Method` using RTTI names where possible; `FUN_xxxxxxxx` → meaningful names as you learn them.
- Add decompiler comments for constants and branches you've confirmed.
- Ghidra project files stay in `re/ghidra/` (gitignored). Spec docs are the shareable artifact.

Cheat Engine conventions:
- Use CE to confirm runtime values, struct layouts, and per-tick changes; to set breakpoints on writes to fields; and to capture traces.
- Don't leave hooks or patches running in a session after you're done; clear breakpoints.

## Architecture

Cargo workspace. Crate boundaries are load-bearing — respect them.

```
crates/
  sexy-core/        # Game-agnostic SexyApp-equivalent framework (no Bevy):
                    #   resource manifest (resources.xml), image+alpha-mask compositing
                    #   (`foo.gif` + `_foo.gif`), ImageFont descriptor parser, widget/
                    #   dialog model, RNG(s), fixed-tick clock, input event model.
  insaniquarium-sim/# Deterministic game simulation (no Bevy, no I/O, no wall clock).
                    #   Board, GameObjects (fish, pets, aliens, coins, food, shots),
                    #   store, levels, profiles/save data, virtual tank sim.
  insaniquarium-app/# Bevy frontend: windowing, rendering, audio (.ogg/.au/.mo3),
                    #   input → sim events, screens/UI, screensaver mode.
  harness/          # Differential testing: trace format, replay runner, diff tool.
tools/              # Dev utilities (asset inspectors, trace capture helpers).
docs/re/            # Behavior specs from reverse engineering (committed).
re/                 # Local RE workspace (gitignored subdirs).
traces/             # Captured traces from the original (gitignored).
```

Rules:
- **`insaniquarium-sim` must not depend on Bevy.** It must be runnable headless in tests. Bevy is presentation only.
- `sexy-core` must not depend on anything Insaniquarium-specific.
- The app reads sim state and sends input events; it never mutates sim state directly.

## Determinism rules (the core of 1:1 fidelity)

- **Fixed timestep.** The sim advances in discrete ticks at the original's update rate (SexyApp default is 100 Hz — **verify** in the binary and record in `docs/re/framework/timing.md`). Rendering interpolates or just shows the latest tick; it never drives logic.
- **Explicit ordered object lists.** The original iterates objects in insertion order, and order affects outcomes (who eats the food first, which coin gets collected). Use `Vec`-backed ordered storage that mirrors the original's lists. Never rely on `HashMap` iteration order or Bevy query order for game logic.
- **Bit-exact RNG.** Reimplement the original's RNG algorithm exactly (MSVC `rand()` LCG vs. PopCap `MTRand` — determine which, and whether multiple streams exist). Every call site must consume RNG in the same order as the original.
- **Match numeric types.** If the original uses `int` with truncating division, or `float` (f32) rather than `double`, do the same. x87 vs. SSE precision differences may matter — document when they do.
- No wall-clock time, threads, or OS randomness inside the sim.

## Differential testing harness

The proof of fidelity is: given the same initial state and input sequence, our sim's state matches the original's tick by tick.

- **Capture** from the original using Cheat Engine (read known struct fields each tick) and/or SexyApp's demo recording if it works.
- **Trace format:** per-tick snapshots of relevant state (object lists with positions/velocities/timers, money, RNG state) plus the input event log. Keep the format versioned and documented in `crates/harness/README.md`.
- **Replay** the same inputs through `insaniquarium-sim` and diff field by field. Report the first divergent tick and field.
- Traces derived from the original live in `traces/` (gitignored). Tests that use them must skip if absent.
- Every system ported should land with at least one trace-based test once capture is possible.

## Build & run

```sh
cargo build                                  # whole workspace
cargo test                                   # unit + headless sim tests
cargo run -p insaniquarium-app -- --data "/path/to/Insaniquarium Deluxe"
cargo run -p insaniquarium-app -- --data ... --screensaver
```

Linux dev builds should enable Bevy's dynamic linking for faster iteration (configure via a cargo feature, not by default for releases).

## Code style

- Rust edition 2024, toolchain pinned in `rust-toolchain.toml` (Bevy 0.20 needs rustc ≥ 1.97.1). `cargo fmt` and `cargo clippy --all-targets` clean.
- Name things after the original's RTTI/class names where it aids cross-referencing with specs (`Board`, `Coin`, `Gekko`), but structure the code idiomatically — don't mirror C++ inheritance. Prefer enums + data for object kinds.
- Comments explain *why* or cite a spec (`// see docs/re/objects/fish.md#hunger`). Don't paste addresses everywhere — the spec holds them.
- Original behavior that looks like a bug is reproduced faithfully and documented as such. Fixes go behind an explicit, off-by-default option (mods/QoL), never silently.
