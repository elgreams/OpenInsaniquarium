#!/bin/bash
# RE tooling only: record the original playing a level (game's own -record
# demo + a CE capture), scripted so nothing is improvised.
# docs/re/dynamic-analysis.md "Recording with tools/record.sh" has the routine.
#
#   tools/record.sh launch T-L            backup saves, set level, launch -record; prints PID
#   (CE MCP: open_process PID; then evaluate_lua with the capture, see docs)
#   tools/record.sh go PID "ID ID.." [autoplay args...]
#                                         in the background: Adventure, pick pets on the
#                                         pet screen, Continue, autoplay; returns at once
#   tools/record.sh quit                  Resume/Menu/Back/Leave, then Quit the game
#   tools/record.sh save NAME             demo + capture into traces/, restore saves
#
# Env: MONEY=N and CD=N pushed 6 s after the level starts (orig_mem.py).
set -euo pipefail
REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"
D="$HOME/.steam/debian-installation/steamapps/compatdata/3320/pfx/drive_c/ProgramData/Steam/Insaniquarium"
BK="$REPO/re/backup/userdata-record"
CAP=/tmp/capture.jsonl
click() { tools/orig_click.sh "$1" "$2" 150; }

# Pet button centre on the pet screen (screens/pet-screen.md#layout).
pet_xy() {
  local id=$1 x row
  if   ((id <= 4));  then x=25;  row=$id
  elif ((id <= 9));  then x=119; row=$((id - 5))
  elif ((id <= 14)); then x=425; row=$((id - 10))
  elif ((id <= 19)); then x=519; row=$((id - 15))
  elif ((id <= 21)); then x=226; row=$((id - 17))
  else                    x=323; row=$((id - 19)); fi
  echo $((x + 45)) $((41 + 83 * row + 41))
}

case "${1:-}" in
launch)
  [ -d "$BK" ] || { mkdir -p "$BK"; cp "$D"/userdata/*.dat "$BK"/; }
  python3 tools/set_profile.py "$D/userdata/user1.dat" --level "$2" >/dev/null
  rm -f "$D/userdata/adv1.dat"
  (setsid tools/run_inner_proton.sh -record >/tmp/orig.log 2>&1 &)
  sleep 20
  PID=$(pgrep -f 'game.exe -changedir' | while read p; do
    tr '\0' ' ' </proc/$p/cmdline | grep -q '^C:' && echo $p; done | head -1)
  click 318 431   # title "Click here to play" -> main menu
  sleep 4
  sudo python3 tools/orig_mem.py "$PID" owned=all quiet >/dev/null
  echo "PID $PID (main menu, all 24 pets owned)"
  ;;
go)
  PID=$2; PETS=$3; shift 3
  (
    sleep 4; sudo rm -f /tmp/gpos
    click 465 90; sleep 3                     # Adventure -> pet screen
    n=0
    for id in $PETS; do read -r x y < <(pet_xy "$id"); click "$x" "$y"; sleep 0.7; n=$((n+1)); done
    click 318 264; sleep 1.5                  # Continue
    ((n < 3)) && click 243 271                # "Select more pets?" -> Yes
    python3 tools/autoplay_tank11.py /tmp/gpos "$@" >/tmp/auto.log 2>&1 &
    sleep 6
    args=()
    [ -n "${MONEY:-}" ] && args+=("money=$MONEY")
    [ -n "${CD:-}" ] && args+=("cd=$CD")
    ((${#args[@]})) && sudo python3 tools/orig_mem.py "$PID" "${args[@]}" quiet
  ) >/tmp/go.log 2>&1 &
  disown
  echo "started; now start the CE capture (it must begin within 4 s)"
  ;;
quit)
  pkill -f '[a]utoplay_tank11' || true
  click 312 258; sleep 1.5                    # Resume / Game over Continue (harmless otherwise)
  click 573 17; sleep 1.5; click 314 300; sleep 1.5; click 244 285; sleep 3
  click 556 426; sleep 2; click 244 258; sleep 8
  pgrep -f 'game.exe -changedir' >/dev/null && echo "game still running: screenshot it" || echo "game quit"
  ;;
save)
  N=$2
  cp "$D/popcap.dmo" "traces/demos/$N.dmo"
  sudo cat "$CAP" | python3 -c "
import sys, json
out = []
for l in sys.stdin:
    try: json.loads(l); out.append(l)
    except ValueError: pass
sys.stdout.write(''.join(out))" | zstd -q -f -o "traces/original/$N.trace.jsonl.zst"
  cp "$BK"/*.dat "$D/userdata/"; rm -f "$D/userdata/adv1.dat"
  ls -la "traces/demos/$N.dmo" "traces/original/$N.trace.jsonl.zst"
  ;;
*) sed -n 2,14p "$0"; exit 1 ;;
esac
