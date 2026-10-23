#!/usr/bin/env python3
"""Confidence coverage of the behaviour specs (docs/re/**.md).

Every section (a `#`/`##`/`###` heading with a body) must carry at least
one confidence tag: `traced`, `confirmed`, `static` or `guess` (in
backticks). Prints a per-file summary of the strongest tag per section and
the untagged sections.

Usage: tools/spec_confidence.py [--check]   (--check: exit 1 if any
section is untagged)
"""
import glob
import re
import sys

TAGS = ["traced", "confirmed", "static", "guess"]
TAG = re.compile(r"`(%s)`" % "|".join(TAGS))
# Docs about our tooling and RE setup, not the game's behaviour.
TOOLING = ["dynamic-analysis.md", "frame-capture.md", "presentation-capture.md",
           "drm-wrapper.md", "demo-recording.md"]
# Sections that describe tooling or open questions rather than behaviour.
EXEMPT = re.compile(r"open questions|todo|summary|deviations|how to|usage|implications|implementation notes|^#", re.I)


def sections(path):
    cur = None
    out = []
    in_code = False
    for line in open(path, encoding="utf-8").read().split("\n"):
        if line.startswith("```"):
            in_code = not in_code
        m = None if in_code else re.match(r"^(#{1,4}) (.*)", line)
        if m:
            cur = [m.group(2).strip(), []]
            out.append(cur)
        elif cur is not None:
            cur[1].append(line)
    return [(t, "\n".join(b).strip()) for t, b in out if "\n".join(b).strip()]


def main():
    check = "--check" in sys.argv
    missing = []
    for path in sorted(glob.glob("docs/re/**/*.md", recursive=True)):
        if path.endswith("INDEX.md") or any(t in path for t in TOOLING):
            continue
        counts = dict.fromkeys(TAGS + ["untagged"], 0)
        for title, body in sections(path):
            found = TAG.findall(body)
            if found:
                counts[min(found, key=TAGS.index)] += 1
            elif not EXEMPT.search(title):
                counts["untagged"] += 1
                missing.append(f"{path}: {title}")
        summary = " ".join(f"{k}={v}" for k, v in counts.items() if v)
        print(f"{path}: {summary}")
    if missing:
        print("\nUntagged sections:")
        for m in missing:
            print("  " + m)
    if check and missing:
        sys.exit(1)


if __name__ == "__main__":
    main()
