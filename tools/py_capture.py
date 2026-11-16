#!/usr/bin/env python3
"""RE tooling only: per-tick state capture via /proc/PID/mem, bypassing CE.
Drop-in replacement for ce_capture.lua when CE is unavailable.

Usage: sudo python3 tools/py_capture.py <pid> [ticks] [out] [gpos]
  pid:   game process PID
  ticks: number of ticks to capture (default 6000)
  out:   output file (default /tmp/capture.jsonl)
  gpos:  gpos file for autoplay (default /tmp/gpos)
"""
import json, struct, sys, time, os, tempfile

pid = int(sys.argv[1])
max_ticks = int(sys.argv[2]) if len(sys.argv) > 2 else 6000
out_path = sys.argv[3] if len(sys.argv) > 3 else "/tmp/capture.jsonl"
gpos_path = sys.argv[4] if len(sys.argv) > 4 else "/tmp/gpos"

f = open(f"/proc/{pid}/mem", "rb", buffering=0)

APP_PTR = 0x5EB680
GLOBAL_RAND = 0x5EB928

def rd(a):
    f.seek(a); return struct.unpack("<i", f.read(4))[0]
def u32(a):
    f.seek(a); return struct.unpack("<I", f.read(4))[0]
def f64(a):
    f.seek(a); return struct.unpack("<d", f.read(8))[0]
def f32(a):
    f.seek(a); return struct.unpack("<f", f.read(4))[0]
def rb(a, n):
    f.seek(a); return f.read(n)
def hex_block(a, n):
    return rb(a, n).hex()
def vec_ptrs(vec):
    if not vec: return []
    first, last = u32(vec + 4), u32(vec + 8)
    if last < first or (last - first) > 4096: return []
    return [u32(p) for p in range(first, last, 4)]

def rand_state(base):
    b = rb(base, 0x9c4)
    mt = [struct.unpack_from("<I", b, 4*i)[0] for i in range(624)]
    mti = struct.unpack_from("<I", b, 0x9c0)[0]
    return mt, mti

def rand_json(mt, mti):
    return '{"mt":[' + ','.join(str(v) for v in mt) + '],"mti":' + str(mti) + '}'

# MSVC std::map in-order walk
def map_nodes(head, nil_off):
    res = []
    if not head: return res
    def walk(n, depth):
        if not n or n == head or depth > 64: return
        try:
            if rb(n + nil_off, 1)[0] != 0: return
        except: return
        walk(u32(n), depth + 1)
        res.append(n)
        walk(u32(n + 8), depth + 1)
    try: walk(u32(head + 4), 0)
    except: pass
    return res

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

def num(v):
    if v is None: return "null"
    if isinstance(v, int): return str(v)
    return f"{v:.17g}"

def readString(addr, length):
    try:
        b = rb(addr, length)
        return b.decode('ascii', errors='replace')
    except: return ""

def presentation(app):
    parts = []
    music = u32(app + 0x374)
    tracks = []
    if music:
        try:
            for n in map_nodes(u32(music + 8), 0x41):
                tracks.append('{"id":%d,"vol":%s,"fade":%s,"max":%s,"stop":%d,"h":%d}' % (
                    rd(n + 0x10), num(f64(n + 0x20)), num(f64(n + 0x28)), num(f64(n + 0x30)),
                    rb(n + 0x38, 1)[0], u32(n + 0x18)))
        except: pass
    parts.append('"music":{"track":%d,"order":%d,"tracks":[%s]}' % (
        rd(app + 0x88c), rd(app + 0x890), ','.join(tracks)))
    dialogs = []
    try:
        for n in map_nodes(u32(app + 0x328), 0x15):
            dialogs.append(str(rd(n + 0xc)))
    except: pass
    parts.append('"dialogs":[%s]' % ','.join(dialogs))
    screens = []
    for off, name in SCREENS:
        try:
            p = u32(app + off)
            if p: screens.append('"' + name + '"')
        except: pass
    parts.append('"screens":[%s]' % ','.join(screens))
    # Hatch screen
    try:
        h = u32(app + 0x814)
        if h:
            parts.append('"hatch":{"t":%d,"pet":%d,"jx":%d,"jy":%d}' % (
                rd(h + 0x94), rd(h + 0x98), rd(h + 0x9c), rd(h + 0xa0)))
    except: pass
    # Bonus screen
    try:
        bs = u32(app + 0x818)
        if bs:
            parts.append('"bonus":{"t":%d,"price":%d,"anim":%d,"purchased":%d}' % (
                rd(bs + 0x28), rd(bs + 0xd0), rd(bs + 0xd4), rb(bs + 0xe0, 1)[0]))
    except: pass
    # Pet screen
    try:
        ps = u32(app + 0x80c)
        if ps:
            prof = u32(app + 0x8b8)
            sel = []
            for pid_i in range(24):
                b = u32(ps + 0x9c + 4 * pid_i)
                sel.append("%02x" % (rb(b + 0x130, 1)[0] if b else 0))
            parts.append('"petscreen":{"raw":"%s","owned":"%s","chosen":"%s","buttons":"%s"}' % (
                hex_block(ps, 0x200), hex_block(prof, 24), hex_block(prof + 0x5a, 24), ''.join(sel)))
    except: pass
    return "{" + ",".join(parts) + "}"

def write_gpos(app, board):
    """Write state for autoplay (same format as ce_capture.lua)."""
    try:
        tmp = gpos_path + ".tmp"
        with open(tmp, "w") as gf:
            gf.write("S %d %d %d %d %d %d\n" % (
                rb(board + 0x94, 1)[0], rd(board + 0x3f0), rd(board + 0x43c),
                rb(board + 0x3a4, 1)[0], rb(board + 0x3a4 + 11, 1)[0],
                len(vec_ptrs(u32(board + 0xac)))))
            for g in vec_ptrs(u32(board + 0xa0)):
                gf.write("G %d %d %d %d\n" % (rd(g + 0x30), rd(g + 0x34), rd(g + 0x9c), rd(g + 0x1a0)))
            for fd in vec_ptrs(u32(board + 0xac)):
                gf.write("F %d %d %d %d\n" % (rd(fd + 0x30), rd(fd + 0x34), rd(fd + 0x194), rb(fd + 0x185, 1)[0]))
            for c in vec_ptrs(u32(board + 0xa8)):
                gf.write("C %d %d %d %d\n" % (rd(c + 0x30), rd(c + 0x34), rb(c + 0x198, 1)[0], rd(c + 0x194)))
            for a in vec_ptrs(u32(board + 0xb8)):
                gf.write("A %d %d\n" % (rd(a + 0x30), rd(a + 0x34)))
            for m in vec_ptrs(u32(board + 0xdc)):
                gf.write("M %d %d %d %d\n" % (rd(m + 0x30), rd(m + 0x34), rd(m + 0x17c), rd(m + 0x19c)))
            for off in (0xe8, 0xec):
                for c in vec_ptrs(u32(board + off)):
                    gf.write("C %d %d %d\n" % (rd(c + 0x30), rd(c + 0x34), rb(c + 0x198, 1)[0]))
            for o in vec_ptrs(u32(board + 0xa4)):
                gf.write("O %d %d %d\n" % (rd(o + 0x30), rd(o + 0x34), rd(o + 0x9c)))
            for o in vec_ptrs(u32(board + 0xb4)):
                gf.write("P %d %d %d %d %d %d %d\n" % (
                    rd(o + 0x234), rd(o + 0x30), rd(o + 0x34),
                    rd(o + 0x220), rd(o + 0x224), rb(o + 0x230, 1)[0], rd(o + 0x238)))
            for o in vec_ptrs(u32(board + 0xb0)):
                gf.write("Q %d %d %d %d %d\n" % (
                    rd(o + 0x1c4), rd(o + 0x30), rd(o + 0x34),
                    rb(o + 0x1c8, 1)[0], rd(o + 0x1ac)))
            gf.write("R %s\n" % hex_block(board + 0x3a4, 12))
            # Dialogs
            try:
                for n in map_nodes(u32(app + 0x328), 0x15):
                    dlg = u32(n + 0x10)
                    gf.write("D %d %d %d %d %d\n" % (rd(n + 0xc), rd(dlg + 0x30), rd(dlg + 0x34), rd(dlg + 0x38), rd(dlg + 0x3c)))
            except: pass
        os.rename(tmp, gpos_path)
    except Exception as e:
        pass  # non-fatal

# -- Main capture loop --
app = u32(APP_PTR)
assert app, "app not found"

out = open(out_path, "w")

# Header
start_tick = rd(app + 0x47c)
amt, ami = rand_state(u32(app + 0x7b0))
gmt, gmi = rand_state(GLOBAL_RAND)
out.write(json.dumps({
    "format": "insaniquarium-trace", "version": 1, "source": "original",
    "build": "f54c2c6ee54b00ae6da7f4bde15feb90d0a867aba6b4151b0277c1f81da3ff66",
    "start_tick": start_tick,
    "rng": {"app": json.loads(rand_json(amt, ami)), "global": json.loads(rand_json(gmt, gmi))},
    "fields": ["board.money", "counts", "guppies", "rng_mti"]
}) + "\n")

last = start_tick - 1
captured = 0
while captured < max_ticks:
    t = rd(app + 0x47c)
    if t != last:
        last = t
        board = u32(app + 0x730)
        # Counts
        counts = {}
        if board:
            for off, name in LISTS:
                try:
                    n = len(vec_ptrs(u32(board + off)))
                    if n > 0: counts[name] = n
                except: pass
        # Per-guppy
        fish = []
        if board:
            for p in vec_ptrs(u32(board + 0xa0)):
                try:
                    fish.append({
                        "x": f64(p+0x158), "y": f64(p+0x160), "vx": f64(p+0x168), "vy": f64(p+0x170),
                        "div": f64(p+0x180), "hunger": rd(p+0x9c), "mode": rd(p+0x1ac),
                        "mode_t": rd(p+0x1b0), "reroll_t": rd(p+0x1b4), "stage": rd(p+0x1a0),
                        "coin_t": rd(p+0x220), "coin_iv": rd(p+0x224), "ptr": p,
                        "raw": hex_block(p, 0x230)
                    })
                except: pass
        # Raw lists
        raws = {}
        if board:
            for off, name, sz in RAW_LISTS:
                try:
                    items = [hex_block(p, sz) for p in vec_ptrs(u32(board + off))]
                    if items: raws[name] = items
                except: pass
        # Board state
        board_json = None
        if board:
            try:
                bclock = rd(board + 0x28)
                money = rd(board + 0x3f0)
                tank = rd(board + 0x3c8)
                level = rd(board + 0x3cc)
                paused = rb(board + 0x94, 1)[0] != 0
                eggs = rd(board + 0x43c)
                revealed = hex_block(board + 0x3a4, 12)
                flash = rd(board + 0x450)
                ticks = rd(board + 0x444)
                wave = {
                    "kind": rd(board + 0x2bc), "countdown": rd(board + 0x2c0),
                    "t2c4": rd(board + 0x2c4), "lockout": rd(board + 0x2c8),
                    "ax": rd(board + 0x2cc), "ay": rd(board + 0x2d0),
                    "bx": rd(board + 0x2d4), "by": rd(board + 0x2d8),
                    "waves": rd(board + 0x45c)
                }
                laser = rd(board + 0x3e4)
                food_level = rd(0x5e89dc)
                max_food = rd(0x5df58c)
                shot_x = rd(board + 0x3d0)
                shot_y = rd(board + 0x3d4)
                # Message widget
                msg = {}
                mw = u32(board + 0x118)
                if mw:
                    s = mw + 0x8c
                    slen = rd(s + 0x14)
                    cap = rd(s + 0x18)
                    p_addr = u32(s + 4) if cap >= 16 else (s + 4)
                    text = readString(p_addr, slen) if slen > 0 else ""
                    msg = {"t": rd(mw + 0xd0), "flash": rb(mw + 0xd4, 1)[0], "hint": rd(mw + 0xd8), "text": text.replace('"', '').replace('\\', '')}
                lights = [f32(board + 0x3d8), f32(board + 0x3dc), f32(board + 0x3e0)]
                hints = hex_block(board + 0x4b0, 0x36)
                prof = u32(app + 0x8b8)
                owned = hex_block(prof, 24) if prof else ""
                samples = hex_block(board + 0x1a4, 0x100)
                board_json = {
                    "money": money, "clock": bclock, "tank": tank, "level": level,
                    "paused": paused, "eggs": eggs, "revealed": revealed, "flash": flash,
                    "ticks": ticks, "wave": wave, "laser": laser, "food_level": food_level,
                    "max_food": max_food, "shot_x": shot_x, "shot_y": shot_y,
                    "msg": msg, "lights": lights, "hints": hints, "owned": owned,
                    "samples": samples
                }
            except Exception as e:
                board_json = None
        # RNG mti
        try:
            _, ami2 = rand_state(u32(app + 0x7b0))
            _, gmi2 = rand_state(GLOBAL_RAND)
        except:
            ami2, gmi2 = -1, -1
        # Presentation
        try:
            pres = presentation(app)
        except:
            pres = "{}"
        # Emit: use manual JSON for the outer frame to match ce_capture.lua format
        # But use the simpler approach of json.dumps for board/counts
        tick_data = {
            "tick": t, "u": rd(app + 0x484),
            "board": board_json,
            "pres": "__PRES__",
            "counts": counts,
            "guppies": fish,
            "raw": raws,
            "rng_mti": {"app": ami2, "global": gmi2}
        }
        line = json.dumps(tick_data, default=str)
        # Replace the placeholder with the raw presentation JSON
        line = line.replace('"__PRES__"', pres)
        out.write(line + "\n")
        out.flush()
        # Write gpos for autoplay
        if board:
            write_gpos(app, board)
        captured += 1
        if captured % 1000 == 0:
            print(f"captured {captured}/{max_ticks} ticks", file=sys.stderr)
    time.sleep(0.005)  # 5ms poll (faster than 28ms tick)

out.close()
print(f"captured {captured} ticks to {out_path}", file=sys.stderr)
