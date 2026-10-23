#!/bin/bash
# RE tooling: click at (x, y) in the original's 640x480 client area.
# Usage: tools/orig_click.sh X Y [hold_ms]
set -euo pipefail
W=$(xdotool search --name "^Insaniquarium Deluxe 1.1$" | while read w; do
  g=$(xwininfo -id "$w" | awk '/Width:/{w=$2}/Height:/{h=$2}END{print w"x"h}')
  [ "$g" = "640x480" ] && echo "$w"; done | head -1)
xdotool windowraise "$W"
read -r X Y < <(xwininfo -id "$W" | awk '/Absolute upper-left X/{x=$4}/Absolute upper-left Y/{y=$4}END{print x, y}')
xdotool mousemove $((X + $1)) $((Y + $2))
sleep 0.05
xdotool mousedown 1
sleep "$(echo "${3:-60} / 1000" | bc -l)"
xdotool mouseup 1
