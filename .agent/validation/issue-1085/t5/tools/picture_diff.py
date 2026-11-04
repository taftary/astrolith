"""#1085 T5: pixel differences between picture folders (numpy + Pillow).

Usage: python picture_diff.py <folder-a> <folder-b> [<folder-c> ...]
Compares every PNG of the first folder against the same name in each other
folder: share of pixels differing by more than 8/255 in any channel, mean
absolute difference (0-255) and the largest channel difference.
"""

import sys
from pathlib import Path

import numpy as np
from PIL import Image


def read_png(path):
    with Image.open(path) as image:
        return np.asarray(image.convert("RGB"), dtype=np.int32)


def main():
    first, *others = map(Path, sys.argv[1:])
    print("| picture | " + " | ".join(f"vs {o.name}: changed px / mean / max" for o in others) + " |")
    print("|---|" + "---|" * len(others))
    for png in sorted(first.glob("*.png")):
        a = read_png(png)
        cells = []
        for other in others:
            d = np.abs(a - read_png(other / png.name))
            changed = (d.max(axis=2) > 8).mean()
            cells.append(f"{100 * changed:.2f}% / {d.mean():.2f} / {d.max()}")
        print(f"| {png.stem} | " + " | ".join(cells) + " |")


if __name__ == "__main__":
    main()
