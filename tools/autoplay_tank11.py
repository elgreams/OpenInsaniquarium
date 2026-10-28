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
QUANTITY_BTN = (BTN_X[2] + 29, 33)
CARNIVORE_BTN = (BTN_X[3] + 29, 33)
# Later levels (1-3+): how many guppies to buy, and whether to buy a
# carnivore once rich (`--guppies N`, `--carnivore`).
MAX_GUPPIES = int(sys.argv[sys.argv.index("--guppies") + 1]) if "--guppies" in sys.argv else 1
BUY_CARNIVORE = "--carnivore" in sys.argv
# Guppies needed before the carnivore is bought (`--carnivore-min N`).
MIN_GUPPIES_FOR_CARNIVORE = (int(sys.argv[sys.argv.index("--carnivore-min") + 1])
                             if "--carnivore-min" in sys.argv else 4)
# Feed the carnivore: once one is up, keep buying guppies (small ones are
# its food) whenever affordable (`--feed-carnivore`).
FEED_CARNIVORE = "--feed-carnivore" in sys.argv
# Feed hungry guppies before collecting coins (`--feed-first`).
FEED_FIRST = "--feed-first" in sys.argv
EGG_BTN = (BTN_X[6] + 29, 33)
WEAPON_BTN = (BTN_X[5] + 29, 33)
# Buy weapon upgrades (up to N) once revealed and affordable (`--weapon N`).
MAX_WEAPON = int(sys.argv[sys.argv.index("--weapon") + 1]) if "--weapon" in sys.argv else 0
# Keep at least N live guppies, buying replacements (`--keep N`).
KEEP_GUPPIES = int(sys.argv[sys.argv.index("--keep") + 1]) if "--keep" in sys.argv else 0
# Buy food quantity upgrades (up to N) once revealed (`--quantity N`).
MAX_QUANTITY = int(sys.argv[sys.argv.index("--quantity") + 1]) if "--quantity" in sys.argv else 0
RESUME = (312, 258)
# Egg price by level (shop-bar.md): 1-1 150, 1-2 500.
EGG_PRICE = int(sys.argv[sys.argv.index("--egg") + 1]) if "--egg" in sys.argv else 150
# Never buy egg pieces (`--no-egg`): for recordings that must not end the
# level (pet zoos on 1-2, where the egg costs 500, not the 1-1 default).
NO_EGG = "--no-egg" in sys.argv
# Leave coins alone (`--no-coins`): pets that move coins (Nimbus) need them.
NO_COINS = "--no-coins" in sys.argv
# Drop food near the floor (`--feed-low`): guppies follow it down, where
# floor pets (Rhubarb) can reach them.
FEED_LOW = "--feed-low" in sys.argv
# Click a charged Amp three times once N guppies are in (`--zap-amp N`).
ZAP_AMP = int(sys.argv[sys.argv.index("--zap-amp") + 1]) if "--zap-amp" in sys.argv else 0
# Presto (`--presto 0,11,C,19`): right-click the Presto pet once it is
# recharged and pick these pets in turn in the PRESTO CHANGE-O dialog ("C"
# = CANCEL). After each change, one early right-click (BUZZER).
PRESTO = (sys.argv[sys.argv.index("--presto") + 1].split(",")
          if "--presto" in sys.argv else [])
PET_DIALOG = 0x1f
# Feed only when the hungriest guppy is below N (`--hunger N`, default 450):
# low values keep guppies hungry enough to drink Nostradamus's potions.
HUNGER = int(sys.argv[sys.argv.index("--hunger") + 1]) if "--hunger" in sys.argv else 450
# Leave aliens alone (`--no-shoot`): pets that fight them (Stanley) do it.
NO_SHOOT = "--no-shoot" in sys.argv
# Walter (`--walter N`): click him N times, ~2 s apart, preferring moments
# with a guppy beside him (pet-walter.md#clicks-slot-54-0x004fbe20--0x004fb680).
WALTER = int(sys.argv[sys.argv.index("--walter") + 1]) if "--walter" in sys.argv else 0


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


def click(win, gx, gy, button=1):
    w, ox, oy = win
    subprocess.run(["xdotool", "windowactivate", "--sync", w])
    subprocess.run(["xdotool", "mousemove", str(ox + gx), str(oy + gy)])
    time.sleep(0.05)
    subprocess.run(["xdotool", "click", str(button)])
    # Park the cursor outside the window so it can't hover-trigger anything.
    subprocess.run(["xdotool", "mousemove", str(ox - 200), str(oy + 100)])


def read_state():
    try:
        lines = open(GPOS).read().splitlines()
    except OSError:
        return None
    st = {"guppies": [], "coins": [], "aliens": [], "revealed": [0] * 12, "dialogs": [],
          "oscars": []}
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
        elif p[0] == "P":
            st.setdefault("pets", []).append(tuple(map(int, p[1:6])))
            if len(p) >= 8 and int(p[6]):
                st["presto"] = (int(p[2]), int(p[3]), int(p[7]), int(p[1]))
        elif p[0] == "Q" and len(p) >= 6 and int(p[4]) and "presto" not in st:
            st["presto"] = (int(p[2]), int(p[3]), int(p[5]), int(p[1]))
        elif p[0] == "O":
            st["oscars"].append(tuple(map(int, p[1:4])))
        elif p[0] == "A":
            st["aliens"].append(tuple(map(int, p[1:3])))
        elif p[0] == "D" and len(p) >= 6:
            st["dialogs"].append(tuple(map(int, p[1:6])))
        elif p[0] == "R" and len(p) > 1:
            st["revealed"] = [int(p[1][i:i + 2], 16) for i in range(0, 24, 2)]
    return st if "paused" in st else None


def main():
    win = window()
    start = time.time()
    last_feed = 0.0
    bought_guppies = 0
    bought_quality = False
    bought_carnivore = False
    bought_weapon = 0
    bought_quantity = 0
    picks = list(PRESTO)
    presto_at = time.time() + 5  # next right-click on the Presto pet
    buzzed = True
    punches = 0
    punch_at = time.time() + 4
    while time.time() - start < MAX_S:
        st = read_state()
        if st is None:
            time.sleep(0.3)
            continue
        if st["eggs"] >= 4:
            print("level complete")
            return
        now = time.time()
        if any(d[0] == PET_DIALOG for d in st["dialogs"]):
            # PRESTO CHANGE-O: pick the next pet (pet-presto.md#the-petdialog-0x005355c0).
            pick = picks.pop(0) if picks else "C"
            if pick == "C":
                click(win, 320, 46)
            else:
                i = int(pick)
                bx, by = ((10, 120 + 50 * i) if i <= 6 else
                          (62 + 52 * (i - 7), 420) if i <= 17 else (582, 370 - 50 * (i - 18)))
                click(win, bx + 25, by + 25)
            print("presto pick", pick, flush=True)
            presto_at = now + 3
            buzzed = pick == "C"
            time.sleep(0.6)
            continue
        if picks and "presto" in st and now >= presto_at and not st["aliens"] and not st["dialogs"]:
            px, py, recharge, pid = st["presto"]
            w = 160 if pid == 16 else 80
            if recharge == 0 or not buzzed:
                # Early once (BUZZER, "Recharging..."), then when ready.
                click(win, px + w // 2, py + 40, button=3)
                print("presto right-click", pid, "recharge", recharge, flush=True)
                buzzed = True
                presto_at = now + (2 if recharge == 0 else 4)
                time.sleep(0.35)
                continue
        walter = [q for q in st.get("pets", []) if q[0] == 23]
        if WALTER and punches < WALTER and walter and now >= punch_at and not st["dialogs"]:
            _, wx, wy, _, _ = walter[0]
            near = any(abs(g[0] - wx) < 120 and abs(g[1] - wy) < 60 for g in st["guppies"])
            if near or now >= punch_at + 6:
                click(win, wx + 40, min(max(wy + 35, 70), 470))
                print("walter click", punches, "near" if near else "", flush=True)
                punches += 1
                punch_at = now + 2.2
                time.sleep(0.35)
                continue
        if st["dialogs"]:
            # A board dialog (DANGER!, ...): click its button, centred along
            # the bottom (framework/dialog.md: inset 36, button 33 high).
            _, dx, dy, dw, dh = st["dialogs"][0]
            click(win, dx + dw // 2, dy + dh - 36 - 16)
            time.sleep(0.4)
            continue
        amp = [q for q in st.get("pets", []) if q[0] == 16 and q[3] >= q[4]]
        if st["paused"]:
            click(win, *RESUME)
        elif ZAP_AMP and amp and not st["aliens"] and len(st["guppies"]) >= ZAP_AMP:
            # A charged Amp (`--zap-amp N`, once N guppies are in): three
            # clicks on his middle zap the tank (pet-amp.md#clicks).
            _, ax, ay, _, _ = amp[0]
            for _ in range(3):
                click(win, min(max(ax + 80, 10), 630), min(max(ay + 40, 70), 470))
                time.sleep(0.3)
        elif st["aliens"] and not NO_SHOOT:
            # Shoot the alien: click its centre (160x160 widget).
            # A burst of quick clicks (one hit lands per 10-tick cooldown;
            # alien-sylvester.md#hits), then re-read its position.
            ax, ay = st["aliens"][0]
            w, ox, oy = win
            subprocess.run(["xdotool", "mousemove", str(ox + min(max(ax + 80, 10), 630)),
                            str(oy + min(max(ay + 80, 45), 470)),
                            "click", "--repeat", "4", "--delay", "70", "1"])
            continue
        elif (BUY_CARNIVORE and not bought_carnivore and st["revealed"][4]
              and st["money"] >= 1100 and len(st["guppies"]) >= MIN_GUPPIES_FOR_CARNIVORE):
            # First: on 1-3+ the carnivore reveals the weapon and egg buttons.
            click(win, *CARNIVORE_BTN)
            bought_carnivore = True
        elif (KEEP_GUPPIES and st["rev_guppy"] and len(st["guppies"]) < KEEP_GUPPIES
              and st["money"] >= 120):
            # Replace guppies lost to aliens.
            click(win, *GUPPY_BTN)
        elif st["revealed"][10] and bought_weapon < MAX_WEAPON and st["money"] >= 1100:
            click(win, *WEAPON_BTN)
            bought_weapon += 1
        elif st["revealed"][2] and not bought_quality and st["money"] >= 220:
            click(win, *QUALITY_BTN)
            bought_quality = True
        elif st["revealed"][3] and bought_quantity < MAX_QUANTITY and st["money"] >= 400:
            click(win, *QUANTITY_BTN)
            bought_quantity += 1
        elif not NO_EGG and st["rev_egg"] and st["money"] >= EGG_PRICE:
            click(win, *EGG_BTN)
        elif (st["rev_guppy"] and bought_guppies < MAX_GUPPIES and st["money"] >= 120
              and (not st["rev_egg"] or MAX_GUPPIES > 1)):
            click(win, *GUPPY_BTN)
            bought_guppies += 1
        elif (FEED_CARNIVORE and st["oscars"] and st["money"] >= 300
              and min(o[2] for o in st["oscars"]) < 700):
            # A hungry carnivore: drop a fresh (small) guppy in for it.
            click(win, *GUPPY_BTN)
        elif (FEED_FIRST and st["guppies"] and st["foods"] <= bought_quantity
              and now - last_feed > 0.8 and st["money"] >= 5
              and min(g[2] for g in st["guppies"]) < HUNGER):
            # Pets that move coins (Nimbus) can keep the coin branch busy
            # forever: feed hungry guppies first.
            g = min(st["guppies"], key=lambda g: g[2])
            y = 385 if FEED_LOW else min(max(g[1] - 20, 75), 390)
            click(win, min(max(g[0] + 40, 40), 580), y)
            last_feed = now
        elif not NO_COINS and any(c[2] == 0 for c in st["coins"]):
            c = max((c for c in st["coins"] if c[2] == 0), key=lambda c: c[1])
            click(win, c[0] + 36, min(c[1] + 36, 440))
        elif (st["guppies"] and st["foods"] <= bought_quantity and now - last_feed > 0.8
              and st["money"] >= 5):
            g = min(st["guppies"], key=lambda g: g[2])
            if g[2] < HUNGER:
                x = min(max(g[0] + 40, 40), 580)
                y = min(max(g[1] - 20, 75), 390)
                click(win, x, y)
                last_feed = now
        time.sleep(0.35)
    print("timed out")


if __name__ == "__main__":
    main()
