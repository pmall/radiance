#!/usr/bin/env python3
"""Reference views: the same views at day, dusk and night, rendered with `--shot` and tiled into
one sheet per time of day, so every look change is judged on the same images.

    python3 tools/lookdev.py before        # renders into screenshots/lookdev/before/
    python3 tools/lookdev.py after --only spawn,square-d2 --times dusk

Needs a release build (`cargo build --release`) and Pillow. Views are seed + feet position
(x, y, z, yaw, pitch), as screenshot notes give them; `--report` lists places to add.
"""

import argparse
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "radiance"

# Name, seed, view (None: the spawn).
VIEWS = [
    ("spawn", 7, None),
    ("street-d0", 7, "12,-72,-60,200,0"),
    ("court-d1", 7, "168,-36,-96,45,0"),
    ("square-d2", 7, "216,0,250,0,0"),
    ("street-d3", 7, "-96,36,72,0,-5"),
    ("shrine-d2", 7, "-144,0,-196,0,0"),
    ("oldtown-d2", 7, "-436,0,-94,270,3"),
    ("stadium-d5", 7, "216,108,-400,180,5"),
    ("park-d3", 7, "-120,36,-72,90,0"),
    ("canopy-d6", 7, "12,144,-60,200,5"),
    ("grotto-d3", 1, "4,36,80,180,0"),
]
TIMES = [("day", 0.42), ("dusk", 0.74), ("night", 0.92)]
THUMB = (800, 450)


def render(out: Path, seed: int, view: str | None, time: float) -> None:
    cmd = [str(BIN), str(seed), "--time", str(time), "--shot", str(out)]
    if view:
        cmd += ["--view", view]
    subprocess.run(cmd, cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=120)


def sheet(out: Path, shots: list[tuple[str, Path]]) -> None:
    cols = 2
    rows = (len(shots) + cols - 1) // cols
    img = Image.new("RGB", (THUMB[0] * cols, THUMB[1] * rows))
    draw = ImageDraw.Draw(img)
    try:
        font = ImageFont.truetype("DejaVuSans.ttf", 22)
    except OSError:
        font = ImageFont.load_default()
    for i, (name, path) in enumerate(shots):
        x, y = (i % cols) * THUMB[0], (i // cols) * THUMB[1]
        if path.exists():
            img.paste(Image.open(path).convert("RGB").resize(THUMB), (x, y))
        draw.text((x + 10, y + 8), name, fill=(255, 255, 255), font=font, stroke_width=2, stroke_fill=(0, 0, 0))
    img.save(out)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("label", help="folder under screenshots/lookdev/")
    ap.add_argument("--only", help="comma-separated view names")
    ap.add_argument("--times", help="comma-separated: day, dusk, night")
    ap.add_argument("--jobs", type=int, default=3)
    a = ap.parse_args()
    if not BIN.exists():
        sys.exit("build first: cargo build --release")
    views = [v for v in VIEWS if not a.only or v[0] in a.only.split(",")]
    times = [t for t in TIMES if not a.times or t[0] in a.times.split(",")]
    out = ROOT / "screenshots" / "lookdev" / a.label
    out.mkdir(parents=True, exist_ok=True)
    jobs = [(out / f"{name}-{tname}.png", seed, view, t) for name, seed, view in views for tname, t in times]
    with ThreadPoolExecutor(a.jobs) as pool:
        list(pool.map(lambda j: render(*j), jobs))
    for tname, _ in times:
        sheet(out / f"sheet-{tname}.png", [(name, out / f"{name}-{tname}.png") for name, _, _ in views])
        print(out / f"sheet-{tname}.png")


if __name__ == "__main__":
    main()
