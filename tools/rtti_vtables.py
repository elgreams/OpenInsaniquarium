#!/usr/bin/env python3
"""Dump MSVC x86 RTTI vtables from a PE32 image.

Usage: rtti_vtables.py <exe> [--class SUBSTR]

Prints one block per vtable: class name (demangled, simple), this-offset,
vtable VA, base classes, and each slot's target VA. Output is derived from
the user's own binary and should be written under re/ (gitignored).
"""

import struct
import sys


def load_pe(path):
    data = open(path, "rb").read()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    nsec = struct.unpack_from("<H", data, pe + 6)[0]
    opt_size = struct.unpack_from("<H", data, pe + 20)[0]
    base = struct.unpack_from("<I", data, pe + 24 + 28)[0]
    secs = []
    off = pe + 24 + opt_size
    for i in range(nsec):
        name = data[off : off + 8].rstrip(b"\0").decode()
        vsize, va, rsize, raw = struct.unpack_from("<IIII", data, off + 8)
        secs.append((name, base + va, max(vsize, rsize), raw, rsize))
        off += 40
    return data, base, secs


class Image:
    def __init__(self, path):
        self.data, self.base, self.secs = load_pe(path)

    def sec(self, name):
        return next(s for s in self.secs if s[0] == name)

    def contains(self, name, va):
        _, start, size, _, _ = self.sec(name)
        return start <= va < start + size

    def off(self, va):
        for _, start, size, raw, rsize in self.secs:
            if start <= va < start + size:
                d = va - start
                return raw + d if d < rsize else None
        return None

    def u32(self, va):
        o = self.off(va)
        if o is None or o + 4 > len(self.data):
            return None
        return struct.unpack_from("<I", self.data, o)[0]

    def cstr(self, va):
        o = self.off(va)
        end = self.data.index(b"\0", o)
        return self.data[o:end].decode("latin-1")


def demangle(raw):
    body = raw[4:]
    if body.endswith("@@"):
        body = body[:-2]
    return "::".join(reversed(body.split("@")))


def main():
    path = sys.argv[1]
    flt = None
    if "--class" in sys.argv:
        flt = sys.argv[sys.argv.index("--class") + 1].lower()
    img = Image(path)

    # TypeDescriptors: {pVFTable, spare, name[]} with name ".?AV..." / ".?AU..."
    tds = {}
    for name, start, size, raw, rsize in img.secs:
        blob = img.data[raw : raw + rsize]
        i = 0
        while True:
            i = blob.find(b".?A", i)
            if i < 0:
                break
            if blob[i + 3 : i + 4] in (b"V", b"U"):
                va = start + i - 8
                s = img.cstr(start + i)
                tds[va] = demangle(s)
            i += 3

    rd_name, rd_start, rd_size, _, _ = img.sec(".rdata")

    def chd_bases(chd):
        n = img.u32(chd + 8)
        arr = img.u32(chd + 12)
        out = []
        if n is None or arr is None or n > 64:
            return out
        for k in range(1, n):
            bcd = img.u32(arr + 4 * k)
            if bcd is None:
                continue
            td = img.u32(bcd)
            out.append(tds.get(td, "?%08x" % (td or 0)))
        return out

    cols = {}
    for va in range(rd_start, rd_start + rd_size - 20, 4):
        if img.u32(va) != 0:
            continue
        td = img.u32(va + 12)
        chd = img.u32(va + 16)
        if td in tds and chd and img.contains(".rdata", chd):
            cols[va] = (tds[td], img.u32(va + 4), chd)

    rows = []
    for va in range(rd_start, rd_start + rd_size - 8, 4):
        col = img.u32(va)
        if col not in cols:
            continue
        vt = va + 4
        if not img.contains(".text", img.u32(vt) or 0):
            continue
        cls, thisoff, chd = cols[col]
        slots = []
        k = 0
        while True:
            t = img.u32(vt + 4 * k)
            if t is None or not img.contains(".text", t):
                break
            if k > 0 and (vt + 4 * k - 4 + 4) != vt and img.u32(vt + 4 * k) in cols:
                break
            # stop if next dword is a COL pointer for another vtable
            if k > 0 and img.u32(vt + 4 * k - 4 + 4) in cols:
                break
            slots.append(t)
            k += 1
        rows.append((cls, thisoff, vt, chd_bases(chd), slots))

    for cls, thisoff, vt, bases, slots in sorted(rows, key=lambda r: (r[0], r[1])):
        if flt and flt not in cls.lower():
            continue
        print("%s  this+0x%x  vftable=%08x  slots=%d" % (cls, thisoff, vt, len(slots)))
        if bases:
            print("  bases: " + ", ".join(bases))
        for k, t in enumerate(slots):
            print("  [%2d] +0x%02x  %08x" % (k, 4 * k, t))


if __name__ == "__main__":
    main()
