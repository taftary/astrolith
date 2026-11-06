"""#1162: a labelled contact sheet of every picture in one folder, in shot-list order.

Usage (from the repo root): py .agent/validation/issue-1162/fixtures/sheet.py <pictures dir> <out.png>
"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw

W, H, COLS = 400, 225, 4
SHOTS = Path(".agent/validation/issue-1162/fixtures/water.shots")


def names():
    return [l.split()[0] for l in SHOTS.read_text().splitlines() if l.strip() and not l.startswith("#")]


def main(src, dst):
    shots = [n for n in names() if Path(src, f"{n}.png").exists()]
    rows = (len(shots) + COLS - 1) // COLS
    sheet = Image.new("RGB", (W * COLS, (H + 16) * rows), (20, 20, 20))
    d = ImageDraw.Draw(sheet)
    for i, n in enumerate(shots):
        x, y = (i % COLS) * W, (i // COLS) * (H + 16)
        sheet.paste(Image.open(Path(src, f"{n}.png")).convert("RGB").resize((W, H)), (x, y + 16))
        d.text((x + 4, y + 2), n, fill=(255, 255, 255))
    sheet.save(dst)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
