# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
"""Generate the ReCraft application icon (PNG + multi-size ICO + raw RGBA).

The mark is the ReCraft "R" above a cyan scan line with byte ticks: the scan line is
what a reverse engineer sees under the cursor, the ticks are the bytes it reads.

Two variants are rendered, because one drawing cannot survive both ends of the size
range:

* **compact** (< 48 px): a 5×7 bitmap "R" plus a 2 px cyan rule and three ticks, drawn
  at the final resolution with hard pixels. Downsampling the large master turned the
  letter into a pale bar and the 1 %-thick scan line into nothing at 16 px.
* **detailed** (>= 48 px): the geometric R, glow, rim, drop shadow and seven ticks,
  rendered 4× and downsampled.

Everything is drawn from primitives so the icon can be regenerated without a design
tool or a font file. Requires Pillow.

    python assets/generate_icon.py
"""

from __future__ import annotations

import io
import struct
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets"
SKELETON = 2048  # canvas for the R geometry (the letter reaches x=1270)
FINAL = 1024
SIZES = [16, 24, 32, 48, 64, 128, 256]
SUPERSAMPLE = 4

# Palette: a dark slate plate, an ink-white letter, one cyan accent.
PLATE_TOP = (30, 41, 54)
PLATE_BOTTOM = (15, 20, 27)
INK_TOP = (247, 252, 255)
INK_BOTTOM = (196, 219, 228)
CYAN = (64, 205, 224, 255)
CYAN_DIM = (43, 128, 143, 255)
RIM = (70, 92, 110, 150)

# Bitmap "R" for the compact variant (5×7).
R_GLYPH = [
    "11110",
    "10001",
    "10001",
    "11110",
    "10100",
    "10010",
    "10001",
]


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


def draw_r(draw: ImageDraw.ImageDraw, fill: int = 255) -> None:
    """Geometric R with heavy rounded strokes, drawn on the 1024 px skeleton."""
    stroke = 220
    draw.rounded_rectangle((600, 500, 820, 1500), radius=110, fill=fill)
    draw.arc((470, 430, 1130, 1090), start=-92, end=92, fill=fill, width=stroke)
    draw.rounded_rectangle((700, 430, 900, 650), radius=70, fill=fill)
    draw.rounded_rectangle((700, 870, 900, 1090), radius=70, fill=fill)
    draw.line((820, 1050, 1270, 1490), fill=fill, width=stroke)
    draw.ellipse((820 - stroke // 2, 1050 - stroke // 2, 820 + stroke // 2, 1050 + stroke // 2), fill=fill)
    draw.ellipse((1270 - stroke // 2, 1490 - stroke // 2, 1270 + stroke // 2, 1490 + stroke // 2), fill=fill)


def render_compact(size: int) -> Image.Image:
    plate = vertical_gradient(size, PLATE_TOP, PLATE_BOTTOM)
    draw = ImageDraw.Draw(plate)

    cell = max(1, size // 12)
    glyph_w = len(R_GLYPH[0]) * cell
    glyph_h = len(R_GLYPH) * cell
    left = (size - glyph_w) // 2
    top = max(1, int(size * 0.36) - glyph_h // 2)
    for row, line in enumerate(R_GLYPH):
        for column, bit in enumerate(line):
            if bit == "1":
                x = left + column * cell
                y = top + row * cell
                draw.rectangle((x, y, x + cell - 1, y + cell - 1), fill=INK_TOP)

    # Cyan scan rule with three byte ticks, thick enough to survive 16 px.
    line_y = min(size - 4, int(size * 0.70))
    line_h = max(2, size // 10)
    draw.rectangle(
        (int(size * 0.18), line_y, size - int(size * 0.18), line_y + line_h),
        fill=CYAN,
    )
    tick_w = max(1, size // 14)
    tick_h = max(3, size // 5)
    for step in range(3):
        x = int(size * (0.30 + step * 0.20))
        draw.rectangle(
            (x - tick_w, line_y - tick_h // 3, x + tick_w, line_y + line_h + tick_h // 3),
            fill=CYAN,
        )

    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.paste(plate, (0, 0), rounded_mask(size, int(size * 0.22)))
    return out


def render_detailed(size: int) -> Image.Image:
    canvas = size * SUPERSAMPLE
    plate = vertical_gradient(canvas, PLATE_TOP, PLATE_BOTTOM)

    rim = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    ImageDraw.Draw(rim).rounded_rectangle(
        (int(canvas * 0.006), int(canvas * 0.006), canvas - int(canvas * 0.006), canvas - int(canvas * 0.006)),
        radius=int(canvas * 0.215),
        outline=RIM,
        width=max(1, int(canvas * 0.002)),
    )
    plate = Image.alpha_composite(plate, rim)

    halo = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    ImageDraw.Draw(halo).ellipse(
        (int(canvas * 0.23), int(canvas * 0.18), int(canvas * 0.68), int(canvas * 0.67)),
        fill=(53, 205, 214, 46),
    )
    halo = halo.filter(ImageFilter.GaussianBlur(canvas * 0.06))
    plate = Image.alpha_composite(plate, halo)

    # Letter: draw on the skeleton, crop to its ink box, then place it centred so the mark
    # fills the plate instead of drifting to the right edge.
    skeleton = Image.new("L", (SKELETON, SKELETON), 0)
    draw_r(ImageDraw.Draw(skeleton))
    box = skeleton.getbbox() or (0, 0, SKELETON, SKELETON)
    letter_mask = skeleton.crop(box)
    height = int(canvas * 0.66)
    width = max(1, int(letter_mask.width * height / letter_mask.height))
    letter_mask = letter_mask.resize((width, height), Image.Resampling.LANCZOS)
    r_mask = Image.new("L", (canvas, canvas), 0)
    r_mask.paste(letter_mask, ((canvas - width) // 2, int(canvas * 0.40) - height // 2))

    shadow = Image.new("RGBA", (canvas, canvas), (6, 10, 14, 255))
    shadow.putalpha(
        r_mask.filter(ImageFilter.GaussianBlur(canvas * 0.013)).point(lambda value: int(value * 0.45))
    )
    plate = Image.alpha_composite(plate, shadow)

    ink = vertical_gradient(canvas, INK_TOP, INK_BOTTOM)
    letter = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    letter.paste(ink, (0, 0), r_mask)
    plate = Image.alpha_composite(plate, letter)

    # Scan band: a dark strip cuts the letter, a cyan rule runs across it, and seven ticks
    # mark the bytes it reads.
    band_y = int(canvas * 0.70)
    band = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    draw = ImageDraw.Draw(band)
    draw.rectangle((0, band_y, canvas, band_y + max(1, int(canvas * 0.012))), fill=(11, 15, 20, 255))
    draw.line((canvas * 0.08, band_y, canvas * 0.92, band_y), fill=CYAN, width=max(1, int(canvas * 0.0045)))
    for step in range(7):
        x = canvas * 0.08 + step * canvas * 0.14
        draw.rectangle(
            (
                x - int(canvas * 0.004),
                band_y - int(canvas * 0.008),
                x + int(canvas * 0.004),
                band_y + int(canvas * 0.02),
            ),
            fill=CYAN,
        )
    draw.line(
        (canvas * 0.08, band_y + int(canvas * 0.012), canvas * 0.92, band_y + int(canvas * 0.012)),
        fill=CYAN_DIM,
        width=max(1, int(canvas * 0.0018)),
    )
    plate = Image.alpha_composite(plate, band)

    out = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    out.paste(plate, (0, 0), rounded_mask(canvas, int(canvas * 0.22)))
    return out.resize((size, size), Image.Resampling.LANCZOS)


def render(size: int) -> Image.Image:
    return render_compact(size) if size < 48 else render_detailed(size)


def write_ico(path: Path, frames: dict[int, Image.Image]) -> None:
    """Write a PNG-compressed ICO with one custom frame per size."""
    payloads = []
    for size in sorted(frames):
        buffer = io.BytesIO()
        frames[size].save(buffer, format="PNG", optimize=True)
        payloads.append((size, buffer.getvalue()))

    header = struct.pack("<HHH", 0, 1, len(payloads))
    offset = len(header) + 16 * len(payloads)
    directory = b""
    for size, payload in payloads:
        directory += struct.pack(
            "<BBBBHHII",
            0 if size >= 256 else size,
            0 if size >= 256 else size,
            0,
            0,
            1,
            32,
            len(payload),
            offset,
        )
        offset += len(payload)
    with path.open("wb") as handle:
        handle.write(header + directory + b"".join(payload for _, payload in payloads))


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    frames = {size: render(size) for size in SIZES}
    render(FINAL).save(OUT / "recraft.png")
    write_ico(OUT / "recraft.ico", frames)
    # Raw RGBA for the runtime window icon (egui::IconData) without a PNG decoder.
    (OUT / "recraft-64.rgba").write_bytes(frames[64].tobytes())
    print("wrote", OUT / "recraft.png", ",", OUT / "recraft.ico", "and", OUT / "recraft-64.rgba")


if __name__ == "__main__":
    main()
