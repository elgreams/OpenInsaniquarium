#!/usr/bin/env python3
"""Extract the game PE embedded in a PopCap-DRM-wrapped executable.

Usage: extract_inner_pe.py <wrapped.exe> <out.exe>

The Steam/PopCap build of Insaniquarium.exe is a DRM launcher stub; the
actual game is a plain (unencrypted) PE stored in the overlay after the
`!popcapdrmprotect!` marker. This copies those bytes out unchanged so
static analysis targets the real game code. Output is for local RE only
(write it under re/, which is gitignored). Never distribute it.
"""

import struct
import sys

MARKER = b"!popcapdrmprotect!"


def main():
    src, dst = sys.argv[1], sys.argv[2]
    data = open(src, "rb").read()
    m = data.find(MARKER)
    if m < 0:
        sys.exit("no PopCap DRM marker found")
    mz = data.find(b"MZ", m)
    pe = mz + struct.unpack_from("<I", data, mz + 0x3C)[0]
    if data[pe : pe + 4] != b"PE\0\0":
        sys.exit("embedded PE header not found")
    nsec = struct.unpack_from("<H", data, pe + 6)[0]
    opt = struct.unpack_from("<H", data, pe + 20)[0]
    end = 0
    off = pe + 24 + opt
    for _ in range(nsec):
        _, _, rsize, raw = struct.unpack_from("<IIII", data, off + 8)
        end = max(end, raw + rsize)
        off += 40
    open(dst, "wb").write(data[mz : mz + end])
    print("extracted %d bytes from offset 0x%x" % (end, mz))


if __name__ == "__main__":
    main()
