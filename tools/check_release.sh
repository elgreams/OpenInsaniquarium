#!/bin/bash
# Fail if a release archive contains anything from the original game
# (bring your own assets: CLAUDE.md). Usage: tools/check_release.sh ARCHIVE...
# Allowed: our binary, licences, README/CHANGELOG, data-path.txt, the
# gameData placeholder note, and the bundled
# libopenmpt DLLs (Windows) with their licences.
set -euo pipefail
status=0
for a in "$@"; do
  case "$a" in
    *.zip) list=$(unzip -Z1 "$a") ;;
    *.tar.gz|*.tgz) list=$(tar -tzf "$a") ;;
    *) echo "unknown archive type: $a"; exit 2 ;;
  esac
  bad=$(printf '%s\n' "$list" | grep -v '/$' | grep -iE \
    '\.(gif|jpe?g|png|bmp|ogg|au|wav|mp3|mo3|it|xm|mod|s3m|txt\.font|ttf|fon|dat|pak|exe|dll|scr|xml)$|resources\.xml|/(images|sounds|music|properties|fonts|data|userdata)/|fishsong' \
    | grep -viE '(^|/)openinsaniquarium(\.exe)?$|(^|/)gameData/PUT-GAME-FILES-HERE\.txt$|(^|/)(libopenmpt|openmpt-(mpg123|ogg|vorbis|zlib))\.dll$' || true)
  if [ -n "$bad" ]; then
    echo "FAIL $a: game-like files found:"; printf '  %s\n' $bad; status=1
  else
    echo "ok   $a ($(printf '%s\n' "$list" | grep -vc '/$') files)"
  fi
done
exit $status
