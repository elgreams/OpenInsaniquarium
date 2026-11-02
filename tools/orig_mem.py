#!/usr/bin/env python3
"""RE tooling only: read/write the running original's board through
/proc/<pid>/mem (needs sudo). Works while a CE capture holds CE's Lua engine.

Usage: sudo python3 tools/orig_mem.py <pid> [money=N] [cd=N] [owned=all]
                                          [potion] [quiet]
  (no args)  print clock, wave countdown, money, pets, guppies, food
  money=N    board money (+0x3f0)
  cd=N       wave countdown (+0x2c0)
  owned=all  profile owns all 24 pets (+0x00..+0x17 = 1, count +0x18 = 24):
             with more than 3 owned, Adventure opens the pet screen
  potion     Nostradamus (id 21): push his throw clock to 5 ticks before a potion
"""
import struct
import sys

pid = int(sys.argv[1])
args = sys.argv[2:]
f = open(f"/proc/{pid}/mem", "r+b", buffering=0)


def rd(a):
    f.seek(a)
    return struct.unpack("<i", f.read(4))[0]


def ptr(a):
    return rd(a) & 0xFFFFFFFF


def wr(a, v):
    f.seek(a)
    f.write(struct.pack("<i", v))


def vec(v):
    return range(ptr(v + 4), ptr(v + 8), 4)


app = ptr(0x5EB680)
prof = ptr(app + 0x8B8)
b = ptr(app + 0x730)
for a in args:
    if a == "owned=all":
        f.seek(prof)
        f.write(bytes([1] * 24))
        wr(prof + 0x18, 24)
        print("owned all 24")
if not b or "quiet" in args and not any(a[:3] in ("mon", "cd=") for a in args):
    sys.exit(0)
for a in args:
    if a.startswith("money="):
        wr(b + 0x3F0, int(a[6:]))
    elif a.startswith("cd="):
        wr(b + 0x2C0, int(a[3:]))
if "quiet" in args:
    sys.exit(0)
print("clock", rd(b + 0x28), "countdown", rd(b + 0x2C0), "money", rd(b + 0x3F0))
for p in vec(ptr(b + 0xB4)):
    o = ptr(p)
    print("pet", rd(o + 0x234), "t", rd(o + 0x220), "iv", rd(o + 0x224), "w", rd(o + 0x30), rd(o + 0x34))
    if "potion" in args and rd(o + 0x234) == 21 and rd(o + 0x240) == 0:
        wr(o + 0x220, rd(o + 0x224) - 5)
        print(" pushed")
if "presto_near_walter" in args:
    # Move the Presto-flagged pet next to Walter so the glove can hit it.
    # The fist reaches ~100 px from Walter's centre; put the flagged pet
    # 60 px to his left/right (well within glove range) and at the same y.
    walter_x, walter_y, flagged = None, None, None
    for p in vec(ptr(b + 0xB4)):
        o = ptr(p)
        if rd(o + 0x234) == 23:
            walter_x, walter_y = rd(o + 0x30), rd(o + 0x34)
        if (rd(o + 0x230) & 0xFF) and rd(o + 0x234) != 19:
            flagged = o
    if walter_x is not None and flagged is not None:
        fx = max(10, min(walter_x - 60, 460))
        wr(flagged + 0x30, fx)
        wr(flagged + 0x34, walter_y)
        print(f"flagged pet -> ({fx},{walter_y})")
    else:
        print(f"walter={walter_x},{walter_y} flagged={flagged}")
for p in vec(ptr(b + 0xA0)):
    o = ptr(p)
    print("guppy hunger", rd(o + 0x9C), "stage", rd(o + 0x1A0))
