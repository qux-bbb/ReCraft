# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
"""Generate the ReCraft application icon (PNG + multi-size ICO + raw RGBA).

The mark is the ReCraft "R" above a cyan scan line with byte ticks: the scan line is
what a reverse engineer sees under the cursor, the ticks are the bytes it reads. Flat
geometry instead of a framed emblem, because the icon has to survive 16 px in a task
bar: no fine lines, no text, one high-contrast letter and one accent.

Everything is drawn from primitives so the icon can be regenerated without a design
tool or a font file. Requires Pillow.

    python assets/generate_icon.py
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets"
SIZE = 2048
FINAL = 1024

# Palette: a dark slate plate, an ink-white letter, one cyan accent.
PLATE_TOP = (30, 41, 54)
PLATE_BOTTOM = (15, 20, 27)
INK_TOP = (247, 252, 255)
INK_BOTTOM = (196, 219, 228)
CYAN = (64, 205, 224, 255)
CYAN_DIM = (43, 128, 143, 255)
RIM = (70, 92, 110, 150)

# The scan band sits at 70 % height, under the letter's bowl.
BAND_Y = int(SIZE * 0.70)


def vertical_gradient(size: int, top: tuple[int, int, int], bottom: tuple[int, int, int]) -> Image.Image:
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    for y in range(size):
        blend = y / (size - 1)
        color = tuple(int(top[i] + (bottom[i] - top[i]) * blend) for i in range(3))
        draw.line([(0, y), (size, y)], fill=color + (255,))
    return image


def rounded_mask(size: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size - 1, size - 1), radius, fill=255)
    return mask


def scaled_mask(mask: Image.Image, factor: float, center: tuple[float, float]) -> Image.Image:
    """Scale a mask by `factor` around `center`, keeping the canvas size."""
    scaled = mask.resize((int(SIZE * factor), int(SIZE * factor)), Image.Resampling.LANCZOS)
    out = Image.new("L", (SIZE, SIZE), 0)
    left = int(center[0] * factor - SIZE / 2)
    top = int(center[1] * factor - SIZE / 2)
    out.paste(scaled, (-left, -top))
    return out


def draw_r(draw: ImageDraw.ImageDraw, fill: int = 255) -> None:
    """Geometric R with heavy rounded strokes (the same skeleton as the old emblem)."""
    stroke = 220
    # Stem.
    draw.rounded_rectangle((600, 500, 820, 1500), radius=110, fill=fill)
    # Bowl: right half of a thick ring, with the junctions filled so it reads solid.
    draw.arc((470, 430, 1130, 1090), start=-92, end=92, fill=fill, width=stroke)
    draw.rounded_rectangle((700, 430, 900, 650), radius=70, fill=fill)
    draw.rounded_rectangle((700, 870, 900, 1090), radius=70, fill=fill)
    # Diagonal leg.
    draw.line((820, 1050, 1270, 1490), fill=fill, width=stroke)
    draw.ellipse((820 - stroke // 2, 1050 - stroke // 2, 820 + stroke // 2, 1050 + stroke // 2), fill=fill)
    draw.ellipse((1270 - stroke // 2, 1490 - stroke // 2, 1270 + stroke // 2, 1490 + stroke // 2), fill=fill)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)

    # Plate: vertical gradient inside a 22 % rounded square, plus a hairline rim.
    plate = vertical_gradient(SIZE, PLATE_TOP, PLATE_BOTTOM)
    rim = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(rim).rounded_rectangle(
        (6, 6, SIZE - 6, SIZE - 6),
        radius=int(SIZE * 0.215),
        outline=RIM,
        width=4,
    )
    plate = Image.alpha_composite(plate, rim)

    # Soft mint glow behind the letter: depth without a gradient mesh.
    halo = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(halo).ellipse((470, 360, 1400, 1380), fill=(53, 205, 214, 46))
    halo = halo.filter(ImageFilter.GaussianBlur(120))
    plate = Image.alpha_composite(plate, halo)

    # Letter: a flat ink gradient, clipped to the plate. Drawn at the skeleton's size,
    # then scaled up a little around its own centre so it fills the plate at 16 px.
    r_mask = Image.new("L", (SIZE, SIZE), 0)
    draw_r(ImageDraw.Draw(r_mask))
    r_mask = scaled_mask(r_mask, 1.08, (870, 940))

    # Drop shadow so the letter separates from the glow behind it.
    shadow = Image.new("RGBA", (SIZE, SIZE), (6, 10, 14, 255))
    shadow.putalpha(
        r_mask.filter(ImageFilter.GaussianBlur(26)).point(lambda value: int(value * 0.45))
    )
    plate = Image.alpha_composite(plate, shadow)

    ink = vertical_gradient(SIZE, INK_TOP, INK_BOTTOM)
    letter = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    letter.paste(ink, (0, 0), r_mask)
    plate = Image.alpha_composite(plate, letter)

    # Scan band: a dark strip cuts the letter, a cyan rule runs across it, and seven
    # ticks mark the bytes it reads (kept chunky so they survive at 16 px).
    band = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    bd = ImageDraw.Draw(band)
    bd.rectangle((0, BAND_Y, SIZE, BAND_Y + int(SIZE * 0.012)), fill=(11, 15, 20, 255))
    bd.line((SIZE * 0.08, BAND_Y, SIZE * 0.92, BAND_Y), fill=CYAN, width=int(SIZE * 0.0045))
    for step in range(7):
        x = SIZE * 0.08 + step * SIZE * 0.14
        bd.rectangle(
            (x - int(SIZE * 0.004), BAND_Y - int(SIZE * 0.008), x + int(SIZE * 0.004), BAND_Y + int(SIZE * 0.02)),
            fill=CYAN,
        )
    bd.line(
        (SIZE * 0.08, BAND_Y + int(SIZE * 0.012), SIZE * 0.92, BAND_Y + int(SIZE * 0.012)),
        fill=CYAN_DIM,
        width=int(SIZE * 0.0018),
    )
    plate = Image.alpha_composite(plate, band)

    # Clip to the rounded square and downsample.
    out = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    out.paste(plate, (0, 0), rounded_mask(SIZE, int(SIZE * 0.22)))
    out = out.resize((FINAL, FINAL), Image.Resampling.LANCZOS)

    out.save(OUT / "recraft.png")
    out.save(
        OUT / "recraft.ico",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    # Raw RGBA for the runtime window icon (egui::IconData) without a PNG decoder.
    (OUT / "recraft-64.rgba").write_bytes(out.resize((64, 64), Image.Resampling.LANCZOS).tobytes())
    print("wrote", OUT / "recraft.png", ",", OUT / "recraft.ico", "and", OUT / "recraft-64.rgba")


if __name__ == "__main__":
    main()
