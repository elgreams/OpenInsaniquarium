#!/usr/bin/env python3
"""RE tooling only: play tank 1-1 of the *original* game by clicking with
xdotool, driven by the board state `tools/ce_capture.lua` publishes to
CAPTURE_GPOS. Used to record full-level traces for the differential harness.

Usage: autoplay_tank11.py /tmp/gpos [--max-seconds N]

Priorities each step: resume if paused; buy an egg piece when affordable;
buy a second guppy early; collect the lowest uncollected coin; feed the
hungriest guppy. Shop button rects come from docs/re/sim/shop-bar.md.
"""
import os
import subprocess
import sys
import time

GPOS = sys.argv[1] if len(sys.argv) > 1 else "/tmp/gpos"
MAX_S = float(sys.argv[sys.argv.index("--max-seconds") + 1]) if "--max-seconds" in sys.argv else 900

GUPPY_BTN = (18 + 29, 3 + 30)
EGG_BTN = (436 + 29, 3 + 30)
RESUME = (312, 258)


def window():
    out = subprocess.run(["xdotool", "search", "--name", "Insaniquarium Deluxe 1.1"],
                         capture_output=True, text=True).stdout.split()
    for w in out:
        info = subprocess.run(["xwininfo", "-id", w], capture_output=True, text=True).stdout
        if "Width: 640" in info:
            x = int(info.split("Absolute upper-left X:")[1].split()[0])
            y = int(info.split("Absolute upper-left Y:")[1].split()[0])
            return w, x, y
    raise SystemExit("game window not found")


def click(win, gx, gy):
    w, ox, oy = win
    subprocess.run(["xdotool", "windowactivate", "--sync", w])
    subprocess.run(["xdotool", "mousemove", str(ox + gx), str(oy + gy)])
    time.sleep(0.05)
    subprocess.run(["xdotool", "click", "1"])
    # Park the cursor outside the window so it can't hover-trigger anything.
    subprocess.run(["xdotool", "mousemove", str(ox - 200), str(oy + 100)])


def read_state():
    try:
        lines = open(GPOS).read().splitlines()
    except OSError:
        return None
    st = {"guppies": [], "coins": []}
    for ln in lines:
        p = ln.split()
        if not p:
            continue
        if p[0] == "S" and len(p) >= 7:
            st.update(paused=int(p[1]), money=int(p[2]), eggs=int(p[3]),
                      rev_guppy=int(p[4]), rev_egg=int(p[5]), foods=int(p[6]))
        elif p[0] == "G":
            st["guppies"].append(tuple(map(int, p[1:4])))
        elif p[0] == "C":
            st["coins"].append(tuple(map(int, p[1:4])))
    return st if "paused" in st else None


def main():
    win = window()
    start = time.time()
    last_feed = 0.0
    bought_guppies = 0
    while time.time() - start < MAX_S:
        st = read_state()
        if st is None:
            time.sleep(0.3)
            continue
        if st["eggs"] >= 4:
            print("level complete")
            return
        now = time.time()
        if st["paused"]:
            click(win, *RESUME)
        elif st["rev_egg"] and st["money"] >= 150:
            click(win, *EGG_BTN)
        elif st["rev_guppy"] and bought_guppies < 1 and st["money"] >= 120 and not st["rev_egg"]:
            click(win, *GUPPY_BTN)
            bought_guppies += 1
        elif any(c[2] == 0 for c in st["coins"]):
            c = max((c for c in st["coins"] if c[2] == 0), key=lambda c: c[1])
            click(win, c[0] + 36, min(c[1] + 36, 440))
        elif st["guppies"] and st["foods"] == 0 and now - last_feed > 1.2 and st["money"] >= 5:
            g = min(st["guppies"], key=lambda g: g[2])
            if g[2] < 450:
                x = min(max(g[0] + 40, 40), 580)
                y = min(max(g[1] - 20, 75), 390)
                click(win, x, y)
                last_feed = now
        time.sleep(0.35)
    print("timed out")


if __name__ == "__main__":
    main()
