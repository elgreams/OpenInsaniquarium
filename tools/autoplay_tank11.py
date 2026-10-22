#!/usr/bin/env python3
"""RE tooling only: play tank 1-1 / 1-2 of the *original* game by clicking with
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

BTN_X = [18, 87, 144, 217, 290, 363, 436]
GUPPY_BTN = (BTN_X[0] + 29, 33)
QUALITY_BTN = (BTN_X[1] + 29, 33)
EGG_BTN = (BTN_X[6] + 29, 33)
RESUME = (312, 258)
# Egg price by level (shop-bar.md): 1-1 150, 1-2 500.
EGG_PRICE = int(sys.argv[sys.argv.index("--egg") + 1]) if "--egg" in sys.argv else 150


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
    st = {"guppies": [], "coins": [], "aliens": [], "revealed": [0] * 12}
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
        elif p[0] == "A":
            st["aliens"].append(tuple(map(int, p[1:3])))
        elif p[0] == "R" and len(p) > 1:
            st["revealed"] = [int(p[1][i:i + 2], 16) for i in range(0, 24, 2)]
    return st if "paused" in st else None


def main():
    win = window()
    start = time.time()
    last_feed = 0.0
    bought_guppies = 0
    bought_quality = False
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
        elif st["aliens"]:
            # Shoot the alien: click its centre (160x160 widget).
            ax, ay = st["aliens"][0]
            click(win, min(max(ax + 80, 10), 630), min(max(ay + 80, 45), 470))
            time.sleep(0.05)
            continue
        elif st["revealed"][2] and not bought_quality and st["money"] >= 220:
            click(win, *QUALITY_BTN)
            bought_quality = True
        elif st["rev_egg"] and st["money"] >= EGG_PRICE:
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
