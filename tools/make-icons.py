#!/usr/bin/env python3
"""Draws the HappyGreaseFloppy icon (a disk seen from above, its cylinders as rings) and renders
every file the three systems need.

    python3 tools/make-icons.py            # needs rsvg-convert (librsvg) and Pillow

Outputs: packaging/icon.svg, icon-medium.svg (32-64 px), icon-small.svg (16-24 px), icons/<size>.png, icon.png, icon.ico, icon.icns,
crates/hgf-gui/assets/icon128.rgba.
"""
import io
import math
import os
import subprocess

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PKG = os.path.join(ROOT, "packaging")
C = 512  # centre of the 1024×1024 canvas

# Colours of the app's disk visualizer (theme.rs).
OK = "#43c478"
RECOVERED = "#2ec4b6"
RETRY = "#f2ae37"
MEDIA = "#14161c"


def arc(r, a0, a1):
    """SVG path of a circle arc of radius r between angles a0 and a1 (degrees, 0 = top)."""
    def pt(a):
        t = math.radians(a - 90)
        return C + r * math.cos(t), C + r * math.sin(t)
    (x0, y0), (x1, y1) = pt(a0), pt(a1)
    large = 1 if (a1 - a0) % 360 > 180 else 0
    return f"M{x0:.1f} {y0:.1f} A{r} {r} 0 {large} 1 {x1:.1f} {y1:.1f}"


def ring_colour(i, n):
    """Outer tracks green, inner tracks teal, like a healthy read with a few recovered tracks."""
    t = i / max(1, n - 1)
    a, b = (0x43, 0xC4, 0x78), (0x2E, 0xB4, 0xC6)
    return "#%02x%02x%02x" % tuple(round(a[k] + (b[k] - a[k]) * t) for k in range(3))


def disk(rings, sectors, gap_deg, detail):
    outer, inner = 452, 196
    pitch = (outer - inner) / rings
    width = pitch * (0.78 if detail else 0.84)
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">',
        "<title>HappyGreaseFloppy</title>",
        f'<circle cx="{C}" cy="{C}" r="488" fill="{MEDIA}"/>',
        f'<circle cx="{C}" cy="{C}" r="488" fill="none" stroke="#3a4150" stroke-width="10"/>',
    ]
    retry_ring = rings // 2 if detail else None
    active_ring = 1 if detail else 0
    seg = 360 / sectors
    for i in range(rings):
        r = outer - pitch * (i + 0.5)
        colour = ring_colour(i, rings)
        if sectors == 1:
            parts.append(f'<circle cx="{C}" cy="{C}" r="{r:.1f}" fill="none" stroke="{colour}" stroke-width="{width:.1f}"/>')
            continue
        for s in range(sectors):
            a0 = s * seg + gap_deg / 2
            a1 = (s + 1) * seg - gap_deg / 2
            col = RETRY if (i == retry_ring and s in (7, 8)) else colour
            parts.append(f'<path d="{arc(r, a0, a1)}" fill="none" stroke="{col}" stroke-width="{width:.1f}"/>')
    # The track under the head, highlighted like the visualizer's active track.
    r = outer - pitch * (active_ring + 0.5)
    parts.append(
        f'<path d="{arc(r, 300, 420)}" fill="none" stroke="#ffffff" stroke-opacity="0.92" '
        f'stroke-width="{width * 1.15:.1f}" stroke-linecap="round"/>'
    )
    # Hub, spindle hole and index hole.
    parts += [
        f'<circle cx="{C}" cy="{C}" r="168" fill="#2a2f3a"/>',
        f'<circle cx="{C}" cy="{C}" r="168" fill="none" stroke="#6b7385" stroke-width="12"/>',
        f'<circle cx="{C}" cy="{C}" r="62" fill="{MEDIA}"/>',
        f'<circle cx="{C}" cy="{C - 118}" r="{22 if detail else 30}" fill="#4ea8de"/>',
        "</svg>",
    ]
    return "\n".join(parts) + "\n"


def render(svg, size):
    png = subprocess.run(
        ["rsvg-convert", "-w", str(size), "-h", str(size)], input=svg.encode(), capture_output=True, check=True
    ).stdout
    return Image.open(io.BytesIO(png)).convert("RGBA")


def main():
    full = disk(rings=9, sectors=11, gap_deg=3.2, detail=True)
    # Fewer, thicker rings at small sizes, so they stay readable instead of turning into moiré.
    medium = disk(rings=5, sectors=11, gap_deg=7, detail=True)
    small = disk(rings=3, sectors=1, gap_deg=0, detail=False)
    with open(os.path.join(PKG, "icon.svg"), "w") as f:
        f.write(full)
    with open(os.path.join(PKG, "icon-small.svg"), "w") as f:
        f.write(small)
    with open(os.path.join(PKG, "icon-medium.svg"), "w") as f:
        f.write(medium)

    sizes = [16, 24, 32, 48, 64, 128, 256, 512, 1024]
    imgs = {s: render(small if s <= 24 else medium if s <= 64 else full, s) for s in sizes}
    os.makedirs(os.path.join(PKG, "icons"), exist_ok=True)
    for s in sizes[:-1]:
        imgs[s].save(os.path.join(PKG, "icons", f"{s}.png"))
    imgs[512].save(os.path.join(PKG, "icon.png"))
    ico = [16, 24, 32, 48, 64, 128, 256]
    imgs[256].save(os.path.join(PKG, "icon.ico"), sizes=[(s, s) for s in ico],
                   append_images=[imgs[s] for s in ico if s != 256])
    imgs[1024].save(os.path.join(PKG, "icon.icns"), append_images=[imgs[s] for s in (16, 32, 64, 128, 256, 512)])
    with open(os.path.join(ROOT, "crates", "hgf-gui", "assets", "icon128.rgba"), "wb") as f:
        # Window and title-bar icon: shown at 24-48 px, so drawn from the medium design.
        f.write(render(medium, 128).tobytes())
    print("icons written")


if __name__ == "__main__":
    main()
