#!/bin/bash
# Frame comparison viewer: runs the frame tests (crates/insaniquarium-app/
# tests/frames.rs) and writes target/frames/index.html, one row per captured
# frame: ours | original | difference (mismatched pixels red, failing
# 32×32 regions outlined yellow). Usage: tools/frame_report.sh [--open]
set -uo pipefail
cd "$(dirname "$0")/.."
cargo test -q -p insaniquarium-app --test frames >/dev/null 2>&1
status=$?
out=target/frames
{
  echo '<!doctype html><meta charset="utf-8"><title>Frames vs original</title>'
  echo '<style>body{font:14px sans-serif;background:#222;color:#ddd;margin:1em}'
  echo 'h2{margin-top:2em}img{width:100%;max-width:1920px;image-rendering:pixelated}'
  echo '.ok{color:#7d7}.bad{color:#f77}.cap{margin:.6em 0 .2em}</style>'
  echo '<h1>Our rendering vs the original</h1>'
  echo '<p>Each row: <b>ours</b> | <b>original</b> | <b>difference</b> (red = pixels off by more than 40 per channel; yellow boxes = 32×32 regions with more than 5% of pixels off).</p>'
  for dir in "$out"/*/; do
    name=$(basename "$dir")
    [ -f "$dir/summary.tsv" ] || continue
    total=$(grep -c . "$dir/summary.tsv")
    good=$(awk -F'\t' '$3=="true"' "$dir/summary.tsv" | grep -c .)
    echo "<h2>$name — $good / $total frames match</h2>"
    while IFS=$'\t' read -r png tick ok summary; do
      cls=bad; verdict=differs; [ "$ok" = true ] && { cls=ok; verdict=matches; }
      echo "<div class=cap><span class=$cls>$verdict</span> · $png · tick $tick · $summary</div>"
      echo "<a href=\"$name/$png\"><img loading=lazy src=\"$name/$png\"></a>"
    done < "$dir/summary.tsv"
  done
} > "$out/index.html"
echo "wrote $out/index.html (tests $( [ $status -eq 0 ] && echo passed || echo FAILED ))"
[ "${1:-}" = --open ] && xdg-open "$out/index.html" >/dev/null 2>&1 &
exit $status
