"""Confronta il rendering Rust con quello Python, pixel per pixel.

Uso:
    python scripts/parity.py

Richiede: il repo Python clonato accanto (default ..\\wikiwall), il suo venv con
Playwright installato, e il binario Rust compilato (target\\release\\wikiwall.exe).

Entrambe le versioni disegnano la stessa voce (letta da cache\\last_article.json)
a 1920x1080 con il tema indicato, poi confronta i due PNG.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys

from PIL import Image, ImageChops


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python-repo", default=r"C:\Users\miche\wikiwall")
    parser.add_argument("--rust-repo", default=r"C:\Users\miche\wikiwall-rs")
    parser.add_argument("--theme", default="dark")
    parser.add_argument("--width", type=int, default=1920)
    parser.add_argument("--height", type=int, default=1080)
    args = parser.parse_args()

    python_repo = pathlib.Path(args.python_repo)
    rust_repo = pathlib.Path(args.rust_repo)
    article_path = pathlib.Path.home() / "AppData/Local/WikiWall/cache/last_article.json"
    rust_png = rust_repo / "parity-rust.png"
    python_png = rust_repo / "parity-python.png"

    if not article_path.is_file():
        print(f"Articolo non trovato: {article_path}. Esegui prima: wikiwall next")
        return 1

    subprocess.run(
        [
            str(rust_repo / "target/release/wikiwall.exe"),
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

    script = f"""
import json, pathlib
from wikiwall.wikipedia import Article
from wikiwall.render import render_article
data = json.loads(pathlib.Path(r"{article_path}").read_text(encoding="utf-8"))
article = Article.from_dict(data)
result = render_article(article, pathlib.Path(r"{python_png}"), {args.width}, {args.height}, "{args.theme}")
print("python boxes:", result.qr_box, result.link_box, result.skip_box)
"""
    python = python_repo / ".venv/Scripts/python.exe"
    subprocess.run(
        [str(python), "-c", script],
        check=True,
        cwd=python_repo,
        env={**__import__("os").environ, "PYTHONPATH": str(python_repo / "src")},
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
