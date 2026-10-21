#!/usr/bin/env python3
"""Write THIRD-PARTY-LICENSES.txt: the licence of every crate linked into happygreasefloppy.

Usage: python3 tools/third-party.py  (from the repository root, after `cargo fetch`)

The list covers all target platforms (Windows, Linux, macOS), so it is a superset of what one
binary contains. Re-run it after changing dependencies.
"""
import json
import re
import pathlib
import subprocess
import sys

OWN = {"hgf-core", "hgf-gui"}
LICENCE_FILES = ("LICENSE", "LICENCE", "COPYING", "UNLICENSE", "NOTICE")


def main() -> int:
    meta = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked"], text=True))
    pkgs = {p["id"]: p for p in meta["packages"]}
    # Normal (non-dev, non-build) dependencies reachable from the GUI crate.
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    root = next(i for i, p in pkgs.items() if p["name"] == "hgf-gui")
    seen, todo = set(), [root]
    while todo:
        i = todo.pop()
        if i in seen:
            continue
        seen.add(i)
        for d in nodes[i]["deps"]:
            if any(k["kind"] in (None, "normal") for k in d["dep_kinds"]):
                todo.append(d["pkg"])

    crates = sorted((pkgs[i] for i in seen if pkgs[i]["name"] not in OWN),
                    key=lambda p: (p["name"], p["version"]))
    out = [
        "HappyGreaseFloppy - third-party licences",
        "=" * 40,
        "",
        "HappyGreaseFloppy is built with the following Rust crates. Each is used under the licence",
        "shown (for dual-licensed crates, the user may choose either). The full texts shipped with",
        "each crate follow the summary. The JetBrains Mono font is under the SIL Open Font License",
        "1.1 (JetBrainsMono-OFL.txt).",
        "",
    ]
    missing = []
    for p in crates:
        out.append(f"  {p['name']} {p['version']}  -  {p.get('license') or 'see text'}")
    out += ["", ""]
    # Identical texts (most crates ship the same Apache-2.0 / MIT files) are printed once, with
    # the crates that use them; texts that differ (copyright lines) are kept apart.
    texts = {}
    for p in crates:
        d = pathlib.Path(p["manifest_path"]).parent
        files = sorted(f for f in d.iterdir()
                       if f.is_file() and f.name.upper().startswith(LICENCE_FILES))
        if not files:
            missing.append(p)
        for f in files:
            t = f.read_text(encoding="utf-8", errors="replace").strip()
            key = re.sub(r"\s+", " ", t)
            texts.setdefault(key, (t, []))[1].append(f"{p['name']} {p['version']}")
    for t, users in sorted(texts.values(), key=lambda v: (-len(v[1]), v[1][0])):
        out.append("-" * 78)
        out.append("Used by: " + ", ".join(users))
        out.append("-" * 78)
        out.append(t)
        out += ["", ""]
    if missing:
        out.append("-" * 78)
        out.append("Crates published without a licence file (the licence named in their manifest")
        out.append("applies; standard texts above; copyright holders = authors listed here):")
        out.append("-" * 78)
        for p in missing:
            authors = ", ".join(p.get("authors") or []) or "the authors"
            out.append(f"  {p['name']} {p['version']}  -  {p.get('license')}  -  (c) {authors}")
        out.append("")
    pathlib.Path("THIRD-PARTY-LICENSES.txt").write_text("\n".join(out) + "\n", encoding="utf-8")
    print(f"{len(crates)} crates, {len(texts)} distinct licence texts, {len(missing)} without a file")
    return 0


if __name__ == "__main__":
    sys.exit(main())
