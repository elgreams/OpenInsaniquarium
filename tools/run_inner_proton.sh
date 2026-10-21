#!/bin/bash
# RE tooling only (never used by the engine): run the original's inner game PE
# directly under the Steam Proton prefix so SexyApp command-line flags
# (-record, -play, -demofile, ...) reach it. The DRM launcher drops extra args.
#
# Prereq: python3 tools/extract_inner_pe.py "<install>/Insaniquarium.exe" re/bin/Insaniquarium_game.exe
# Usage:  tools/run_inner_proton.sh -record
set -euo pipefail
STEAM="${STEAM_ROOT:-$HOME/.steam/debian-installation}"
APPID=3320
REPO="$(cd "$(dirname "$0")/.." && pwd)"
export STEAM_COMPAT_DATA_PATH="$STEAM/steamapps/compatdata/$APPID"
export STEAM_COMPAT_CLIENT_INSTALL_PATH="$STEAM"
export WINEDEBUG=${WINEDEBUG:--all}
INNER="$STEAM_COMPAT_DATA_PATH/pfx/drive_c/re_inner/game.exe"
mkdir -p "$(dirname "$INNER")"
cp -f "$REPO/re/bin/Insaniquarium_game.exe" "$INNER"
cd "$STEAM/steamapps/common/Insaniquarium Deluxe"
exec "$STEAM/steamapps/common/${PROTON:-Proton Hotfix}/proton" run \
  'C:\re_inner\game.exe' '-changedir=S:\steamapps\common\Insaniquarium Deluxe' "$@"
