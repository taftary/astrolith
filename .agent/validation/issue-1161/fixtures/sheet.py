"""#1161: a contact sheet of one picture folder (rows: worlds, columns: views).

Usage: py .agent/validation/issue-1161/fixtures/sheet.py <pictures dir> <out.png>
"""
import sys
from PIL import Image, ImageDraw

NAMES = ["home", "mercury", "desert", "iron", "ice", "lava"]
VIEWS = ["plain", "mountain", "pole", "edge", "orbit", "night"]
W, H = 320, 180


def main(src, dst):
    sheet = Image.new("RGB", (W * 6 + 80, H * 7 + 20), (20, 20, 20))
    d = ImageDraw.Draw(sheet)
    for r, n in enumerate(NAMES):
        d.text((4, 20 + r * H + 80), n, fill=(255, 255, 255))
        for c, v in enumerate(VIEWS):
            sheet.paste(Image.open(f"{src}/{n}-{v}.png").convert("RGB").resize((W, H)), (80 + c * W, 20 + r * H))
            if r == 0:
                d.text((80 + c * W + 4, 4), v, fill=(255, 255, 255))
    d.text((4, 20 + 6 * H + 80), "moon", fill=(255, 255, 255))
    for i, v in enumerate(["moon-space", "moon-planet"]):
        sheet.paste(Image.open(f"{src}/{v}.png").convert("RGB").resize((W, H)), (80 + i * W, 20 + 6 * H))
    sheet.save(dst)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
