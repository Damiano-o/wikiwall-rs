"""Confronta il rendering Rust con quello Python, pixel per pixel.

Uso:
    python scripts/parity.py [--python-repo PATH] [--theme paper|dark]

Richiede il repo Python di WikiWall (di default `..\\wikiwall` o la variabile
d'ambiente WIKIWALL_PYTHON_REPO) con il suo venv e Playwright installato, più il
binario Rust compilato in `target\\release\\wikiwall.exe`.

Entrambe le versioni disegnano la stessa voce (letta da
`%LOCALAPPDATA%\\WikiWall\\cache\\last_article.json`) e i due PNG vengono confrontati.
Il QR viene misurato a parte: la crate `qrcode` e python-qrcode scelgono a volte
maschere diverse (entrambe valide), quindi il riquadro del QR può non coincidere
pur essendo corretto.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import re
import subprocess
import sys

from PIL import Image, ImageChops, ImageDraw

QR_BOX = re.compile(r"box qr=Some\(\(([\d.]+), ([\d.]+), ([\d.]+), ([\d.]+)\)\)")


def run_rust(binary: pathlib.Path, args: argparse.Namespace, output: pathlib.Path) -> tuple[int, int, int, int] | None:
    """Render with the Rust binary; return the QR box it reported."""
    result = subprocess.run(
        [
            str(binary),
            "next",
            "--no-set",
            "-v",
            "--output",
            str(output),
            "--theme",
            args.theme,
            "--width",
            str(args.width),
            "--height",
            str(args.height),
        ],
        check=True,
        capture_output=True,
        text=True,
        errors="replace",
    )
    match = QR_BOX.search(result.stderr + result.stdout)
    if match is None:
        return None
    return tuple(float(value) for value in match.groups())  # type: ignore[return-value]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    rust_repo = pathlib.Path(__file__).resolve().parent.parent
    parser.add_argument(
        "--python-repo",
        default=os.environ.get("WIKIWALL_PYTHON_REPO", str(rust_repo.parent / "wikiwall")),
    )
    parser.add_argument("--theme", default="dark")
    parser.add_argument("--width", type=int, default=1920)
    parser.add_argument("--height", type=int, default=1080)
    args = parser.parse_args()

    python_repo = pathlib.Path(args.python_repo)
    local = pathlib.Path(os.environ.get("LOCALAPPDATA", pathlib.Path.home() / "AppData/Local"))
    article_path = local / "WikiWall" / "cache" / "last_article.json"
    rust_png = rust_repo / "parity-rust.png"
    python_png = rust_repo / "parity-python.png"

    if not article_path.is_file():
        print(f"Articolo non trovato: {article_path}. Esegui prima: wikiwall next")
        return 1

    binary = rust_repo / "target" / "release" / "wikiwall.exe"
    if not binary.is_file():
        print(f"Binario non trovato: {binary}. Esegui prima: cargo build --release")
        return 1

    qr_box = run_rust(binary, args, rust_png)

    python = python_repo / ".venv" / "Scripts" / "python.exe"
    if not python.is_file():
        print(f"Python del venv non trovato: {python}")
        return 1

    script = f"""
import json, pathlib
from wikiwall.wikipedia import Article
from wikiwall.render import render_article
data = json.loads(pathlib.Path({str(article_path)!r}).read_text(encoding="utf-8"))
article = Article.from_dict(data)
print("articolo:", article.title)
result = render_article(
    article,
    pathlib.Path({str(python_png)!r}),
    {args.width},
    {args.height},
    {args.theme!r},
)
print("python boxes:", result.qr_box, result.link_box, result.skip_box)
"""
    subprocess.run(
        [str(python), "-c", script],
        check=True,
        cwd=python_repo,
        env={**os.environ, "PYTHONPATH": str(python_repo / "src")},
    )

    rust_image = Image.open(rust_png).convert("RGB")
    python_image = Image.open(python_png).convert("RGB")
    if rust_image.size != python_image.size:
        print(f"Dimensioni diverse: {rust_image.size} vs {python_image.size}")
        return 1

    difference = ImageChops.difference(rust_image, python_image).convert("L")

    # Regione del QR: le due implementazioni possono scegliere maschere diverse.
    qr_mask = Image.new("L", rust_image.size, 0)
    if qr_box is not None:
        left, top, width, height = qr_box
        padding = 4
        ImageDraw.Draw(qr_mask).rectangle(
            (
                max(0, int(left) - padding),
                max(0, int(top) - padding),
                min(rust_image.size[0], int(left + width) + padding),
                min(rust_image.size[1], int(top + height) + padding),
            ),
            fill=255,
        )

    outside = ImageChops.multiply(
        difference, ImageChops.invert(qr_mask)
    )  # solo fuori dal QR
    inside = ImageChops.multiply(difference, qr_mask)  # solo dentro il QR

    def report(name: str, image: Image.Image) -> int:
        histogram = image.histogram()
        total = sum(histogram)
        visible = sum(histogram[9:])
        print(
            f"{name}: pixel identici {100.0 * histogram[0] / total:.4f}% · "
            f"differenza > 8 in {visible} pixel ({100.0 * visible / total:.6f}%) · "
            f"max {image.getextrema()[1]}"
        )
        return visible

    layout_diff = report("fuori dal QR (testo, impaginazione)", outside)
    report("dentro il QR", inside)

    if layout_diff == 0:
        print("esito: impaginazione identica; eventuali differenze restano nel QR")
        return 0
    print("esito: differenze fuori dal QR")
    return 2


if __name__ == "__main__":
    sys.exit(main())
