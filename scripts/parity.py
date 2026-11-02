"""Confronta il rendering Rust con quello Python, pixel per pixel.

Uso:
    python scripts/parity.py [--python-repo PATH] [--theme paper|dark]

Richiede il repo Python di WikiWall (di default `..\\wikiwall` o la variabile
d'ambiente WIKIWALL_PYTHON_REPO) con il suo venv e Playwright installato, più il
binario Rust compilato in `target\\release\\wikiwall.exe`.

Entrambe le versioni disegnano la stessa voce (letta da
`%LOCALAPPDATA%\\WikiWall\\cache\\last_article.json`) e i due PNG vengono confrontati.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import subprocess
import sys

from PIL import Image, ImageChops


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

    subprocess.run(
        [
            str(binary),
            "next",
            "--no-set",
            "--output",
            str(rust_png),
            "--theme",
            args.theme,
            "--width",
            str(args.width),
            "--height",
            str(args.height),
        ],
        check=True,
    )

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
    histogram = difference.histogram()
    total = sum(histogram)
    identical = histogram[0]
    visible = sum(histogram[9:])

    print(f"dimensione: {rust_image.size[0]}x{rust_image.size[1]}")
    print(f"pixel identici: {100.0 * identical / total:.4f}%")
    print(f"pixel con differenza > 8: {visible} ({100.0 * visible / total:.6f}%)")
    print(f"differenza massima: {difference.getextrema()[1]}")
    return 0 if visible == 0 else 2


if __name__ == "__main__":
    sys.exit(main())
