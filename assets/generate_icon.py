# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
"""Generate the ReCraft application icon (PNG + multi-size ICO).

The mark is a geometric R inside a hexagon: "R" for ReCraft, the hexagon for
reverse-engineering / binary structure, and circuit nodes for the MCP/AI layer.
Everything is drawn from primitives so the icon can be regenerated without a design
tool. Requires Pillow and numpy.

    python assets/generate_icon.py
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets"
SIZE = 2048
FINAL = 1024


def lerp(a: tuple[int, int, int], b: tuple[int, int, int], t: np.ndarray) -> np.ndarray:
    a_arr = np.array(a, dtype=np.float32)
    b_arr = np.array(b, dtype=np.float32)
    return a_arr + (b_arr - a_arr) * t[..., None]


def gradient(size: int, start: tuple[int, int, int], end: tuple[int, int, int]) -> Image.Image:
    y, x = np.mgrid[0:size, 0:size].astype(np.float32)
    t = np.clip((x + y) / (2.0 * (size - 1)), 0.0, 1.0)
    rgb = lerp(start, end, t).clip(0, 255).astype(np.uint8)
    return Image.fromarray(rgb, "RGB").convert("RGBA")


def rounded_mask(size: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size - 1, size - 1), radius, fill=255)
    return mask


def draw_r(draw: ImageDraw.ImageDraw, fill: int = 255) -> None:
    """Geometric R with heavy rounded strokes."""
    stroke = 220
    # Stem.
    draw.rounded_rectangle((600, 500, 820, 1500), radius=110, fill=fill)
    # Bowl: right half of a thick ring.
    draw.arc((470, 430, 1130, 1090), start=-92, end=92, fill=fill, width=stroke)
    # Fill the top/bottom junctions so the bowl reads as one solid shape.
    draw.rounded_rectangle((700, 430, 900, 650), radius=70, fill=fill)
    draw.rounded_rectangle((700, 870, 900, 1090), radius=70, fill=fill)
    # Diagonal leg.
    draw.line((820, 1050, 1270, 1490), fill=fill, width=stroke)
    draw.ellipse((820 - stroke // 2, 1050 - stroke // 2, 820 + stroke // 2, 1050 + stroke // 2), fill=fill)
    draw.ellipse((1270 - stroke // 2, 1490 - stroke // 2, 1270 + stroke // 2, 1490 + stroke // 2), fill=fill)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)

    # Background: deep ocean gradient inside a rounded square.
    bg = gradient(SIZE, (7, 27, 38), (12, 58, 70))
    glow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gd.ellipse((-500, -650, 1500, 1200), fill=(53, 220, 196, 44))
    glow = glow.filter(ImageFilter.GaussianBlur(180))
    bg = Image.alpha_composite(bg, glow)

    # Faint hex grid for texture (kept subtle so small sizes stay clean).
    grid = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(grid)
    cx = cy = SIZE / 2
    import math

    radius = 910
    for ring, alpha in ((1.0, 20), (0.62, 14)):
        pts = [
            (
                cx + math.cos(math.radians(60 * i - 30)) * radius * ring,
                cy + math.sin(math.radians(60 * i - 30)) * radius * ring,
            )
            for i in range(6)
        ]
        gdraw.polygon(pts, outline=(96, 235, 215, alpha))
    bg = Image.alpha_composite(bg, grid)

    # Hexagon frame and circuit nodes.
    frame = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    fd = ImageDraw.Draw(frame)
    radius = 760
    pts = [
        (
            cx + math.cos(math.radians(60 * i - 30)) * radius,
            cy + math.sin(math.radians(60 * i - 30)) * radius,
        )
        for i in range(6)
    ]
    fd.polygon(pts, outline=(95, 245, 224, 120), width=10)
    for px, py in pts:
        fd.ellipse((px - 17, py - 17, px + 17, py + 17), fill=(150, 255, 238, 210))
    # Circuit trace from the top-right node toward the mark.
    fd.line((pts[0][0], pts[0][1], 1350, 560), fill=(95, 245, 224, 150), width=8)
    fd.ellipse((1324, 534, 1376, 586), fill=(150, 255, 238, 235))
    bg = Image.alpha_composite(bg, frame)

    # Soft glow behind the monogram.
    halo = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    hd = ImageDraw.Draw(halo)
    hd.ellipse((430, 330, 1430, 1570), fill=(40, 220, 205, 55))
    halo = halo.filter(ImageFilter.GaussianBlur(90))
    bg = Image.alpha_composite(bg, halo)

    # Monogram cut from a bright mint-to-cyan gradient.
    r_mask = Image.new("L", (SIZE, SIZE), 0)
    draw_r(ImageDraw.Draw(r_mask))
    r_fill = gradient(SIZE, (178, 255, 235), (30, 181, 205))
    shadow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    shadow.putalpha(r_mask.filter(ImageFilter.GaussianBlur(18)))
    shadow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 80))
    shadow.putalpha(r_mask.filter(ImageFilter.GaussianBlur(22)).point(lambda v: int(v * 0.55)))
    bg = Image.alpha_composite(bg, shadow.filter(ImageFilter.GaussianBlur(8)))

    mark = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    mark.paste(r_fill, (0, 0), r_mask)
    bg = Image.alpha_composite(bg, mark)

    # One bright node embedded in the leg.
    dot = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    dd = ImageDraw.Draw(dot)
    dd.ellipse((1218, 1438, 1322, 1542), fill=(7, 40, 48, 235))
    dd.ellipse((1240, 1460, 1300, 1520), fill=(170, 255, 239, 255))
    bg = Image.alpha_composite(bg, dot)

    # Clip to the rounded square and downsample.
    out = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    out.paste(bg, (0, 0), rounded_mask(SIZE, 330))
    out = out.resize((FINAL, FINAL), Image.Resampling.LANCZOS)
    out.save(OUT / "recraft.png")
    out.save(OUT / "recraft.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    # Raw RGBA for the runtime window icon (egui::IconData) without a PNG decoder.
    (OUT / "recraft-64.rgba").write_bytes(
        out.resize((64, 64), Image.Resampling.LANCZOS).tobytes()
    )
    print("wrote", OUT / "recraft.png", ",", OUT / "recraft.ico", "and", OUT / "recraft-64.rgba")


if __name__ == "__main__":
    main()
