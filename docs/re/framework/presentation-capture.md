# Presentation state and events (sound, music, dialogs, screens)

Status: sounds `confirmed` (every replay test compares them against the captures' sample tables); music, dialog and screen fields `draft` (static; untested live). Read-only anchors that let a capture record what the player *hears* and *which screen and dialog is up*, so the harness can compare them tick by tick with what our app does. See `crates/harness/README.md` ("Presentation").

## Sound effects

Every board sound goes through `Board::PlaySample(id, gap, volume)` @ `0x00538230`. If the board clock `+0x28` is at least `gap` ticks past the sample's last play, it stores the clock in **`board+0x1a4 + 4·(id − 0x106)`** (64 slots) and starts the sound. The eat, slurp, points and splash helpers (`0x00538560`, `0x005384c0`, `0x005383d0`, `0x00538360`) use the same table through `0x005381f0`.

So **a sample played on tick t ⇔ its slot changed on tick t**, and captures already record the table (`board.samples`). A new value in slot `i` means sample `0x106 + i` started that tick. The stamp is the board clock, or the clock − 1 for a sound played during input handling (before that tick's update increments the clock). Variants share their family's slot: SLURP/2/3 stamp 0x138, POINTS/2/3/4 stamp 0x12c, CHOMP/2 stamp 0x113.

This is exact for every throttled sound, which is all of them on the board except:

- `SexyAppBase::PlaySample` (slot 54 @ `0x004896b0`) and `+0xd8` calls bypass the table: the alien arrival roar (`0x00545928`), `SOUND_HATCH` / `SOUND_EVILLAFF` on the hatch screen, and button clicks. These are logged from the screen state instead (the spawn and the hatch counter), or not at all.
- The volume argument isn't recorded. Only the dead-alien rumble varies it (`alien-sylvester.md#73`).

Sample ids are `0x105` + the index of the `<Sound>` in `resources.xml` (`Sound::sample_id`).

First catches (fixed): buying a guppy plays `SOUND_GROW` (0x122), not BUY; a starving guppy plays `SOUND_DIE`; the ambient bubble burst plays `SOUND_BUBBLES` (0x110, `0x005481bf`).

## Music

`[app+0x374]` is the `BassMusicInterface`. Its track map (an MSVC `std::map<int, MusicInfo>`) has its header at `+8`. Nodes have left `+0`, parent `+4`, right `+8`, key (track id) `+0x10`, and isnil byte `+0x41`. Per track:

| Offset | Meaning |
|---|---|
| `+0x18` / `+0x1c` | BASS music / stream handle |
| `+0x20` | volume (f64) |
| `+0x28` | fade speed per update (f64; > 0 fading in, < 0 fading out, 0 none) |
| `+0x30` | max volume (f64, 1.0) |
| `+0x38` | stop when faded out (byte) |

The app records the last requested track and order in **`app+0x88c` / `+0x890`**. That is the pre-remap request (e.g. (0, 22) for tank 1, (2, 54) for the hatch screen; `assets/music.md`). `StopAllMusic` sets them to (−1, …).

Not recorded: the module's playback position (BASS-internal). Looping and section order are checked against libopenmpt offline (`assets/music.md`).

## Dialogs

`SexyAppBase` keeps open dialogs in a `std::map<int, Dialog*>` at `app+0x31c` (header pointer at `+0x320`; nodes: key `+0xc`, dialog `+0x10`, isnil `+0x15`; `0x0048a630`). The set of keys is the open-dialog ids, e.g. `0x0e` hint, `0x10` last fish, `0x11` game over (`framework/dialog.md`).

## Screens

Screen widgets the app owns (non-null while shown):

| Field | Screen | Set by |
|---|---|---|
| `app+0x730` | Board (tank) | `0x0054c540` |
| `app+0x814` | HatchScreen | `0x0054be80` |
| `app+0x80c` | PetScreen | `0x0054c2d0` |
| `app+0x81c` | InterludeScreen | `0x0054bd80` |
| `app+0x818` | BonusScreen | `0x0054bf60` |
| `app+0x738` | GameSelector (menu) | `0x00552100` |
| `app+0x834` | HelpScreen | `0x0054c220` |

The hatch screen's own state comes from the HatchScreen object: counter `+0x94`, pet `+0x98`, jitter `+0x9c`/`+0xa0` (`screens/hatch-screen.md#2`).

## Capture format

`tools/ce_capture.lua` adds a `pres` object to every snapshot, and keeps capturing when there's no board (`"board": null`):

```json
"pres": {"music": {"track": 1, "order": 0,
                   "tracks": [{"id":1,"vol":1,"fade":0.002,"max":1,"stop":0,"h":…}, …]},
         "dialogs": [14],
         "screens": ["board"],
         "hatch": {"t": 37, "pet": 0, "jx": 2, "jy": 1}}
```
