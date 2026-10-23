# Music (`music/*.mo3`)

Status: `draft` (static). Decoding is verified: the system libopenmpt 0.7.3 loads all three modules (`Insaniq2.mo3`: 67 orders, 113.7 s).

## Format

All music is **MO3**: un4seen's compressed tracker-module container, played by BASS (`bass.dll`, `BassMusicInterface`, vtable `0x005c11bc`). libopenmpt decodes MO3 natively, including MP3/Vorbis-compressed samples, so we use it rather than reimplementing a tracker.

## Tracks

`WinFishApp::LoadingThreadProc` (`0x005503c0`) loads the modules into the music interface (`app+0x374`, slot 1 = `LoadMusic(id, path)`):

| Track id | File |
|---|---|
| 1 | `music\Alien.mo3` |
| 3 | `music\Lullaby.mo3` |
| 4 | `music\Insaniq2.mo3` |

Ids **0** and **2** are the original game's separate "in-tank" and "menu" songs. In this build they are **remapped to track 4** (`Insaniq2.mo3`) at a translated **order offset** whenever the global flag `0x005e28cc` is set (`WinFishApp::RemapMusicTrack` @ `0x0054b090`, `0x0054b150`). The Deluxe build packs every song into one module and starts it at different orders.

Remap table (id, order → 4, order):

| Requested | Becomes |
|---|---|
| 0, 22 | 4, 0 |
| 0, 0 | 4, 12 |
| 2, 32 | 4, 25 |
| 2, 45 | 4, 37 |
| 2, 0 | 4, 45 |
| 2, 54 | 4, 49 |
| 2, 5 | 4, 50 |
| … | (remaining cases in `0x0054b090`; extend when needed) |

## Selection & transitions

- **Tank music** (`WinFishApp::PlayTankMusic` @ `0x0054c390`): chosen by tank number (`board+0x3c8`, or the profile when there's no board): tank 1 → (0, 22), tank 2 → (0, 0), tank 3 → (2, 32), tank 4 → (2, 45). Each then goes through the remap above. Virtual Tank (mode 5) stops all music instead.
- **Fade-out** before switching: speed **0.008** per update (`0x005a1a00`).
- **Alien music** (`0x0054c480`): fade the tank song out at 0.008 per update, then `0x00498d10(1)` (slot 3) sets track 1's volume to its maximum and clears its fade, and `PlayMusic(1, −1, 0.002, loop)` resumes it with fade speed 0.002. That "fade-in" starts from the maximum and is clamped there, so **the alien song plays at full volume at once**, over the tank song's 125-update fade-out.
- **Stop all** (`0x0054b020`): stops ids 0–4 and resets the current-track fields (`app+0x88c` id, `app+0x890` order).
- **PlayMusic** (interface slot 11, `0x004992e0`): `(id, order, fade_speed: f64, no_loop: bool)`. Order −1 means "resume". Otherwise BASS seeks to the order. Looping is on unless `no_loop`.

Fades are linear: `BassMusicInterface` slot 19 (`0x004997d0`) adds the fade speed to the volume, clamps it to [0, max], and stops a song that faded out to 0. It is called from `SexyAppBase::UpdateApp` (`0x004802d0`), so once per 28 ms logic tick.

**Songs are module positions, not files.** Looping is the module's own: BASS plays with repeat −1, so the order list decides what repeats. Measured with libopenmpt:
- The tank 1 song from order 0 plays orders 0–11 once (≈ 113 s), then loops orders 2–11 (an intro, then the loop section).
- The hatch song from order 49 plays 49 once, then loops 45–48.
- `Alien.mo3` loops its single section.

A pre-rendered fixed-length loop gets this wrong (it restarts the intro). The app streams the module live (`insaniquarium_app::song`).

## Implementation notes

- `sexy_core::music` loads `libopenmpt.so.0` / `libopenmpt.dll` **at runtime** (`libloading`), so building doesn't require libopenmpt. Without the library, music is silently unavailable.
- Order seeking uses `openmpt_module_set_position_order_row(order, 0)`. Looping uses `set_repeat_count(-1)`.
- Starting a song = a new player from its start order (`PlayMusic` seeks). The tank→alien switch fades the tank song out (0.008/update) while the alien song starts at full volume; killing the last alien does the reverse (`PlayTankMusic(1)`: fade-out of the alien song, the tank song from its start order at full volume). Entering the hatch screen or a new level stops all music and starts that screen's song at full volume.
