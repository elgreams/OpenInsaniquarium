# Image loading, alpha masks & cel animation

Status: `draft` (static analysis of the inner PE; shipped data checked against it).
Addresses are inner-PE VAs.

## Loader entry points

| Function | Address | Role |
|---|---|---|
| `ResourceManager::ParseImageResource` | `0x004166c0` | Reads `<Image>` attributes into an `ImageRes` |
| `ResourceManager::DoLoadImage` | `0x00418d80` | Loads one image resource and applies masks/flags |
| `ImageLib::GetImage(path, lookForAlpha)` | `0x0049fed0` | Extension probe + implicit alpha-mask lookup |
| `ResourceManager::ApplyAlphaImage` | `0x00418bf0` | Explicit `alphaimage` |
| `ResourceManager::ApplyAlphaGrid` | `0x004189b0` | Explicit `alphagrid` |
| `ImageLib::GetGIFImage` | `0x0049c160` | GIF decoder (own LZW) |
| `AnimInfo::Compute` | `0x004575e0` | Builds animation tables after parsing |
| `AnimInfo::GetCel(time)` | `0x004577e0` | Time → cel index |

Pixels are 32-bit `0xAARRGGBB`. "Byte 0" below is the low byte (blue).

## Extension resolution (`GetImage`)

The path is split into directory, stem and extension at the last `.` that comes after the last `/` or `\`.
- **Path has an extension:** only that format's decoder is tried.
- **Path has no extension** (every manifest entry): formats are tried in this order, and the first that decodes wins:
  **`.tga` → `.jpg` → `.png` → `.gif` → `.j2k` → `.jp2`**

The shipped data only uses `.jpg` and `.gif`. Where both exist for one stem, `.jpg` wins. Filesystem lookups are case-insensitive (Windows).

## Implicit alpha mask (`GetImage` with `lookForAlpha = true`)

After trying to load the colour image (it may be missing), the loader looks for a mask:

1. `<dir>/_<stem>`: **prefix underscore**, using the same extension probe, without a recursive mask lookup
2. if not found: `<dir>/<stem>_`: **suffix underscore**

If a mask is found:
- **Colour image present:** copy the mask's byte 0 into the colour image's alpha byte, pixel for pixel. This happens **only if the dimensions match exactly**. On a mismatch the mask is silently dropped here (unlike the explicit path below).
- **Colour image missing:** the result is the mask itself, recoloured: each pixel becomes `(mask.byte0 << 24) | rgb`, where `rgb` is the resource's `alphacolor` (default `0xFFFFFF`, white). This is how the 14 mask-only images (`TITLEPAGEMASK`, `TANKMASK1–6`, `HATCHREFLECTION`, `PETBUTTONHOLE`, …) and the mask-only font sheet `_BlambotPro15` load.

If no mask is found, the colour image keeps its own alpha (opaque, except GIF transparency, below).

Masks in the install are pure grayscale, so taking byte 0 (blue) is the same as taking luminance.

## Explicit alpha attributes (`DoLoadImage`)

Applied after `GetImage`, so they override any implicit mask:
- `alphaimage="name"`: load `name` from the current `SetDefaults` path. Its byte 0 → the colour image's alpha. A size mismatch is an **error** ("AlphaImage size mismatch…") and fails the resource. One mask can be shared by many images (e.g. `tankbackdrops_`).
- `alphagrid="name"`: the mask is **one cel**. It must be exactly `(width / cols) × (height / rows)` and is stamped into every cel of the grid. A mismatch is an error.
- `alphacolor="RRGGBB"` (hex, via `sscanf("%x")`): RGB used when the colour image is missing. Not used by the shipped manifest.

## GIF specifics (`GetGIFImage`)

- Palette entries become opaque (`0xFF000000 | rgb`).
- If a Graphic Control Extension sets the transparency flag, **the transparent index decodes to `0x00000000`**: fully transparent black. That also applies inside masks, where a transparent mask pixel gives alpha 0.
- Only the first image frame is decoded. Interlaced images are supported. NETSCAPE loop and comment extensions are skipped.

## Other `<Image>` attributes

| Attribute | Effect | Relevance to us |
|---|---|---|
| `nopal` | Don't palettize | Render hint; ignore |
| `a4r4g4b4`, `a8r8g8b8` | D3D texture format hints | Ignore |
| `ddsurface` | Keep as a DirectDraw surface (fast opaque blit) | Ignore (6 large backgrounds) |
| `nobits`, `nobits2d`, `nobits3d` | Discard the CPU copy after upload | Ignore |
| `minsubdivide` | D3D texture tiling hint | Ignore |
| `noalpha` | Treat as opaque | Honour if it ever appears |
| `variant` | Locale/variant suffix | Not used |
| `rows`, `cols` | Cel grid; default 1 | **Required** |
| `anim`, `framedelay`, `begindelay`, `enddelay`, `perframedelay`, `framemap` | Animation | **Required** (below) |

## Cel animation (`AnimInfo`)

| Field | Meaning |
|---|---|
| `type` | `none`=0, `once`=1, `pingpong`=2, `loop`=3 (case-insensitive; anything else is an error) |
| `frameDelay` | from `framedelay`, clamped to ≥ 1 |
| `numCels` | **`max(rows, cols)`**, not `rows*cols`. Strips are 1-D; 2-D sheets like `SYLV` 10×2 are animated only along one axis, and the other axis is chosen by game code (direction/state) |
| `perFrameDelay[]` | from `perframedelay` |
| `frameMap[]` | from `framemap` |

Build steps (`Compute`):
1. `pingpong` with `numCels > 1`: `frameMap = [0, 1, …, n-1, n-2, …, 1]`. An explicit `framemap` takes precedence (TODO confirm the precedence order).
2. If `frameMap` is non-empty: `numCels = len(frameMap)`.
3. `begindelay > 0` → `perFrameDelay[0] = begindelay`. `enddelay > 0` → `perFrameDelay[numCels-1] = enddelay`. The vector grows as needed.
4. If `perFrameDelay` is empty: `totalTime = numCels * frameDelay`. Otherwise it is resized to `numCels`, entries `< 1` become `frameDelay`, and `totalTime = sum`.

`GetCel(t)` (`t` is supplied by the caller; see below):
- `once` and `t >= totalTime`: return the last frame (`frameMap.last()` if mapped, else `numCels-1`).
- No per-frame delays: `i = ((t % totalTime) / frameDelay) % numCels`.
- With per-frame delays: walk `perFrameDelay`, subtracting each delay from `t % totalTime`… (`0x00457790`). It returns the first index where the running value goes negative. **Note:** the walk subtracts from `t` itself, not `t % totalTime`. TODO verify whether the modulo is applied first.
- If `frameMap` is non-empty: return `frameMap[i]`.

`-1` in a frame map (`MERYLBLINK`: `0,1,2,1,0,-1` with `anim=once`) is returned as cel `-1`. Callers treat a negative cel as "draw nothing". That's how the blink overlay disappears once the one-shot ends (guess; confirm at the call site).

**Time units:** `GetCel` is unitless. Game objects pass their own update counters, so `framedelay` counts **28 ms logic ticks** (see `framework/timing.md`). TODO: confirm at a call site (e.g. `Sylvester` draw).

## Open questions

- ~~Mirrored / scaled sprites~~: resolved in `sprite-variants.md`. Mirroring happens at draw time, and the scaled sprites are shipped `scl_*` files.
- Implicit-mask lookup for the **font** glyph sheet (`ImageFont` loads via `GetImage(…, true)`?). The mask-only `_BlambotPro15` implies yes.
- Precedence when both `framemap` and `pingpong` are present.
