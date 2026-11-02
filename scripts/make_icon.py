"""Genera assets/wikiwall.ico: carta, dorso rosso e "W" in Georgia bold.

Uso: python scripts/make_icon.py
Serve solo quando si vuole rigenerare l'icona: il file .ico è già nel repo.
"""

from __future__ import annotations

import pathlib

from PIL import Image, ImageDraw, ImageFont

PAPER = (243, 237, 226, 255)
INK = (37, 35, 32, 255)
ACCENT = (151, 3, 2, 255)
SIZES = [16, 24, 32, 48, 64, 128, 256]


def font_for(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    for name in ("georgiab.ttf", "georgia.ttf", "timesbd.ttf", "segoeuib.ttf"):
        try:
            return ImageFont.truetype(name, max(9, int(size * 0.58)))
        except OSError:
            continue
    return ImageFont.load_default()


def render(size: int) -> Image.Image:
    scale = 8  # disegna grande e riduci: bordi puliti anche a 16 px
    work = size * scale
    image = Image.new("RGBA", (work, work), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    margin = round(work * 0.045)
    radius = round(work * 0.19)
    draw.rounded_rectangle(
        (margin, margin, work - margin - 1, work - margin - 1),
        radius=radius,
        fill=INK,
    )
    inner = margin + round(work * 0.055)
    draw.rounded_rectangle(
        (inner, inner, work - inner - 1, work - inner - 1),
        radius=max(1, radius - round(work * 0.05)),
        fill=PAPER,
    )

    spine = round(work * 0.035)
    left = inner + round(work * 0.045)
    draw.rounded_rectangle(
        (left, inner + round(work * 0.02), left + spine, work - inner - round(work * 0.02)),
        radius=max(1, spine // 3),
        fill=ACCENT,
    )

    font = font_for(work)
    draw.text(
        (work * 0.56, work * 0.53),
        "W",
        font=font,
        fill=INK,
        anchor="mm",
    )
    return image.resize((size, size), Image.LANCZOS)


def main() -> int:
    root = pathlib.Path(__file__).resolve().parent.parent
    target = root / "assets" / "wikiwall.ico"
    # Pillow ridimensiona l'immagine sorgente per ogni `size`: serve la più grande.
    largest = render(max(SIZES))
    largest.save(
        target,
        format="ICO",
        sizes=[(size, size) for size in SIZES],
    )
    preview = root / "assets" / "wikiwall-256.png"
    largest.save(preview, format="PNG")
    print(f"scritto {target} ({target.stat().st_size} byte) e {preview}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
