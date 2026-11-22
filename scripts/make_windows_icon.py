#!/usr/bin/env python3
"""Draws packaging/windows/yapfast.ico: HaloCord Yapfast's icon, the logo in
Mocha blue to mauve with Mocha-dark dots, at each size Windows asks for.

The shape is the one in packaging/icon.svg (same numbers). Each size is drawn
16 times larger and shrunk with a good filter, so edges are crisp from 16 px
in the title bar to 256 px in Explorer. Needs Pillow only.
"""
import sys
from pathlib import Path

from PIL import Image, ImageDraw

STAR = [(50, 14), (58, 27), (73, 19), (72, 35), (88, 38), (77, 50), (87, 62),
        (71, 64), (69, 78), (56, 71), (38, 88), (40, 71), (26, 74), (30, 60),
        (14, 52), (27, 43), (18, 29), (35, 31), (37, 17)]
DOTS = [(38, 51), (51, 51), (64, 51)]
BLUE, MAUVE, DARK = (0x89, 0xB4, 0xFA), (0xCB, 0xA6, 0xF7), (0x1E, 0x1E, 0x2E)
VIEW = (12, 12, 78)  # x, y, size of the SVG's viewBox
SIZES = [16, 20, 24, 32, 40, 48, 64, 256]
SCALE = 16


def star_mask(px):
    """The star with its 3-wide rounded outline, as a mask `px` wide."""
    k = px / VIEW[2]
    mask = Image.new("L", (px, px), 0)
    draw = ImageDraw.Draw(mask)
    points = [((x - VIEW[0]) * k, (y - VIEW[1]) * k) for x, y in STAR]
    draw.polygon(points, fill=255)
    half = 1.5 * k
    for (ax, ay), (bx, by) in zip(points, points[1:] + points[:1]):
        draw.line([(ax, ay), (bx, by)], fill=255, width=round(2 * half))
        draw.ellipse([ax - half, ay - half, ax + half, ay + half], fill=255)
    return mask


def gradient(px):
    """Blue at the top left to mauve at the bottom right, as the SVG's."""
    k = px / VIEW[2]
    x1, y1, x2, y2 = [(v - VIEW[i % 2]) * k for i, v in enumerate((16, 18, 86, 84))]
    dx, dy = x2 - x1, y2 - y1
    span = dx * dx + dy * dy
    image = Image.new("RGB", (px, px))
    pixels = image.load()
    for y in range(px):
        for x in range(px):
            t = min(1.0, max(0.0, ((x - x1) * dx + (y - y1) * dy) / span))
            pixels[x, y] = tuple(round(a + (b - a) * t) for a, b in zip(BLUE, MAUVE))
    return image


def draw(size):
    px = size * SCALE
    icon = Image.new("RGBA", (px, px), (0, 0, 0, 0))
    icon.paste(gradient(px), (0, 0), star_mask(px))
    k = px / VIEW[2]
    draw = ImageDraw.Draw(icon)
    for x, y in DOTS:
        cx, cy, r = (x - VIEW[0]) * k, (y - VIEW[1]) * k, 5 * k
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=DARK + (255,))
    return icon.resize((size, size), Image.LANCZOS)


def main():
    root = Path(__file__).resolve().parent.parent
    out = root / "packaging" / "windows" / "yapfast.ico"
    pictures = [draw(size) for size in SIZES]
    pictures[-1].save(out, format="ICO", sizes=[(s, s) for s in SIZES], append_images=pictures[:-1])
    if len(sys.argv) > 1:
        for size, picture in zip(SIZES, pictures):
            picture.save(Path(sys.argv[1]) / f"icon-{size}.png")


main()
