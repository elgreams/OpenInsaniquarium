# PopCap DRM wrapper & the inner game binary

Status: `confirmed` (structural, from file inspection).

`Insaniquarium.exe` / `InsaniquariumDeluxe.exe` (SHA-256 `f54c2c6e…`) is **not the game**. It is a PopCap DRM launcher: its own `.text` holds the SexyApp-based DRM UI (Lua-scripted, see `drm/common/scripts/main.lua`) and a Steam check (`steam.dll`, `SteamIsAppSubscribed`). The actual game is a complete, **unencrypted** PE32 stored in the file's overlay:

- Marker `!popcapdrmprotect!` at file offset `0x188000`
- Inner PE (`MZ`) at file offset `0x1881a2`, size `0x212000` (2,170,880 bytes)
- After it: an Authenticode signature blob

| | SHA-256 |
|---|---|
| Inner game PE (extracted) | `0c0928b12be736be76a97a2a83597525dcd76d250c552cb341708a58db53edad` |

Inner PE layout: image base `0x400000`, entry `0x567f2e`. Sections: `.text` `0x401000` (size `0x1948fc`), `.rdata` `0x596000`, `.data` `0x5df000`, `.rsrc` `0x623000`. 251 RTTI vtables, including all gameplay classes (`Board`, `Fish`, `Alien`, `Coin`, `Food`, `GameObject`, `WinFishApp`, …).

`WinFish_Scr.exe` has the same section layout and identical `.text` size, but differs in ~179 KB of bytes. It's a separate build of the same game (screensaver variant), not a byte copy. Analyze it separately when we reach M6.

## Workflow

`tools/extract_inner_pe.py <Insaniquarium.exe> re/bin/Insaniquarium_game.exe` copies the inner PE out verbatim for static analysis. The Ghidra project at `re/ghidra/Insaniquarium` contains both programs. **All RE addresses in `docs/re/` refer to the inner PE unless stated otherwise.**

We do not reimplement, bypass, or document how to bypass the DRM. Extraction is only for reading the game's code. Dynamic analysis runs the game normally through Steam.

At runtime the wrapper maps the inner PE at its preferred base `0x400000`, so the addresses should match the live process. TODO: confirm with CE once attached.
