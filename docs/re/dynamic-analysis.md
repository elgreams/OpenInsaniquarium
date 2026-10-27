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
| Update counter | `app+0x484` (logic updates; `+0x47c` counts frames that slept) |
| App MTRand | `[app+0x7b0]`; state words `+0x000..+0x9bc`, `mti` `+0x9c0` |
| Global MTRand | `0x005eb928` |

## Known hazards

- **The game crashed (access violation at `0x004068c1`)** ~1–2 minutes after attaching during the first session. The faulting function (`0x00406880`) calls `VirtualQuery` + `GetModuleFileNameA` and then parses a module's PE header. That's PopCap's crash-reporter/anti-tamper module walk, and it probably tripped over memory CE allocated or injected. Mitigations to try:
  - Avoid `allocateMemory`/`autoAssemble`/`executeCode` in the target. Stick to reads, and to hardware breakpoints when needed.
  - Do bulk reads in a single Lua call rather than many small ones.
  - If it recurs with read-only use, find what triggers the walk (likely a periodic check) and note it here.
- **CE dialogs freeze the bridge.** When CE attaches to a Proton process it shows a modal "Proton target" dialog, and the trial build periodically shows a "Patreon or Trial" nag. Either one blocks CE's main thread, and every MCP call then times out (30 s). Dismiss them (for the nag, click the button whose number the dialog names) before relying on the bridge.
- **Focus loss pauses the game.** Anything that takes focus (CE's trial nag, CE's window) puts the board into "Game Paused". Captures record `board.paused` and the replay harness trims to the running segment. Scripted clickers should read the pause flag (published by `ce_capture.lua` via `CAPTURE_GPOS`) and click Resume at (312, 258).
- `ce_capture.lua` returns after the MCP call times out (30 s). That's expected for long captures: the Lua keeps running, and the output file fills to `CAPTURE_TICKS + 1` lines.
- **Catch-up bursts tear captures.** After a stall the game runs several logic updates back to back (up to the 200 ms cap, `framework/timing.md`), faster than the capture's 5 ms poll. Snapshots inside a burst can miss updates or straddle one (objects read before an update, board fields after). Detect it by the board clock `+0x28` advancing differently from the capture tick (`harness::replay::first_burst`) and don't compare across it. Captures key snapshots on `app+0x47c` (frames that slept) and, since 2026-10-26, also record the logic update count `app+0x484` as `u` (`framework/demo-recording.md#tick-basis`). (Earlier notes had the two swapped.) Replays re-key by the board clock, so either works within a level.
- Long-running Lua loops inside one MCP call can drop the bridge connection ("connection reset"). The Lua usually finishes anyway; write results to a file and read them back.

## Recording a pet zoo (any pets, any level)

How `zoo_a_01` and `zoo_b_01` were made (2026-10-26). RE tooling only.

1. Back up `userdata/` to `re/backup/`, then `tools/set_profile.py <user1.dat> --level T-L` and delete `adv1.dat`.
2. `tools/run_inner_proton.sh -record`, attach CE to the `game.exe -changedir…` process.
3. In CE, set the profile `[app+0x8b8]`:
   - owned flags `+0x00..+0x13` = 1, count `+0x18` = 20;
   - chosen `+0x5a + id` = 1 for the 3 pets (the rest 0).
4. Go to the level through Adventure. The help screen (1-2) starts it via StartLevel, which takes `+0x5a` as is. A level started earlier must be restarted (Menu → Restart Level); set the chosen bytes again just before.
5. Run `ce_capture.lua` (`CAPTURE_GPOS=/tmp/gpos`) and `tools/autoplay_tank11.py`. Captures cap at `CAPTURE_TICKS`; start a second one for long sessions and join them.
6. Quit through the menu, so the game writes `popcap.dmo`. Save it as `traces/demos/<name>.dmo` and the capture as `traces/original/<name>.trace.jsonl.zst`. Restore the save.

Replay with `replay_level(name, level, true)` in `tank1_1.rs`, which takes the inputs from the demo.

### The working routine (as used for `gash_snack_01`, `amp_zap_01`)

Keep each recording **short and focused on one ability**: one capture of ~2800 ticks (about 80 s), pushing a slow timer with CE rather than waiting for it.

1. Backup + `set_profile.py --level 1-2` + delete `adv1.dat`; launch with `-record`; `open_process` on the `game.exe -changedir…` pid.
2. One `evaluate_lua`: owned bytes `+0x00..+0x17` = 1, count `+0x18`.
3. Clicks always with a hold time: `tools/orig_click.sh 318 431 150` (title "Click here to play"), `sleep 4`, `tools/orig_click.sh 465 90 150` (Adventure), `sleep 4`, then a screenshot (`tools/orig_shot.sh /tmp/now.png`) to confirm the Instructions screen.
4. **One `evaluate_lua` that sets the chosen bytes and starts the capture** (`for i=0,23 … writeBytes(prof+0x5a+id,1); CAPTURE_OUT=…; CAPTURE_TICKS=2800; CAPTURE_GPOS="/tmp/gpos"; dofile(…ce_capture.lua)`). It times out after 30 s; that's expected. The chosen bytes must be set **on the Instructions screen**: entering Adventure clears them (NextLevel).
5. `sudo rm -f /tmp/gpos; sleep 1; tools/orig_click.sh 318 431 150` (Click Here To Continue starts the level), then autoplay in the background with `--max-seconds` a bit under the capture length.
6. Push timers with CE while it runs (see below), check progress with `grep ^P /tmp/gpos`.
7. Quit: kill autoplay; `orig_click.sh 573 17 150` (Menu), `sleep 1.5`, `314 300` (Back to Main Menu), `sleep 1.5`, `244 285` (Leave), screenshot. From the main menu `556 426` (Quit), `sleep 2`, `244 258` (Quit), `sleep 8`; the game process is gone and `popcap.dmo` has a new timestamp.
8. Save `popcap.dmo` → `traces/demos/<name>.dmo`, the capture → `traces/original/<name>.trace.jsonl.zst` (drop a torn last line first), restore the save, delete `adv1.dat`.

### Common hiccups

- **Guppies starve in the first seconds:** the capture was started *after* the start click. The `dofile` call blocks the MCP for 30 s, and nobody feeds the tank meanwhile (1-2's guppies start at hunger ~450–550 and starve in ~16 s). Always start the capture on the Instructions screen (step 4), then click start and launch autoplay **in the same Bash call** (step 5). Size `CAPTURE_TICKS` for the ~1000 help-screen ticks before the level (1-2's first wave comes ~1750 board ticks in: use ~4200 to include it).

- **CE "not responding" / every MCP call times out:** a capture is running. `ce_capture.lua` holds CE's Lua engine for its whole length, so CE can't answer until it finishes. Keep captures short (≤ 3000 ticks), and wait for the file to reach `CAPTURE_TICKS + 1` lines before the next CE call. To write memory meanwhile, write `/proc/<pid>/mem` as root (`sudo python3`, see `/tmp`-style helper: seek to the address, write a little-endian int): same effect as CE, no Lua needed.
- **A second `dofile(ce_capture.lua)` while one runs** queues behind it: the second capture starts only when the first ends. Don't overlap them.
- **No pets in the tank after starting:** the chosen bytes were written before entering Adventure (cleared) or on a previous level. Write them on the Instructions screen, just before the start click.
- **Game over** (all guppies died: Brinkley eats their food, potions turn them into noses, no feeding): GAME OVER dialog; click Continue at **(312, 258)** → main menu. Quit from there (step 7's second half). Feed the guppies (autoplay's default or `--hunger N`) unless the ability needs them hungry, and keep the recording short.
- **Leaving a level writes `adv1.dat`**: the next Adventure resumes mid-level with the old pets. Delete it before re-entering.
- **Danger dialog** (1-2's first wave, ~1750 ticks in): autoplay clicks its button; if autoplay has stopped, click (315, 271).
- **CE broken pipe / connection reset** in `/tmp/ce.log`: the bridge dropped. Restart CE (`sudo pkill -9 -x cheatengine-x86`, not `-f`, which also matches your own shell), relaunch, `open_process` again.
- **The capture file is root-owned**: copy it before editing; its last line is often torn when the game quits.

**Long sessions in several capture chunks:** each `ce_capture.lua` run writes its own header with the full RNG state at its start. When joining chunks into one trace, keep each later chunk's header as a snapshot line carrying `"rng"` (or store the chunks as separate traces). Without it, a chunk after a hole can't be replayed, because the RNG can't be walked across ticks nobody captured. `zoo_e_01` lost its later chunks' headers, so only its first chunk (ticks 2538–11155) is replayable.
