# OpenInsaniquarium

An open-source, clean-room reimplementation of **Insaniquarium Deluxe 1.1** in Rust (Bevy for presentation), in the spirit of OpenMW. The goal is an engine that plays *identically* to the original (same inputs, same outcomes), verified tick by tick against the real game. It's meant as a base for mods and for new games that aren't bound by the original's limits.

**You need your own copy of Insaniquarium Deluxe.** This repository contains no game assets. The engine reads them from your install at runtime.

## Status: preview

**v0.1.0 is an early preview.** Adventure's first tank (levels 1-1 to 1-5, with pets) plays from the main menu; tank 1 and all 24 pets are tested tick by tick against recordings of the original. There are no saves yet, and tanks 2–5 and the other modes are still to come. See [`CHANGELOG.md`](CHANGELOG.md) and [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Playing

1. **You need your own copy of Insaniquarium Deluxe**, installed (for example from Steam). OpenInsaniquarium ships none of the game's pictures, sounds or music; it reads them from your install. Please don't share game files.
2. Download the archive for your system from [Releases](https://github.com/elgreams/OpenInsaniquarium/releases) and unpack it anywhere.
3. Run `openinsaniquarium` (`openinsaniquarium.exe` on Windows).

It finds the game by itself if it's in a Steam library (any drive, Flatpak too). Otherwise pick one:

- **Drag and drop:** copy the game's files (the folders `images`, `sounds`, `properties`, `data` and the rest) into the `gameData` folder next to the program.
- **Point to it:** open `data-path.txt` next to the program and put the game folder's path on a line of its own.
- **Command line:** `openinsaniquarium --data "/path/to/Insaniquarium Deluxe"` (or set `INSANIQUARIUM_DATA`).

If it still can't find the game, it says where it looked and what to do.

The folder is the one that contains `images`, `sounds` and `properties`. On Linux, music needs libopenmpt (package `libopenmpt0` or similar); without it the game plays silently.

## Building

```sh
cargo run --bin openinsaniquarium -- --data "/path/to/Insaniquarium Deluxe"
cargo run --bin asset-browser -- --data "/path/to/Insaniquarium Deluxe"
cargo test
```

## Layout

- `crates/sexy-core`: game-agnostic SexyApp-style framework (resources, images, fonts, RNG, audio decoding).
- `crates/insaniquarium-sim`: the deterministic game simulation (no Bevy, no I/O).
- `crates/insaniquarium-app`: Bevy frontend.
- `crates/harness`: differential testing against captures of the original.
- `docs/re/`: behaviour specs written from reverse engineering.

## License

Except where noted, all code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.

Insaniquarium is a trademark of its owners. This project is not affiliated with or endorsed by PopCap Games or Electronic Arts, and it ships none of their content.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
