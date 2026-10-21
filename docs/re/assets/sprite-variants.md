# Sprite variants: mirroring, scaled pet images, load-time derived images

Status: `draft`. Static analysis of the inner PE only; nothing here has been checked dynamically yet. Addresses are inner-PE VAs. Function names are the ones now applied in the Ghidra project.

## Summary

- **No image is mirrored at load time.** Mirroring is a **draw-time flag**. Draw calls go through `Graphics::DrawImageMirror_SrcRect`, which copies the chosen source rectangle (one cel) to the screen with each row read right-to-left. The same image object is used for both facings. Cel order and cel positions in the sheet never change.
- **No image is scaled at load time by a fixed factor.** The `SCL_*` images are **separate files shipped pre-scaled** (`images/scl_*.gif` plus `_scl_*.gif` masks). They load through the normal manifest path (`assets/images.md`).
- The `resources.xml` comments "need to be mirrored (and scaled)" are **authoring notes**. They have no effect on the engine. The engine can mirror any image, and several images outside those sections are drawn mirrored (e.g. `SMALLTURN`, `TANKLIGHTING`, `WAVESIDE`).
- Game code builds **four kinds of derived image** once all resources have loaded. These are the only post-load image transforms (listed below). The loader must reproduce them.

## Image id table

| Function | Address | Role |
|---|---|---|
| `WinFish::ExtractLoadingThreadResources` | `0x005073e0` | Resolves every `IMAGE_*` string to its `Image*` and stores it in a global |
| `WinFish::GetImageById` | `0x005016a0` | `id → *table[id]`. The table at `0x005e0a10` holds pointers to the `IMAGE_*` globals (0-based id = manifest order) |
| `WinFish::GetStringIdById` | `0x005016c0` | `id → "IMAGE_…"` string (jump table) |
| `WinFish::ReplaceImageById` | `0x0050e770` | Replaces a resource's image in the ResourceManager **and** rewrites the global |
| `WinFish::LoadImageById` | `0x0050e6b0` | Loads one image by id early, before the loading thread (used for `STINKY`, id `0xca`, in `WinFishApp::Init`) |

The ids used below are `5` TITLEPAGE, `6` TITLEPAGEMASK, `0x50` HORZCREASE, `0x51` VERTCREASE, `0x9f..0xa4` AQUARIUM1–6, `0xa5..0xaa` TANKMASK1–6, `0xca..0xe1` STINKY…WALTER (the 24 full-size pets), and `0xe8..0xff` SCL_STINKY…SCL_WALTER (the 24 scaled pets, in the same order as the full-size ones).

## Mirrored drawing (draw time)

### API

| Function | Address | Behavior |
|---|---|---|
| `Graphics::DrawImageMirror_SrcRect(img, x, y, srcRect, mirror)` | `0x004560a0` | Main entry point (≈70 call sites) |
| `Graphics::DrawImageMirror_XY(img, x, y, mirror)` | `0x00456060` | Whole image; `srcRect = (0,0,w,h)` |
| `Graphics::DrawImageMirror_Stretch(img, destRect, srcRect, mirror)` | `0x004561c0` | Mirrored stretch (hit-shrink and pulse effects); not used by the loader |
| `WinFish::DrawImageMirrorMaybeStretch` | `0x005008f0` | Calls the plain mirror draw when dest size == src size, otherwise the mirrored stretch |
| `DDImage::BltMirror` (vtable slot 15, `+0x3c`) | `0x004490f0` | Dispatches by draw mode to `NormalBltMirror` `0x00444550` or `AdditiveBltMirror` `0x00447b70` |

### Semantics of `DrawImageMirror_SrcRect` (static)

- `mirror == false`: identical to `Graphics::DrawImage_SrcRect` (`0x00455e40`).
- `mirror == true`:
  1. `dx = trunc(x + transX)`, `dy = trunc(y + transY)` (float add, then ftol).
  2. If `srcRect` does not fit inside the image (`sx+sw > img.w` or `sy+sh > img.h`), **nothing is drawn**.
  3. Clip the destination rectangle `(dx, dy, sw, sh)` against the clip rect, giving `(cx, cy, cw, ch)`. Stop if it is empty.
  4. Adjust the source rectangle for a horizontally flipped clip:
     - `sx' = sx + sw − (cx − dx) − cw`
     - `sy' = sy + (cy − dy)`
     - size `cw × ch`
  5. Blit with each source row read from `sx'+cw−1` down to `sx'`, written left to right. The colorize colour and draw mode (normal or additive) apply as usual.
- The Graphics **scale is ignored** by the mirror path. The non-mirror path honours it. Mirroring only ever flips around the source rectangle's own vertical centre line.
- **Per-cel, never whole-sheet.** Callers pick the cel with `srcRect = (col*cw, row*ch, cw, ch)` and pass the flag. Flipping the whole sheet would reverse cel order; that never happens.
- Only `DDImage` (screen and surface) implements the mirror blit. On `Sexy::Image` and `MemoryImage` the slot `+0x3c` is a no-op (`0x00457b90`, `ret 0x18`). **A mirrored draw into an off-screen MemoryImage draws nothing.** That is another reason no mirrored copies can have been built at load time.

### Native facing

The source art faces **left**: guppy, pets, aliens. The flag is `true` when the object should face right.

### Who passes what (static; the offsets are raw `this` offsets taken from disassembly)

- **`Fish::Draw`** `0x004e4a90` (base fish draw, inherited by FishTypePet, Gekko, Oscar, SylvesterFish, Ultra, BallFish, BiFish):
  - `turn = +0x1c4` (int, non-zero while turning). `inv = +0xf5` (byte, a per-object "invert facing" flag that `GameObject::GameObject` clears).
  - `turn > 0` → `mirror = !inv`. `turn < 0` → `mirror = inv`.
  - `turn == 0`: look at `vx = +0x168` (double).
    - `vx < 0` → `mirror = inv`.
    - `vx > 0` → `mirror = !inv`.
    - `vx == 0` → use the facing scale `+0x188`: `< 0` → `inv`, `> 0` → `!inv`, `== 0` → **the body is not drawn this frame**.
  - The result goes to the virtual body draw (vtable `+0x14c`, e.g. `Fish::DrawBody` `0x004dea00`).
- **`Fish::GetSwimImage(hungry)`** `0x004d5aa0` chooses the guppy sheet. It is drawn with the same flag:
  - `+0x1c4 != 0` → `SMALLTURN` / `HUNGRYTURN`.
  - `+0x1c8 < 1` and `+0x108 < 101` → `SMALLSWIM` / `HUNGRYSWIM`.
  - otherwise → `SMALLEAT` / `HUNGRYEAT`.

  So the turn sheets are mirrored too, even though `resources.xml` lists them in the "not mirrored" section.
- **`Alien::DrawSprite`** `0x004dae20`: the flag comes from the caller. If `+0x1b4 != 0` it is overridden to `(+0x1b4 > 0)`, and the sheet row becomes 1 (the turn row).
- **`Stinky::DrawSprite`** `0x004e2cc0`: the flag comes from the caller. If `+0x1b4 != 0`, row 2 is used. Otherwise, if `+0x19c != 0`, row 1 is used with `mirror = (+0x19c > 0)`. The cel is `+0x1a4`, and cels are 80×80.
- Generic pet draw `0x004dfbb0` (Vert, Seymour, Blip, Nimbus, Presto, Brinkley, Nostradamus via `FishTypePet` `0x004f8610`): `mirror = (+0x1c4 > 0)` when `+0x1c4 != 0`.
- Literal flags:
  - `Board::DrawBackdrop` `0x00538e50` draws `TANKLIGHTING` twice normally and twice with `mirror = 1`.
  - `0x00500c90` draws `WAVESIDE` at x=0 normally and at x=480 mirrored.
  - `TitleScreen` draws `STINKY` mirrored.
  - `StoryScreen` draws `BILATERUS` mirrored.
  - `BoxingGlove` (Walter) draws one mirrored and one normal copy.
  - Grubber `0x004e19f0` and Starcatcher `0x004e2e30` always pass `0`.

Mirror-capable call sites seen (images): SMALLSWIM / HUNGRYSWIM / SMALLEAT / HUNGRYEAT / SMALLTURN / HUNGRYTURN, SMALLDIE, HUNGRYBREEDER, BREEDER, SIMSWIM1–3 / SIMEAT1–3 / SIMTURN1–3, SIMCARNIVORE, ULTRA, GEKKO, GRUBBER and STARCATCHER (flag always 0), SYLV, BALROG, GUS, DESTRUCTOR, ULYSSES, PSYCHOSQUID, MINISYLV, BOSS, BILATERUS, BALLS, and all 24 pets plus their overlays (HALO, GUMBOLIGHT, AMPCHARGE). Overlays use the same flag as their pet. Also TANKLIGHTING and WAVESIDE.

**Implementer note:** the app layer needs only one capability: "draw cel with optional horizontal flip". In Bevy that is `Sprite { flip_x, rect }`, with no extra textures. The sim must expose the inputs to the flag (turn timer, vx, facing scale, invert flag) or the flag itself.

## Scaled pet images (`SCL_*`): shipped files, no runtime scaling

Shipped dimensions, read from the GIFs:

| Group | Full-size cel | `scl_*` sheet | `scl_*` cel | Factor |
|---|---|---|---|---|
| 23 pets (STINKY … WALTER except AMP) | 80×80 (sheets 800×H) | 600×60, `cols=10`, 1 row | 60×60 | 0.75 |
| AMP | 160×60 (1600×120) | 640×24 | 64×24 | 0.4 |
| OSCAR, GRUBBER, STARCATCHER, GEKKO, BREEDER, ULTRA | 80×80 (ULTRA 160×160) | 400×40 | 40×40 | 0.5 (ULTRA 0.25) |

Only the first animation row was pre-rendered. No code resamples a full-size sheet into an `SCL_*` image. Those manifest ids load like any other image (an implicit `_scl_*` mask is found by the prefix rule).

Readers (static): the tutorial / egg-piece text (`0x005220a0`: SCL_STINKY, SCL_ITCHY), `PetButtonWidget` (`0x00531300`: SCL_AMP), the FoodDialog / store (`0x00534030`: SCL_OSCAR, SCL_ULTRA), and the store tutorial (`0x005406a0`: SCL_OSCAR … SCL_ULTRA). The PetDialog does **not** draw the SCL sheets directly; it uses the thumbnails below.

## Load-time derived images (must be reproduced by our loader)

All four are built **after** the resource groups have loaded. Each new image is a fresh 32-bit MemoryImage whose pixels start at `0x00000000`. Drawing into it uses the normal (non-mirror) blit.

### 1. Crease strips: `WinFishApp::LoadingThreadProc` `0x005510b0` (static)

- `VERTCREASE` (8×3) is tiled vertically: copies at y = 0, 3, 6, … while y < 60. The result is **8×60** and replaces resource id `0x51`.
- `HORZCREASE` (3×8) is tiled horizontally the same way. The result is **60×8** and replaces id `0x50`.
- Both sources have implicit `…_` masks, so the result keeps per-pixel alpha.

### 2. Tank foreground cut-outs: same function, loop `i = 0..5` (static)

`TANKMASK(i+1) := CreateMaskedCutout(src = AQUARIUM(i+1), mask = TANKMASK(i+1), x = 0, y = 365)`. It replaces ids `0xa5+i`.

`WinFish::CreateMaskedCutout(src, mask, x, y)` `0x00503120`:
1. `r = (x, y, mask.w, mask.h) ∩ (0, 0, src.w, src.h)`. If empty, return null.
2. Create a new `r.w × r.h` image and draw `src` at `(−r.x, −r.y)` into it, i.e. copy that region of `src`.
3. For each pixel in linear order `k = 0 .. r.w*r.h−1`: `out.alpha = mask.alpha[k]`; RGB is kept. The mask is indexed linearly with no stride fix-up, which only matters if the rect was clipped (never for shipped data).

With shipped data the results are 640×80. RGB comes from rows 365–444 of `aquariumN.jpg`; alpha comes from `_maskN.jpg` (`TANKMASKn` is mask-only, so its alpha is the mask's byte 0; see `images.md`). `Board::DrawBackdrop` draws the cut-out at (0, 365) over the backdrop.

### 3. Title-page cut-out: `WinFishApp::Init` `0x005503c0` (static)

`TITLEPAGEMASK := CreateMaskedCutout(TITLEPAGE, TITLEPAGEMASK, 384, 388)`. It replaces id `6`. The mask `titlescreenmask_.gif` is 202×67, so the result is the 202×67 region of `titlescreen.jpg` at (384, 388) with the mask's alpha.

### 4. PetDialog thumbnails: `LoadingThreadProc`, loop `i = 0..23` (static)

For each `i`, take `src = image id 0xe8+i` (SCL_STINKY … SCL_WALTER) and create a **40×40** image with *fast-stretch off*:

- `i == 16` (SCL_AMP): stretch source rect `(0, 0, min(w,64), min(h,24))` (cel 0, 64×24) to dest rect `(0, 10, 40, 20)`. The scale is non-uniform: 0.625 × 0.833.
- otherwise: stretch `(0, 0, min(w,60), min(h,60))` (cel 0, 60×60) to `(0, 0, 40, 40)`. Scale 2/3.

The result is stored in an app-owned array, `app+0x73c` (0x518 bytes = 326 slots, indexed by **full-size pet resource id**), at slot `0xca+i`. That slot is the id of the matching full-size pet (STINKY = 0xca). `PetDialog::Draw` `0x00535800` draws slots `0xca..0xe1` at (button.x+5, button.y+5).

**Downscale filter** (`MemoryImage::SlowStretchBlt` `0x004657a0`, reached via `MemoryImage::StretchBlt` `0x00466e30` and `ClipStretchRects` `0x00462a50`; static):

- It is a separable area-average (box) filter with fractional edge coverage, in fixed point.
  - **Horizontal pass:** each source column covers the destination span `[k·s + o, (k+1)·s + o)`, where `s = dstW/srcW` and `o = 1 + (srcX₀ − srcRect.x)·s` (a 1-pixel border in the accumulator). If both ends truncate to the same dest column, it adds `pixel·trunc(s·257)` there. Otherwise it splits between the two columns with weights `trunc(coverage·257)`.
  - **Vertical pass:** the same, with factor **256**.
  - Channels (B, G, R, A) are accumulated independently and **not premultiplied**. Transparent GIF pixels are `0x00000000`, so they pull edge colours toward black.
- Final channel value = `acc >> 16`. Full coverage of 255 gives 255.
- **Compositing onto the destination:**
  - For a source alpha `a ≠ 0`: `c_out = (c·a >> 8) + (c_dst·(256−a) >> 8)` per colour channel, and `a_out = ((255−a_dst)·a)/255 + a_dst`.
  - Onto the empty thumbnail canvas this reduces to `c_out = c·a >> 8`, `a_out = a`. Thumbnails therefore come out slightly darker than the source (255 becomes 254 even at full alpha).
- The upscale branch (dst > src) is a different, bilinear-like path (weights `0x101 − f`). No load-time variant uses it.

Fidelity here is visual only; nothing in the sim depends on it. A Rust implementation can start with "box-filter downscale of cel 0 into 40×40 (AMP: into the 40×20 band at y=10)" and refine to the exact fixed-point weights later if screenshot diffs show a difference.

## Open questions

- Confirm dynamically that the four derived-image steps are the complete set, e.g. by watching `ReplaceImageById` and the `app+0x73c` writes. Two other MemoryImage creators exist:
  - `0x005172e0`, the egg-hatch banner: built when the event happens, not at load.
  - `0x005006a0` (`GameObject::DrawBackdropCutout`): a per-frame temporary that copies backdrop pixels under an object's alpha, with its own manual reversed-row read when mirrored. This is a draw effect, not a cached variant.
- The exact blend formula of the **non-stretch** normal blit into a MemoryImage, used by the crease tiling and the cut-outs. For opaque sources (JPG backdrops) the copy should be exact. The crease GIFs have partial alpha, so confirm whether colours are altered the way they are in the stretch path.
- The exact semantics of `+0x1c4` (turn timer and its sign convention), `+0x1c8`, `+0x108` and `+0xf5` belong in the fish spec. `+0xf5` is copied in `0x004f21e0` (clone); who sets it is unknown.
- Whether the "body not drawn when vx == 0 and facing scale == 0" case can actually happen. The guppy ctor always sets the scale to ±1.0.
- The pet / alien callers' own flag sources (the alien `param_3`, the Stinky `param_3`) need tracing in their spec docs.
