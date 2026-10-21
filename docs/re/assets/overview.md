# Asset formats — survey

Status: `draft` (from inspecting the shipped data files; no binary analysis yet).
Target: Insaniquarium Deluxe 1.1 (see CLAUDE.md for hashes).

All assets are loose files under the install root. There is no archive in this build (SexyApp's `PakInterface` is linked in but no `.pak` ships).

## Install layout

| Path | Contents |
|---|---|
| `properties/resources.xml` | Resource manifest (images, fonts, sounds) |
| `images/` | 425 image stems: 390 `.gif`, 35 `.jpg` |
| `data/` | ImageFont descriptors (`*.txt`) + their glyph sheets (`*.gif`, `_*.gif`) |
| `sounds/` | 65 sounds, `.ogg` and Sun `.au` |
| `music/` | `Insaniq2.mo3`, `Alien.mo3`, `Lullaby.mo3` (BASS MO3 tracker modules) |
| `fishsongs/` | 72 text-format songs for the Virtual Tank singing fish |
| `userdata/` | Created at runtime — saves (see below) |
| `drm/`, `properties/partner.xml(.sig)` | PopCap DRM wrapper — out of scope |

## `resources.xml`

SexyApp ResourceManager manifest. **Not strict XML:** some attributes are unquoted (`cols=3`, lines 64–67). The parser must be lenient, so a stock XML crate won't do without preprocessing.

Structure:
- `<ResourceManifest>` → 3 `<Resources id="...">` groups: `Init`, `Register`, `LoadingThread`. Groups load at different phases (Init before the title screen; LoadingThread while the loader bar runs).
- `<SetDefaults path="dir" idprefix="PREFIX_" />` is stateful. It applies to every following entry until the next SetDefaults. The full id is `idprefix + id` (e.g. `IMAGE_SYLV`); the file path is `path/` + entry path, **without extension**.
- Entries: `<Image>` (246), `<Sound>` (65), `<Font>` (15).

Extension resolution: entry paths have no extension. The loader must probe for the actual file (`.gif`, `.jpg`, `.png`, `.ogg`, `.au`, …). The exact probe order is TODO (binary).

### `<Image>` attributes

| Attr | Count | Meaning |
|---|---|---|
| `path` | all | Color image stem |
| `alphaimage` | 13 | Explicit alpha mask stem. Can be **shared** by several images (e.g. all `tankbackdropN` use `tankbackdrops_`) |
| `alphagrid` | 4 | Alpha mask is a single cel tiled across a `cols`×`rows` grid |
| `cols`, `rows` | 106 / 52 | Sprite sheet cel layout, default 1 |
| `anim` | 66 | `loop` (56), `pingpong` (8), `once` (2) |
| `framedelay` | 65 | Ticks per frame (unit: update ticks, TODO confirm) |
| `begindelay` | 3 | Delay before the anim starts |
| `perframedelay` | 1 | Comma list, one delay per frame (`MERYLBLINK`) |
| `framemap` | 2 | Explicit frame order; `-1` appears as a terminator/blank (`0,1,2,1,0,-1`) |
| `alias` | 2 | Alternate id |

Implicit alpha convention: if no `alphaimage` is given, `foo` uses `_foo` (leading underscore) when it exists. 59 stems are not referenced by the manifest. Most are trailing-underscore masks (`money_`, `shells_`, `bag_`, …), which suggests a second convention: `foo` → `foo_`. **TODO:** confirm the lookup order for `_foo` vs. `foo_` in the binary. Unreferenced non-mask images `nebula1` and `talkshow` are loaded by hardcoded path from code (`images/nebula1`, `images/talkshow` strings in the exe).

Alpha masks are grayscale: the mask's luminance (probably the red channel; TODO) becomes the color image's alpha. JPGs never carry alpha themselves.

The XML comment "Characters that need to be mirrored (and scaled)" means some sprites get **mirrored/scaled copies generated at load time** rather than shipped. We need to find which ones and how (binary TODO).

### `<Sound>` attributes

`path`, `volume` (7 entries; a 0–1 float base volume). Formats: Ogg Vorbis, and Sun/NeXT `.au` (μ-law or PCM; TODO check encodings).

## ImageFont descriptors (`data/*.txt`)

SexyApp ImageFont script, ISO-8859-1 encoded (the CharList contains Latin-1 bytes; read as bytes, not UTF-8).

Grammar (informal):
```
Define <Name> <Value>;
<Command> <args...>;
Value   := '(' item (',' item)* ')' | scalar
item    := 'c' (char literal) | "ab" (string) | int | '(' int,int,int,int ')' (rect)
```
`Define` binds a named list. Commands reference names or inline lists. Commands seen:

| Command | Args |
|---|---|
| `CreateLayer` | layer name |
| `LayerSetImage` | layer, `'imagestem'`: glyph sheet in `data/`, with alpha via `_stem` |
| `LayerSetAscent` | layer, int |
| `LayerSetAscentPadding` | layer, int |
| `LayerSetLineSpacingOffset` | layer, int (can be negative) |
| `LayerSetPointSize` | layer, int |
| `LayerSetCharWidths` | layer, charlist, widthlist (repeatable; later calls override, e.g. `(' ') (8)`) |
| `LayerSetImageMap` | layer, charlist, rectlist (x, y, w, h on the sheet) |
| `LayerSetCharOffsets` | layer, charlist, offsetlist (x, y draw offsets) |
| `LayerSetKerningPairs` | layer, pairlist (`"AV"` strings), valuelist (int deltas) |
| `SetDefaultPointSize` | int |

All 15 shipped fonts use one layer (`Main`). The "outline"/"outback" variants are separate fonts, not extra layers. The renderer must still support multiple layers, since PvZ uses them.

The SexyApp framework source PopCap published publicly documents this format. Use it as a behavioral reference for edge cases (layer draw order, color modulation), and confirm the details against this binary.

## Fish songs (`fishsongs/*.txt`)

Plain-text note format for the Virtual Tank singing fish. 72 files, many in `_long`/`_short` pairs.

- Directives (`*` prefix): `*attrib title = <name>`, `*attrib <flag>` (`beethoven`, `beethovenrare`, `santarare`, `kilgore`: these appear to gate which pet/fish sings it), `*speed N`, `*line N` (voice/channel select), `*volume F`, `*on`, `*rest`, `*shift N`, `*localshift N`.
- Notes: `<pitch><octave> <duration>`, comma-separated. Pitch `a`–`g` with optional `b`/`#`; `r` = rest; the octave can be omitted (`f h`).
- Durations: `w` `h` `q` `e` `s` `t` `z` (whole → 1/64-ish); suffix `d` = dotted (`dd` double), `t` = triplet; `0` = zero length, i.e. a chord with the next note.
- Line breaks and blank lines appear to be cosmetic. TODO confirm.

Synthesis method (which instrument sample, envelope) is unknown. Binary TODO, M5.

## Music (`music/*.mo3`)

Un4seen MO3 (compressed tracker module). libopenmpt decodes MO3, including Vorbis/MP3-compressed samples. The original plays via BASS and switches song **positions/orders within one module** for different game states (TODO: map which orders). Matching playback speed and fades is a framework concern.

## Save data (`userdata/`)

File names from the exe strings: `users.dat` (profile list), `user%d.dat` (per-profile progress), `highscores.dat`, `scr%d.dat` (likely screensaver/virtual-tank state), `screensaver.dat`. Binary format TODO. Saving (`DataWriter`/`DataReader` classes exist) is M4.
