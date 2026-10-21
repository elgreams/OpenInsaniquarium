# Reverse-engineering spec index

Every behavior spec lives under `docs/re/<area>/`. See CLAUDE.md → "Clean-room workflow" for conventions.

Status: `todo` · `draft` (static analysis only) · `confirmed` (verified dynamically) · `traced` (covered by a differential test)

| Area | Spec | Status | Notes |
|---|---|---|---|
| framework | `framework/timing.md` | todo | Tick rate, updates per frame (M0) |
| framework | `framework/rng.md` | todo | Algorithm, seeding, streams (M0) |
| framework | `framework/demo-recording.md` | todo | Is `-record`/`-demofile` functional? (M0) |
| sim | `sim/board.md` | todo | Object lists, update order (M0) |
| assets | `assets/overview.md` | draft | Survey: resources.xml, ImageFont, fishsongs, music, saves. Split into per-format specs as each is implemented |
