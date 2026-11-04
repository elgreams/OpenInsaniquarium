#!/usr/bin/env python3
"""RE tooling: find a class vtable by RTTI name and dump key slots.

Usage: vtable_read.py <rtti_name> [<slot> ...]
Example: vtable_read.py Grubber 0 22 77 81 82 83 84 85 86 87
If no slots given, dumps all slots.
"""
import struct, sys
from pathlib import Path

PE = Path(__file__).resolve().parent.parent / "re/bin/Insaniquarium_game.exe"


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return
    name = sys.argv[1]
    slots = [int(s) for s in sys.argv[2:]] if len(sys.argv) > 2 else None

    pe = PE.read_bytes()
    e = struct.unpack_from("<I", pe, 0x3C)[0]
    n = struct.unpack_from("<H", pe, e + 6)[0]
    opt = struct.unpack_from("<H", pe, e + 20)[0]
    base = struct.unpack_from("<I", pe, e + 24 + 28)[0]
    secs = []
    for i in range(n):
        o = e + 24 + opt + 40 * i
        va = struct.unpack_from("<I", pe, o + 12)[0]
        raw_size = struct.unpack_from("<I", pe, o + 16)[0]
        raw_ptr = struct.unpack_from("<I", pe, o + 20)[0]
        secs.append((base + va, raw_size, raw_ptr))

    def file_off(va):
        for start, size, ptr in secs:
            if start <= va < start + size:
                return ptr + va - start
        return None

    def va_from_file(fo):
        for start, size, ptr in secs:
            if ptr <= fo < ptr + size:
                return start + (fo - ptr)
        return None

    # Find RTTI type descriptor
    target = f".?AV{name}@Sexy@@".encode()
    idx = pe.find(target)
    if idx < 0:
        print(f"RTTI for {name} not found")
        return
    td_va = va_from_file(idx - 8)
    print(f"{name} type descriptor: {td_va:#010x}")

    # Find COL
    td_bytes = struct.pack("<I", td_va)
    pos = 0
    while True:
        pos = pe.find(td_bytes, pos)
        if pos < 0:
            break
        if pos >= 12 and struct.unpack_from("<I", pe, pos - 12)[0] == 0:
            col_va = va_from_file(pos - 12)
            if col_va and 0x550000 <= col_va <= 0x600000:
                # Find vtable
                col_bytes = struct.pack("<I", col_va)
                vpos = 0
                while True:
                    vpos = pe.find(col_bytes, vpos)
                    if vpos < 0:
                        break
                    vt_start = vpos + 4
                    vt_va = va_from_file(vt_start)
                    if vt_va and 0x590000 <= vt_va <= 0x5a0000:
                        nslots = 0
                        for s in range(120):
                            entry = struct.unpack_from("<I", pe, vt_start + s * 4)[0]
                            if entry < 0x400000 or entry > 0x600000:
                                break
                            nslots = s + 1
                        print(f"{name} vtable: {vt_va:#010x} ({nslots} slots)")
                        dump = slots if slots else list(range(nslots))
                        for s in dump:
                            if s < nslots:
                                entry = struct.unpack_from("<I", pe, vt_start + s * 4)[0]
                                print(f"  slot {s:3d}: {entry:#010x}")
                    vpos += 1
        pos += 1


main()
