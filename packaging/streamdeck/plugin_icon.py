"""Draws the plugin's own picture (Stream Deck wants a PNG for it): a Mocha
square, the blue to mauve ring and six sound bars. Run: python plugin_icon.py"""
import os

from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "com.davidhifi.yapfast.sdPlugin", "imgs")
BASE = (0x1E, 0x1E, 0x2E)
BLUE = (0x89, 0xB4, 0xFA)
MAUVE = (0xCB, 0xA6, 0xF7)


def mix(t):
    return tuple(round(a + (b - a) * t) for a, b in zip(BLUE, MAUVE))


def picture(size):
    big = size * 4
    image = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    pen = ImageDraw.Draw(image)
    radius = big * 0.22
    pen.rounded_rectangle([0, 0, big - 1, big - 1], radius=radius, fill=BASE)
    ring = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    ring_pen = ImageDraw.Draw(ring)
    width = max(4, big // 24)
    inset = width
    ring_pen.rounded_rectangle([inset, inset, big - 1 - inset, big - 1 - inset],
                               radius=radius - inset, outline=(255, 255, 255, 255), width=width)
    shade = Image.new("RGBA", (big, big))
    shade_pen = ImageDraw.Draw(shade)
    for y in range(big):
        shade_pen.line([(0, y), (big, y)], fill=mix(y / big))
    image.paste(shade, (0, 0), ring)
    heights = [0.22, 0.46, 0.7, 0.36, 0.56, 0.22]
    bar = big * 0.075
    gap = big * 0.045
    total = len(heights) * bar + (len(heights) - 1) * gap
    left = (big - total) / 2
    for index, height in enumerate(heights):
        x = left + index * (bar + gap)
        h = big * height
        top = (big - h) / 2
        pen.rounded_rectangle([x, top, x + bar, top + h], radius=bar / 2, fill=mix(index / (len(heights) - 1)))
    return image.resize((size, size), Image.LANCZOS)


os.makedirs(OUT, exist_ok=True)
picture(256).save(os.path.join(OUT, "plugin.png"))
picture(512).save(os.path.join(OUT, "plugin@2x.png"))
print("wrote plugin.png and plugin@2x.png")
