# Releasing

Releases are built by `.github/workflows/release.yml` from a `v*` tag into a **draft** GitHub release. Nothing is public until the draft is published by hand.

## Checklist

1. **Version**: set `[workspace.package] version` in `Cargo.toml`; run `cargo build` so `Cargo.lock` follows.
2. **Changelog**: add a `## [x.y.z] - YYYY-MM-DD` section to `CHANGELOG.md`, written for players: what works, what is verified against the original, what's missing. Add its link at the bottom. The workflow uses this section as the release notes and fails if it's missing.
3. **Checks**, all green:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - the full suite with the game data and traces present: `cargo test --release --workspace -- --test-threads=1`
   - `python3 tools/spec_confidence.py --check`
4. **Local dry run**: `cargo build --release --bin openinsaniquarium`, package it like the workflow does, run `tools/check_release.sh` on the archive, then run the packaged binary from a clean directory with no `INSANIQUARIUM_DATA`: first-run setup → main menu → 1-1 → pause/resume → back to the menu → quit.
5. **CI dry run** (optional): run the Release workflow by hand (`gh workflow run release.yml --ref <branch>`). It builds both archives and runs the asset check, and uploads them as workflow artifacts, without making a release.
6. **Commit** the version bump and changelog; push `main`.
7. **Tag**: `git tag -a vX.Y.Z -m "OpenInsaniquarium vX.Y.Z"` then `git push origin vX.Y.Z`.
8. **Watch** the workflow (`gh run watch`). When it's done, download both archives from the draft (`gh release download vX.Y.Z -D /tmp/rel`), run `tools/check_release.sh` on them, and try the Linux one.
9. **Publish** the draft (`gh release edit vX.Y.Z --draft=false`) only after review.

If anything fails after the tag is pushed, don't delete or move the tag and don't force-push: fix forward with a new patch version.

## Rules

- Release archives contain our binary, the two licence files, `README.md`, `CHANGELOG.md` and, on Windows, the libopenmpt DLLs with their licences. Never anything from the original game (`tools/check_release.sh` enforces this).
- Release builds never enable the app's `dev` feature (Bevy `dynamic_linking`).
