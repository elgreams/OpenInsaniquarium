#!/usr/bin/env python3
"""RE tooling only: put a profile of the *original* game at a given level
with given pets owned, so a level can be recorded without playing up to it.

Usage: set_profile.py <user1.dat> --level T-L [--shells N]

Layout of user1.dat (from diffs between saves; docs/re/screens/
hatch-screen.md#profile-fields-touched has the in-memory profile):
  +0x04 i32 shells
  +0x08 i32 tank, +0x0c i32 level
  +0x11 u8  has progress
  +0x3a u8[24] one byte per pet won (not what the game uses for owned)
The file has no checksum (its last word is the same in every save). The
game rewrites the file on quit; a saved board (adv<N>.dat) resumes the old
level, so delete it to start the new one fresh.

Confirmed (2026-10-26): set to 3-2, the game owned all 11 pets of the
levels before it (Stinky through Seymour; profile+0x18 = 11) whatever
+0x3a said, and showed the pet screen to choose 3. So any level can be
recorded with any 3 of its earlier pets, chosen on the pet screen.
"""
import struct
import sys

path = sys.argv[1]
args = sys.argv[2:]
b = bytearray(open(path, "rb").read())
if "--level" in args:
    t, l = map(int, args[args.index("--level") + 1].split("-"))
    struct.pack_into("<ii", b, 8, t, l)
    b[0x11] = 1
if "--shells" in args:
    struct.pack_into("<i", b, 4, int(args[args.index("--shells") + 1]))
open(path, "wb").write(b)
print("tank", struct.unpack_from("<i", b, 8)[0], "level", struct.unpack_from("<i", b, 12)[0])
