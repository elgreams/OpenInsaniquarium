# Dynamic analysis setup (Linux / Proton)

How to attach Cheat Engine to the running original for confirmation and trace capture.

## Launch

1. Launch the game through Steam (`steam steam://rungameid/3320`). The game requires `steam.dll` and won't start standalone under plain Wine.
2. The DRM wrapper (`Insaniquarium.exe`) writes the inner game PE to
   `<compatdata>/3320/pfx/drive_c/ProgramData/PopCap Games/Insaniquarium/popcapgame1.exe`
   and runs it as a child process. That file is byte-identical to `tools/extract_inner_pe.py` output (plus the trailing signature). **Attach to `popcapgame1.exe`**, not the wrapper.
3. The inner PE loads at its preferred base `0x400000`, so Ghidra addresses match live addresses one-to-one.

### Running with SexyApp flags (`-record`, `-play`, …)

The DRM launcher passes only `-changedir` to the game, so flags given to Steam never arrive. Use `tools/run_inner_proton.sh <flags>`. It copies the extracted inner PE into the Proton prefix and runs it directly. The game still runs in Steam's Proton environment, but without the launcher.

- Windowed mode: set `ScreenMode` to `0` under `[Software\\PopCap\\Insaniquarium]` in `<compatdata>/3320/pfx/user.reg`. Back up `user.reg` first.
- `-record` writes `C:\ProgramData\Steam\Insaniquarium\popcap.dmo` on exit. `-play` reads `C:\ProgramData\PopCap Games\Insaniquarium\popcap.dmo`.
- Recording a session also updates save files (`C:\ProgramData\Steam\Insaniquarium\userdata\`). Back them up first: `re/backup/`.

## Cheat Engine

- Native Linux CE at `~/tools/cheatengine`, with the MCP bridge autorun (`autorun/zz_mcp_bridge.lua`) listening on the abstract unix socket `@CE_MCP_Bridge_v99`.
- `kernel.yama.ptrace_scope = 1`, so CE must run as root to attach:
  `cd ~/tools/cheatengine && sudo -E ./cheatengine-x86_64`
- Root-owned files written from CE Lua (e.g. `/tmp/*.txt`) are readable but owned by root.

## Key live anchors

| What | Where |
|---|---|
| App (`WinFishApp*`) | `[0x005eb680]` |
| `mFrameTime` | `app+0x454` (28) |
| Update counter | `app+0x47c` |
| App MTRand | `[app+0x7b0]`; state words `+0x000..+0x9bc`, `mti` `+0x9c0` |
| Global MTRand | `0x005eb928` |

## Known hazards

- **The game crashed (access violation at `0x004068c1`)** ~1–2 minutes after attaching during the first session. The faulting function (`0x00406880`) calls `VirtualQuery` + `GetModuleFileNameA` and then parses a module's PE header. That's PopCap's crash-reporter/anti-tamper module walk, and it probably tripped over memory CE allocated or injected. Mitigations to try:
  - Avoid `allocateMemory`/`autoAssemble`/`executeCode` in the target. Stick to reads, and to hardware breakpoints when needed.
  - Do bulk reads in a single Lua call rather than many small ones.
  - If it recurs with read-only use, find what triggers the walk (likely a periodic check) and note it here.
- **CE dialogs freeze the bridge.** When CE attaches to a Proton process it shows a modal "Proton target" dialog, and the trial build periodically shows a "Patreon or Trial" nag. Either one blocks CE's main thread, and every MCP call then times out (30 s). Dismiss them (for the nag, click the button whose number the dialog names) before relying on the bridge.
- Long-running Lua loops inside one MCP call can drop the bridge connection ("connection reset"). The Lua usually finishes anyway; write results to a file and read them back.
