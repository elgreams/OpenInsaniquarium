"""Tiny client for the GhidraMCP HTTP plugin (default http://127.0.0.1:8080).

Used by local RE helper scripts. Output derived from the user's binary
belongs under re/ (gitignored).
"""

import os
import urllib.parse
import urllib.request

BASE = os.environ.get("GHIDRA_MCP_URL", "http://127.0.0.1:8080")


def get(path, **params):
    q = urllib.parse.urlencode(params)
    url = f"{BASE}/{path}" + (f"?{q}" if q else "")
    with urllib.request.urlopen(url, timeout=120) as r:
        return r.read().decode("utf-8", "replace")


def disasm(addr):
    """Return [(addr:int, text:str)] for the function containing addr."""
    out = []
    for line in get("disassemble_function", address=hex(addr)).splitlines():
        if ": " not in line:
            continue
        a, t = line.split(": ", 1)
        try:
            out.append((int(a, 16), t.strip()))
        except ValueError:
            pass
    return out


def xrefs_to(addr, limit=2000):
    res = []
    for line in get("xrefs_to", address=hex(addr), limit=limit).splitlines():
        parts = line.split()
        if len(parts) >= 2 and parts[0] == "From":
            res.append((int(parts[1], 16), " ".join(parts[3:])))
    return res


def function_at(addr):
    """Entry address of the function containing addr, scanning backwards."""
    for back in range(0, 0x4000):
        r = get("get_function_by_address", address=hex(addr - back))
        if r.startswith("Function:"):
            for line in r.splitlines():
                if line.startswith("Entry:"):
                    return int(line.split()[1], 16)
    return None
