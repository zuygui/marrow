#!/usr/bin/env python3
"""Render the README bar chart (dark and light SVGs) from results.json.

Usage: python3 benchmarks/chart.py   (writes .github/assets/bench-{dark,light}.svg)
"""
import json, os, re

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", ".github", "assets")

THEMES = {
    "dark": dict(card="#140f26", edge="#2c2350", text="#f5f3ff", dim="#b9aee0", grid="#2c2350",
                 bar="#5b5078", marrow=("#7c3aed", "#8b5cf6", "#c4b5fd"), accent="#c4b5fd", good="#86efac"),
    "light": dict(card="#faf7ff", edge="#e4d9fb", text="#2e1065", dim="#6b5b95", grid="#e4d9fb",
                  bar="#c5bdd9", marrow=("#6d28d9", "#7c3aed", "#a78bfa"), accent="#6d28d9", good="#15803d"),
}
SANS = "'Hanken Grotesk','Inter',system-ui,-apple-system,'Segoe UI',sans-serif"
MONO = "'JetBrains Mono',ui-monospace,SFMono-Regular,Menlo,monospace"


def version(meta, tid):
    key = {"marrow": "marrow", "gcc-O0": "gcc", "gcc-O2": "gcc", "clang-O2": "clang", "rustc-O": "rustc", "go": "go"}[tid]
    m = re.search(r"\d+\.\d+(\.\d+)?", meta["versions"][key])
    return ("v" if tid == "marrow" else "") + (m.group(0) if m else "")


def render(theme, data):
    c = THEMES[theme]
    meta = data["meta"]
    progs = data["programs"]
    acc = {}
    labels = {}
    for p in progs:
        for t in p["toolchains"]:
            acc.setdefault(t["id"], []).append(t["median_ms"])
            labels[t["id"]] = t["label"]
    rows = sorted(((sum(v) / len(v), k) for k, v in acc.items()))
    top = (int(max(r[0] for r in rows) // 50) + 1) * 50
    W, H = 880, 120 + 46 * len(rows) + 92
    x0, x1 = 190, 680  # bar track
    scale = (x1 - x0) / top
    o = []
    a = o.append
    a(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" role="img" '
      f'aria-label="Compile time of Marrow, gcc, clang, rustc and go, mean of {len(progs)} programs">')
    a('<defs><linearGradient id="m" x1="0" y1="0" x2="1" y2="0">'
      f'<stop offset="0" stop-color="{c["marrow"][0]}"/><stop offset="0.7" stop-color="{c["marrow"][1]}"/>'
      f'<stop offset="1" stop-color="{c["marrow"][2]}"/></linearGradient></defs>')
    a(f'<rect x="0.5" y="0.5" width="{W-1}" height="{H-1}" rx="20" fill="{c["card"]}" stroke="{c["edge"]}"/>')
    a(f'<text x="40" y="56" font-family="{SANS}" font-size="22" font-weight="700" fill="{c["text"]}">Compile time</text>')
    a(f'<text x="40" y="80" font-family="{SANS}" font-size="13" fill="{c["dim"]}">mean over {len(progs)} programs · '
      f'milliseconds, median of {meta["runs"]} runs (lower is better)</text>')
    y = 112
    best = rows[0][1]
    for mean, tid in rows:
        is_m = tid == "marrow"
        cy = y + 15
        a(f'<text x="{x0-16}" y="{cy-2}" text-anchor="end" font-family="{SANS}" font-size="15" font-weight="600" '
          f'fill="{c["text"]}">{labels[tid]}</text>')
        a(f'<text x="{x0-16}" y="{cy+13}" text-anchor="end" font-family="{MONO}" font-size="10.5" fill="{c["dim"]}">'
          f'{version(meta, tid)}</text>')
        w = mean * scale
        fill = "url(#m)" if is_m else c["bar"]
        a(f'<path d="M{x0} {y} H{x0+w-8:.1f} a8 8 0 0 1 8 8 v14 a8 8 0 0 1 -8 8 H{x0} Z" fill="{fill}"/>')
        if is_m:
            mx = x0 + w - 24
            a(f'<g transform="translate({mx:.1f} {y+4}) scale(0.11)"><path d="M 100,45 C 130,45 150,70 150,105 C 150,140 125,155 '
              f'100,155 C 75,155 50,140 50,105 C 50,70 70,45 100,45 Z" fill="#fff"/><circle cx="100" cy="95" r="16" fill="{c["marrow"][0]}"/>'
              f'<circle cx="100" cy="95" r="8" fill="#fff"/></g>')
        vx = x1 + 24
        size, weight, col = (22, 800, c["accent"]) if is_m else (16, 700, c["text"])
        a(f'<text x="{vx}" y="{cy+6}" font-family="{MONO}" font-size="{size}" font-weight="{weight}" fill="{col}">{mean:.0f}'
          f'<tspan font-size="11" font-weight="400" fill="{c["dim"]}"> ms</tspan></text>')
        if tid == best:
            a(f'<text x="{vx+70}" y="{cy+5}" font-family="{SANS}" font-size="9.5" font-weight="700" letter-spacing="0.6" '
              f'fill="{c["good"]}">FASTEST</text>')
        y += 46
    ay = y + 4
    a(f'<line x1="{x0}" y1="{ay}" x2="{x1}" y2="{ay}" stroke="{c["grid"]}"/>')
    for t in range(0, top + 1, 50):
        tx = x0 + t * scale
        a(f'<line x1="{tx}" y1="{ay}" x2="{tx}" y2="{ay+5}" stroke="{c["grid"]}"/>')
        a(f'<text x="{tx}" y="{ay+20}" text-anchor="middle" font-family="{MONO}" font-size="10.5" fill="{c["dim"]}">{t}</text>')
    cpu = meta["cpu"].replace("(R)", "").replace("Processor ", "")
    a(f'<line x1="0" y1="{H-52}" x2="{W}" y2="{H-52}" stroke="{c["edge"]}"/>')
    a(f'<text x="40" y="{H-26}" font-family="{SANS}" font-size="11.5" fill="{c["dim"]}">{cpu}, {meta["cores"]} cores · one compiler '
      f'invocation producing an executable · unique source each run · build time, not run time</text>')
    a('</svg>')
    return "\n".join(o) + "\n"


def main():
    data = json.load(open(os.path.join(HERE, "results.json")))
    os.makedirs(OUT, exist_ok=True)
    for th in THEMES:
        path = os.path.join(OUT, f"bench-{th}.svg")
        open(path, "w").write(render(th, data))
        print("wrote", os.path.relpath(path))


if __name__ == "__main__":
    main()
