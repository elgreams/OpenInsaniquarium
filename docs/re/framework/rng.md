# Random number generation

Addresses are inner-PE VAs.

Status: the algorithm and the app stream are `traced` (every replay compares the app `mti` each tick; any wrong draw diverges immediately). The seeding chain at startup is `static`: the harness always starts from captured RNG state.

## Algorithm: PopCap `MTRand` (MT19937, 31-bit output)

| Function | Address | Behavior |
|---|---|---|
| `MTRand::SRand(seed)` | `0x0040ae50` | Standard MT19937 init: `mt[0]=seed` (`0` → `4357`), `mt[i] = 1812433253 * (mt[i-1] ^ (mt[i-1] >> 30)) + i`, `mti = 624` |
| `MTRand::NextNoAssert()` | `0x0040aeb0` | Standard MT19937 regenerate (N=624, M=397, `mag01 = {0, 0x9908B0DF}` table at `0x005e80ac`) and temper (`>>11`, `<<7 & 0x9D2C5680`, `<<15 & 0xEFC60000`, `>>18`) |

Differences from reference MT19937:
- **Output is masked to 31 bits** (`& 0x7FFFFFFF`), so all results are non-negative `int`s.
- Temper masks appear in the code as `(y & 0xFF3A58AD) << 7` and `(y & 0xFFFFDF8C) << 15`. These are algebraically identical to the reference `(y << 7) & 0x9D2C5680` and `(y << 15) & 0xEFC60000` (the compiler pre-shifted the masks). No functional difference.
- State layout: `uint32 mt[624]; int mti;` (`mti` at `+0x9C0`), 2500 bytes (`0x9c4`, matching the `operator new(0x9c4)` in WinFishApp's ctor).

Ranged and float helpers (`Next(range)`, `Next(float)`) are **inlined** at call sites as `Next() % range`. The modulo is signed-int `%` on a non-negative value, so it's a plain modulo. TODO: confirm no float variant (`NextFloat`) is used by gameplay.

**Implementation note:** this is reproducible from the public MT19937 reference plus the `& 0x7FFFFFFF` mask. We implement it ourselves in `sexy-core` with test vectors captured from the original.

`traced`.

## Instances

| Instance | Location | Seed | Used by |
|---|---|---|---|
| **Global** `gMTRand` | `0x005eb928` (static) | `GetTickCount()` at app init (`SexyAppBase::Init`, slot 50, `0x004888b5`); reseeded from the demo file header on demo playback (`0x0047a10a`) | Wrapper `Rand()` @ `0x0040f9e0` (31 call sites: aliens, missiles, misc FX, and seeding the app RNG) |
| **App** `WinFishApp::mRand` | `MTRand*` at `app+0x7b0` | `gMTRand.Next()` drawn in `WinFishApp` ctor (`0x0054da21`) | **335 of 359** direct `Next()` call sites: virtually all gameplay |
| **Store** `StoreScreen::mRand` | inline at `storescreen+0x1d0` | Deterministic: `profile_field(+0x748..0x74c, int64) + ctr(0x005e8f1c) + arg + game_state(+0x8b8 → +0x98)` (`0x0052fea0`) | Virtual Tank store stock generation, so the shop inventory is reproducible from save state |

The CRT `rand()` (MSVC LCG, `0x005647dc`) is **also** present, seeded with `srand(GetTickCount())` at init (`0x0048892b`). It has 7 callers, mostly framework/UI (dialog effects, the StoreScreen overlay, a vfunction84). TODO: confirm none of the 7 are gameplay-relevant; if any are, the sim needs the MSVC LCG (`seed = seed*214013 + 2531011; return (seed>>16) & 0x7FFF`) too.

App stream: `traced`. Global stream: `confirmed` (captured, but its `mti` isn't compared because the bubble systems aren't modelled). Store stream: `static`.

## Determinism consequences

- **Gameplay RNG is a single stream** (`app+0x7b0`), plus `gMTRand` for some alien/missile code. Our sim needs the same split, and must consume both streams in the original order. Call order within a tick is defined by `Board::Update` (see `sim/board.md`).
- Seeds are wall-clock (`GetTickCount`). For differential tests we must either:
  1. read both MT states from memory at the start of a capture (CE: 2500 bytes at `[app+0x7b0]` and at `0x005eb928`) and load them into our sim, or
  2. use demo playback, which reseeds `gMTRand` from the demo header (and the app RNG is derived from `gMTRand` only at app construction, so it would need option 1 anyway).

  **Option 1 is the plan:** snapshot full MT state at trace start.
- Our `MTRand` must support exact state import/export (`[u32; 624]` + `mti`).

`traced` for tank 1.

## Dynamic confirmation (CE, 2026-10-21)

- App MTRand at `[app+0x7b0]` (app = `[0x005eb680]`). Captured state at the title screen: `mti == 624`, and **all 624 words equal `SRand(mt[0])`** with `mt[0] = 0x7FEF1344`. So the app RNG is freshly seeded and **has not drawn once** before gameplay starts. The seed is < 2³¹, consistent with it being one 31-bit output of `gMTRand` (seeded from `GetTickCount`).
- It does not advance on the title screen (no change over several seconds).
- **Harness consequence:** capturing the app RNG at the start of a level reduces to capturing a single `u32` (`mt[0]`) as long as `mti == 624`. Otherwise capture all 2500 bytes.
- `gMTRand` (`0x005eb928`) had `mti == 6` at the title screen, so a few draws happen during startup.

`confirmed`.

## Open questions

- Dynamic check: with CE, read `mti` at `[app+0x7b0]+0x9c0` across a few ticks of an idle tank. How many draws per tick happen at rest?
- Is the app RNG ever reseeded (e.g. on new game)? Static search found only the ctor store to `+0x7b0` and no `SRand` call with `ecx=[+0x7b0]`, so probably not.
