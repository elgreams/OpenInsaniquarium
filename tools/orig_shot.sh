#!/bin/bash
# RE tooling: screenshot the original game's 640x480 client area (by X11
# window id, found by title) to a PNG. Usage: tools/orig_shot.sh out.png
set -euo pipefail
W=$(xdotool search --name "^Insaniquarium Deluxe 1.1$" | while read w; do
  g=$(xwininfo -id "$w" | awk '/Width:/{w=$2}/Height:/{h=$2}END{print w"x"h}')
  [ "$g" = "640x480" ] && echo "$w"; done | head -1)
xdotool windowraise "$W"; sleep 0.05
read -r X Y < <(xwininfo -id "$W" | awk '/Absolute upper-left X/{x=$4}/Absolute upper-left Y/{y=$4}END{print x, y}')
ffmpeg -loglevel error -f x11grab -draw_mouse 0 -video_size 640x480 -i ":0.0+$X,$Y" -frames:v 1 -y "$1"
