#!/usr/bin/env python3
"""RE tooling: read constants from the inner game PE by virtual address.

Usage: pe_read.py <kind> <va> [<va> ...]
  kind: f64 | f32 | i32 | u32 | str
Example: pe_read.py f64 0x5c3d80 0x596760
Reads re/bin/Insaniquarium_game.exe (extract it with extract_inner_pe.py).
"""
import struct
import sys
from pathlib import Path

PE = Path(__file__).resolve().parent.parent / "re/bin/Insaniquarium_game.exe"


def sections(pe):
    e = struct.unpack_from("<I", pe, 0x3C)[0]
    n = struct.unpack_from("<H", pe, e + 6)[0]
    opt = struct.unpack_from("<H", pe, e + 20)[0]
    base = struct.unpack_from("<I", pe, e + 24 + 28)[0]
    out = []
    for i in range(n):
        o = e + 24 + opt + 40 * i
        va, raw_size, raw_ptr = (
            struct.unpack_from("<I", pe, o + 12)[0],
            struct.unpack_from("<I", pe, o + 16)[0],
            struct.unpack_from("<I", pe, o + 20)[0],
        )
        out.append((base + va, raw_size, raw_ptr))
    return out


def offset(secs, va):
    for start, size, ptr in secs:
        if start <= va < start + size:
            return ptr + va - start
    raise SystemExit(f"{va:#x} not in a file-backed section")


def main():
    kind, vas = sys.argv[1], [int(a, 16) for a in sys.argv[2:]]
    pe = PE.read_bytes()
    secs = sections(pe)
    fmt = {"f64": "<d", "f32": "<f", "i32": "<i", "u32": "<I"}
    for va in vas:
        o = offset(secs, va)
        if kind == "str":
            s = pe[o : pe.index(b"\0", o)].decode("latin-1")
            print(f"{va:#x}: {s!r}")
        else:
            print(f"{va:#x}: {struct.unpack_from(fmt[kind], pe, o)[0]!r}")


main()
