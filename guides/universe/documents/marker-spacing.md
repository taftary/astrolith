# Marker spacing study, L1 to L7

Issue #996, Spec v1 (study only). How far apart neighbouring markers are in
the game, against how far apart the real objects are, for each level from
L1 to L7. This page proposes; it changes nothing in the game. Each fix is
chosen by the owner and built later through its own spec version.

L8 to L10 are left out: they already sit at true distances (#995, ADR 0051).

## 1. How the game numbers are measured

- **Source.** The capture mode replays the fixed journey at `DEMO_SEED` and
  writes, for each level, every marker of the cell the journey enters, with
  its kind (portal or population), form and position in cell units:
  `cargo run --locked -p universe-app -- --capture <dir>` gives
  `level-L*.txt`.
- **Cell size.** A level-`l` cell is `10^e_l` metres across, with `e_l` from
  `LADDER_EXPONENTS` in `crates/universe-core/src/nest.rs` (ladder R5 sizes).
  Positions run from -0.5 to 0.5 cells.
- **What a marker is.** A portal marker at level `l` is a whole level-`l+1`
  cell (ladder R6): at L3 a galaxy, at L6 a star system. A population marker
  is a group the player cannot open (a ring of field galaxies, a shell of
  comets).
- **Gap.** For every marker, the distance to its nearest neighbour; the
  table gives the median and the 10th to 90th percentile, all markers and
  then portals only. Metres are cells times the cell size.
- **Measured on** `main` at `79bc26c`. Only the one cell per level that the
  journey enters is measured.

### Game gaps (measured)

| Level | Cell size (m) | Markers (portals + populations) | Median gap, all (cells) | 10th-90th (cells) | Median gap, all (m) | Median gap, portals (m) |
|---|---|---|---|---|---|---|
| L1 | 8.71e+26 | 8 (8 + 0) | 0.500 | 0.500-0.500 | 4.36e+26 | 4.36e+26 |
| L2 | 1.29e+25 | 28 (3 + 25) | 0.201 | 0.097-0.285 | 2.58e+24 | 1.99e+24 |
| L3 | 1.41e+23 | 33 (10 + 23) | 0.114 | 0.053-0.265 | 1.61e+22 | 2.16e+22 |
| L4 | 9.55e+20 | 32 (5 + 27) | 0.132 | 0.081-0.222 | 1.26e+20 | 4.82e+20 |
| L5 | 3.09e+18 | 32 (4 + 28) | 0.017 | 0.003-0.061 | 5.09e+16 | 1.98e+17 |
| L6 | 4.17e+16 | 6 (2 + 4) | 0.156 | 0.017-0.419 | 6.52e+15 | 2.33e+16 |
| L7 | 1.48e+16 | 32 (1 + 31) | 0.118 | 0.053-0.211 | 1.75e+15 | one portal |

### Measuring script

Run it on a fresh capture directory: `python -I nn.py <capture dir>`.

```python
"""Nearest-neighbour gap between markers per level, from a --capture dir.

Usage: python -I nn.py <capture dir>. Cell sizes: 10^e metres with e from
LADDER_EXPONENTS (crates/universe-core/src/nest.rs); positions are in cells.
"""
import glob, math, os, re, sys
EXP = [26.94, 25.11, 23.15, 20.98, 18.49, 16.62, 16.17]
LINE = re.compile(r"kind=(\w+) form=(\S+) x=(\S+) y=(\S+) z=(\S+) r=(\S+)")

def pct(v, q):
    v = sorted(v); i = (len(v) - 1) * q; lo = math.floor(i)
    return v[lo] + (v[min(lo + 1, len(v) - 1)] - v[lo]) * (i - lo)

def nn(pts):
    return [min(math.dist(p, q) for j, q in enumerate(pts) if j != i)
            for i, p in enumerate(pts)]

for lv in range(1, 8):
    path = glob.glob(os.path.join(sys.argv[1], f"level-L{lv}-*.txt"))[0]
    rows = [(m[0], tuple(map(float, m[2:5]))) for m in LINE.findall(open(path).read())]
    cell = 10 ** EXP[lv - 1]
    for name, sel in (("all", rows), ("portal", [r for r in rows if r[0] == "portal"]),
                      ("population", [r for r in rows if r[0] == "population"])):
        pts = [p for _, p in sel]
        if len(pts) < 2:
            print(f"L{lv} {name:10} n={len(pts)}"); continue
        d = nn(pts)
        print(f"L{lv} {name:10} n={len(pts):2} cell_m={cell:.3e} "
              f"nn_cells p10={pct(d, .1):.4f} median={pct(d, .5):.4f} p90={pct(d, .9):.4f} "
              f"median_m={pct(d, .5) * cell:.3e}")
```
