# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-10-29

The first preview: Adventure's first tank with its pets, playable from the main menu. It's an early look, not a finished game.

You need your own copy of Insaniquarium Deluxe. This download contains no game files; OpenInsaniquarium reads the pictures, sounds and music from your install.

### What you can play

- **Adventure, tank 1** (levels 1-1 to 1-5): feeding, coins, the shop, egg pieces, aliens and the laser, the hatch screen and the pet you win, and choosing pets between levels once you own more than three. After 1-5 the preview ends and you go back to the menu.
- **Main menu**: Start Adventure and Quit. The other buttons are shown but don't do anything yet.
- **In-game Menu button**: opens the Options dialog and pauses the game; OK resumes, Back to Main Menu leaves (asking "Leave Game?" once your fish have eaten).
- **Game Paused** when the window loses focus; Resume Game continues.
- **GAME OVER** returns to the main menu, as in the original.
- Music and sound effects.
- **Finding your game:** it looks in every Steam library by itself. Otherwise copy the game's files into the `gameData` folder that comes with the download, or put the game's folder path into `data-path.txt` next to the program.

### How close it is to the original

- Tank 1 (all five levels) and all 24 pets are tested against recordings of the original game: the same clicks give the same fish, coins and aliens, tick by tick.
- The menus, Options, Leave Game? and Game Paused were built from the original's code and checked by eye against screenshots of it. They aren't tested pixel for pixel yet.
- Every known difference is written down in the specs (`docs/re/`).

### Known missing or different

- **No saves or profiles.** Progress lasts until you quit, and you aren't asked for a name. ("Your game will be saved" in Leave Game? isn't true yet.)
- No title screen, and no Instructions screen before the first levels.
- Options: the volume sliders, checkboxes and Help don't do anything yet.
- Tanks 2–5, bonus rounds, Time Trial, Challenge, Virtual Tank, Hall of Fame and the screensaver aren't in this build.
- The main menu's mermaid doesn't blink or move her tail, and the menu has no "Welcome back" bubble.
- Linux and Windows only (x86-64). On Linux, music needs libopenmpt from your distribution (e.g. `libopenmpt0`); without it the game is silent but plays. The Windows download includes libopenmpt.

[0.1.0]: https://github.com/elgreams/OpenInsaniquarium/releases/tag/v0.1.0
