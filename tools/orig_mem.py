#!/usr/bin/env python3
"""RE tooling only: read/write the running original's board through
/proc/<pid>/mem (needs sudo). Works while a CE capture holds CE's Lua engine.

Usage: sudo python3 tools/orig_mem.py <pid> [money=N] [cd=N] [owned=all]
                                          [potion] [alien_hp=N] [quiet]
  (no args)  print clock, wave countdown, money, pets, guppies, food
  money=N    board money (+0x3f0)
  cd=N       wave countdown (+0x2c0)
  owned=all  profile owns all 24 pets (+0x00..+0x17 = 1, count +0x18 = 24):
             with more than 3 owned, Adventure opens the pet screen
  potion     Nostradamus (id 21): push his throw clock to 5 ticks before a potion
  alien_hp=N set the first alien's health (+0x1f0, double) to N
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


def wrd(a, v):
    """Write a double (f64) at address a."""
    f.seek(a)
    f.write(struct.pack("<d", v))


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
if not b and not any(a.startswith("capture=") for a in args):
    sys.exit(0)
if "quiet" in args and not any(a[:3] in ("mon", "cd=", "ali") for a in args):
    sys.exit(0)
for a in args:
    if a.startswith("money="):
        wr(b + 0x3F0, int(a[6:]))
    elif a.startswith("cd="):
        wr(b + 0x2C0, int(a[3:]))
    elif a.startswith("alien_hp="):
        hp = float(a[9:])
        for p in vec(ptr(b + 0xB8)):
            o = ptr(p)
            wrd(o + 0x1F0, hp)
            print(f"alien @ 0x{o:08x} hp -> {hp}")
            break  # first alien only
if "bomb" in args:
    # Push Shrapnel's (pet 11) bomb counter to interval - 5 for a quick drop.
    for p in vec(ptr(b + 0xB4)):
        o = ptr(p)
        if rd(o + 0x234) == 11:
            iv = rd(o + 0x224)
            wr(o + 0x220, iv - 5)
            print(f"shrapnel bomb counter -> {iv - 5}/{iv}")
            break
if "quiet" in args:
    sys.exit(0)
# capture=TICKS: run a full trace capture (bypass CE).
cap_arg = [a for a in args if a.startswith("capture=")]
if cap_arg:
    import json, time, os
    max_ticks = int(cap_arg[0].split("=")[1])
    out_path = "/tmp/capture.jsonl"
    gpos_path = "/tmp/gpos"
    cap_out = [a for a in args if a.startswith("out=")]
    if cap_out: out_path = cap_out[0].split("=", 1)[1]

    APP_PTR = 0x5EB680
    GLOBAL_RAND = 0x5EB928
    LISTS = [
        (0xa0,"guppy"),(0xa4,"oscar"),(0xd4,"ultra"),(0xcc,"gekko"),(0xd0,"penta"),
        (0xc4,"grubber"),(0xc0,"breeder"),(0xb0,"pet_other"),(0xb4,"pet_fish"),
        (0xb8,"alien"),(0xbc,"alien_boss"),(0xf4,"bilaterus"),(0xa8,"coin"),
        (0xe8,"coin_b"),(0xec,"coin_c"),(0xac,"food"),(0xc8,"larva"),(0xdc,"missile"),
        (0xe0,"missile_b"),(0xd8,"shot"),(0x98,"dead_fish"),(0x9c,"dead_alien"),
        (0xe4,"shadow"),(0xf0,"warp"),(0xf8,"special"),
    ]
    RAW_LISTS = [
        (0xac,"food",0x200),(0xa8,"coin",0x200),(0xe8,"coin_b",0x200),(0xec,"coin_c",0x200),
        (0xb8,"alien",0x210),(0xbc,"alien_b",0x210),(0xd8,"shot",0x170),(0x9c,"dead_alien",0x1a8),
        (0x98,"dead_fish",0x1a8),(0xf0,"warp",0x170),(0xb0,"pet_other",0x1d0),(0xb4,"pet_fish",0x268),
        (0xe4,"shadow",0x170),(0xa4,"oscar",0x230),(0xd0,"penta",0x1b0),
        (0xcc,"gekko",0x230),(0xc4,"grubber",0x1b0),(0xc0,"breeder",0x230),(0xc8,"larva",0x170),
        (0xdc,"missile",0x200),(0xe0,"missile_b",0x200),(0xf8,"special",0x200),
    ]
    SCREENS = [
        (0x730,"board"),(0x814,"hatch"),(0x80c,"pets"),(0x81c,"interlude"),
        (0x818,"bonus"),(0x738,"selector"),(0x834,"help"),
    ]

    def rdd(a):
        f.seek(a); return struct.unpack("<d", f.read(8))[0]
    def rdf(a):
        f.seek(a); return struct.unpack("<f", f.read(4))[0]
    def rb(a, n):
        f.seek(a); return f.read(n)
    def hx(a, n):
        return rb(a, n).hex()
    def vp(v):
        if not v: return []
        fi, la = ptr(v + 4), ptr(v + 8)
        if la < fi or (la - fi) > 4096: return []
        return [ptr(p) for p in range(fi, la, 4)]
    def rand_state(base):
        d = rb(base, 0x9c4)
        mt = [struct.unpack_from("<I", d, 4*i)[0] for i in range(624)]
        mti = struct.unpack_from("<I", d, 0x9c0)[0]
        return mt, mti
    def rand_json(mt, mti):
        return {"mt": mt, "mti": mti}
    def map_nodes(head, nil_off):
        res = []
        if not head: return res
        def walk(n, depth):
            if not n or n == head or depth > 64: return
            try:
                if rb(n + nil_off, 1)[0] != 0: return
            except: return
            walk(ptr(n), depth + 1)
            res.append(n)
            walk(ptr(n + 8), depth + 1)
        try: walk(ptr(head + 4), 0)
        except: pass
        return res
    def readString(addr, length):
        try: return rb(addr, length).decode('ascii', errors='replace')
        except: return ""
    def pres(a):
        parts = []
        music = ptr(a + 0x374)
        tracks = []
        if music:
            try:
                for n in map_nodes(ptr(music + 8), 0x41):
                    tracks.append('{"id":%d,"vol":%.17g,"fade":%.17g,"max":%.17g,"stop":%d,"h":%d}' % (
                        rd(n + 0x10), rdd(n + 0x20), rdd(n + 0x28), rdd(n + 0x30),
                        rb(n + 0x38, 1)[0], ptr(n + 0x18)))
            except: pass
        parts.append('"music":{"track":%d,"order":%d,"tracks":[%s]}' % (
            rd(a + 0x88c), rd(a + 0x890), ','.join(tracks)))
        dlgs = []
        try:
            for n in map_nodes(ptr(a + 0x328), 0x15):
                dlgs.append(str(rd(n + 0xc)))
        except: pass
        parts.append('"dialogs":[%s]' % ','.join(dlgs))
        scr = []
        for off, name in SCREENS:
            try:
                p = ptr(a + off)
                if p: scr.append('"' + name + '"')
            except: pass
        parts.append('"screens":[%s]' % ','.join(scr))
        try:
            h = ptr(a + 0x814)
            if h: parts.append('"hatch":{"t":%d,"pet":%d,"jx":%d,"jy":%d}' % (rd(h+0x94),rd(h+0x98),rd(h+0x9c),rd(h+0xa0)))
        except: pass
        try:
            bs = ptr(a + 0x818)
            if bs: parts.append('"bonus":{"t":%d,"price":%d,"anim":%d,"purchased":%d}' % (rd(bs+0x28),rd(bs+0xd0),rd(bs+0xd4),rb(bs+0xe0,1)[0]))
        except: pass
        try:
            ps = ptr(a + 0x80c)
            if ps:
                prof2 = ptr(a + 0x8b8)
                sel = []
                for i in range(24):
                    bb = ptr(ps + 0x9c + 4*i)
                    sel.append("%02x" % (rb(bb + 0x130, 1)[0] if bb else 0))
                parts.append('"petscreen":{"raw":"%s","owned":"%s","chosen":"%s","buttons":"%s"}' % (
                    hx(ps, 0x200), hx(prof2, 24), hx(prof2 + 0x5a, 24), ''.join(sel)))
        except: pass
        return "{" + ",".join(parts) + "}"
    def write_gpos(a, bd):
        try:
            tmp = gpos_path + ".tmp"
            with open(tmp, "w") as gf:
                gf.write("S %d %d %d %d %d %d\n" % (
                    rb(bd+0x94,1)[0], rd(bd+0x3f0), rd(bd+0x43c),
                    rb(bd+0x3a4,1)[0], rb(bd+0x3a4+11,1)[0], len(vp(ptr(bd+0xac)))))
                for g in vp(ptr(bd+0xa0)):
                    gf.write("G %d %d %d %d\n" % (rd(g+0x30),rd(g+0x34),rd(g+0x9c),rd(g+0x1a0)))
                for fd in vp(ptr(bd+0xac)):
                    gf.write("F %d %d %d %d\n" % (rd(fd+0x30),rd(fd+0x34),rd(fd+0x194),rb(fd+0x185,1)[0]))
                for c in vp(ptr(bd+0xa8)):
                    gf.write("C %d %d %d %d\n" % (rd(c+0x30),rd(c+0x34),rb(c+0x198,1)[0],rd(c+0x194)))
                for al in vp(ptr(bd+0xb8)):
                    gf.write("A %d %d\n" % (rd(al+0x30),rd(al+0x34)))
                for m in vp(ptr(bd+0xdc)):
                    gf.write("M %d %d %d %d\n" % (rd(m+0x30),rd(m+0x34),rd(m+0x17c),rd(m+0x19c)))
                for off in (0xe8, 0xec):
                    for c in vp(ptr(bd+off)):
                        gf.write("C %d %d %d\n" % (rd(c+0x30),rd(c+0x34),rb(c+0x198,1)[0]))
                for o in vp(ptr(bd+0xa4)):
                    gf.write("O %d %d %d\n" % (rd(o+0x30),rd(o+0x34),rd(o+0x9c)))
                for o in vp(ptr(bd+0xb4)):
                    gf.write("P %d %d %d %d %d %d %d\n" % (
                        rd(o+0x234),rd(o+0x30),rd(o+0x34),rd(o+0x220),rd(o+0x224),rb(o+0x230,1)[0],rd(o+0x238)))
                for o in vp(ptr(bd+0xb0)):
                    gf.write("Q %d %d %d %d %d\n" % (
                        rd(o+0x1c4),rd(o+0x30),rd(o+0x34),rb(o+0x1c8,1)[0],rd(o+0x1ac)))
                gf.write("R %s\n" % hx(bd+0x3a4, 12))
                try:
                    for n in map_nodes(ptr(a+0x328), 0x15):
                        dlg = ptr(n+0x10)
                        gf.write("D %d %d %d %d %d\n" % (rd(n+0xc),rd(dlg+0x30),rd(dlg+0x34),rd(dlg+0x38),rd(dlg+0x3c)))
                except: pass
            os.rename(tmp, gpos_path)
        except: pass

    # Header
    st = rd(app + 0x47c)
    amt2, ami2 = rand_state(ptr(app + 0x7b0))
    gmt2, gmi2 = rand_state(GLOBAL_RAND)
    out = open(out_path, "w")
    out.write(json.dumps({"format":"insaniquarium-trace","version":1,"source":"original",
        "build":"f54c2c6ee54b00ae6da7f4bde15feb90d0a867aba6b4151b0277c1f81da3ff66",
        "start_tick":st,"rng":{"app":rand_json(amt2,ami2),"global":rand_json(gmt2,gmi2)},
        "fields":["board.money","counts","guppies","rng_mti"]}) + "\n")
    grubber_bomb = "grubber_bomb" in args
    gb_done = 0
    last_t = st - 1
    captured = 0
    while captured < max_ticks:
        t = rd(app + 0x47c)
        if t != last_t:
            last_t = t
            bd = ptr(app + 0x730)
            counts = {}
            if bd:
                for off, name in LISTS:
                    try:
                        n = len(vp(ptr(bd + off)))
                        if n > 0: counts[name] = n
                    except: pass
            # grubber_bomb: while a Grubber is alive, keep it fed so it
            # survives until a natural bomb hits it. Don't push Shrapnel's
            # counter (the replay can't reproduce that).
            if grubber_bomb and bd and counts.get("grubber", 0) > 0:
                try:
                    for grp in vp(ptr(bd + 0xc4)):
                        wr(grp + 0x9c, 1400)  # keep Grubber alive
                    if not gb_done:
                        gb_done = captured
                        print(f"grubber_bomb: grubber alive at tick {t}", file=sys.stderr, flush=True)
                except: pass
            fish = []
            if bd:
                for p in vp(ptr(bd + 0xa0)):
                    try:
                        fish.append({"x":rdd(p+0x158),"y":rdd(p+0x160),"vx":rdd(p+0x168),"vy":rdd(p+0x170),
                            "div":rdd(p+0x180),"hunger":rd(p+0x9c),"mode":rd(p+0x1ac),"mode_t":rd(p+0x1b0),
                            "reroll_t":rd(p+0x1b4),"stage":rd(p+0x1a0),"coin_t":rd(p+0x220),"coin_iv":rd(p+0x224),
                            "ptr":p,"raw":hx(p,0x230)})
                    except: pass
            raws = {}
            if bd:
                for off, name, sz in RAW_LISTS:
                    try:
                        items = [hx(p, sz) for p in vp(ptr(bd + off))]
                        if items: raws[name] = items
                    except: pass
            bj = None
            if bd:
                try:
                    mw = ptr(bd + 0x118)
                    msg = {}
                    if mw:
                        s = mw + 0x8c; sl = rd(s+0x14); cp = rd(s+0x18)
                        pa = ptr(s+4) if cp >= 16 else (s+4)
                        tx = readString(pa, sl) if sl > 0 else ""
                        msg = {"t":rd(mw+0xd0),"flash":rb(mw+0xd4,1)[0],"hint":rd(mw+0xd8),"text":tx.replace('"','').replace('\\','')}
                    pr2 = ptr(app + 0x8b8)
                    bj = {"money":rd(bd+0x3f0),"clock":rd(bd+0x28),"tank":rd(bd+0x3c8),"level":rd(bd+0x3cc),
                        "paused":rb(bd+0x94,1)[0]!=0,"eggs":rd(bd+0x43c),"revealed":hx(bd+0x3a4,12),
                        "flash":rd(bd+0x450),"ticks":rd(bd+0x444),
                        "wave":{"kind":rd(bd+0x2bc),"countdown":rd(bd+0x2c0),"t2c4":rd(bd+0x2c4),
                            "lockout":rd(bd+0x2c8),"ax":rd(bd+0x2cc),"ay":rd(bd+0x2d0),
                            "bx":rd(bd+0x2d4),"by":rd(bd+0x2d8),"waves":rd(bd+0x45c)},
                        "laser":rd(bd+0x3e4),"food_level":rd(0x5e89dc),"max_food":rd(0x5df58c),
                        "shot_x":rd(bd+0x3d0),"shot_y":rd(bd+0x3d4),"msg":msg,
                        "lights":[rdf(bd+0x3d8),rdf(bd+0x3dc),rdf(bd+0x3e0)],
                        "hints":hx(bd+0x4b0,0x36),"owned":hx(pr2,24) if pr2 else "",
                        "samples":hx(bd+0x1a4,0x100)}
                except: pass
            try:
                _, am3 = rand_state(ptr(app + 0x7b0))
                _, gm3 = rand_state(GLOBAL_RAND)
            except: am3, gm3 = -1, -1
            try: pr_json = pres(app)
            except: pr_json = "{}"
            td = {"tick":t,"u":rd(app+0x484),"board":bj,"pres":"__P__","counts":counts,
                "guppies":fish,"raw":raws,"rng_mti":{"app":am3,"global":gm3}}
            line = json.dumps(td, default=str).replace('"__P__"', pr_json)
            out.write(line + "\n")
            out.flush()
            if bd: write_gpos(app, bd)
            captured += 1
            if captured % 1000 == 0:
                print(f"captured {captured}/{max_ticks}", file=sys.stderr, flush=True)
        time.sleep(0.005)
    out.close()
    print(f"done: {captured} ticks to {out_path}", file=sys.stderr)
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
