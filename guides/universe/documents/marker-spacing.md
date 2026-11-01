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

## 2. Real gaps

- **What is compared.** At each level, the real objects of the same size as
  the game's portal markers (the next level's size): Sloan-Great-Wall-sized
  structures at L1, rich clusters at L2, galaxies in Virgo at L3, molecular
  clouds in the Milky Way at L4, stars at L5 and L6, the Sun's own system at
  L7.
- **Gap from a density.** Published numbers are densities or counts in a
  volume. For objects scattered at random with density `n`, the median
  nearest-neighbour gap is `(3 ln 2 / (4 pi n))^(1/3)` in 3D, and
  `sqrt(ln 2 / (pi s))` for a surface density `s` in a thin disk. This is the
  same median the game column uses. Real objects cluster, so the true median
  is somewhat shorter than this: the real gaps below are an upper bound.
- **Units.** 1 pc = 3.0857e16 m, 1 Mpc = 3.0857e22 m, 1 ly = 9.4607e15 m,
  1 AU = 1.496e11 m. Cluster densities in `h^3 Mpc^-3` use `h = 0.674` [R2].

| Level | Real objects | Published number | Real median gap (m) | Real count in one cell |
|---|---|---|---|---|
| L1 | Superclusters; regions the size of an L2 cell | About 10 million superclusters in the observable universe [R1], 8.8e26 m across [R3] | 1.8e24 (superclusters) | 10 million superclusters; about 310,000 L2-sized regions fill the cell |
| L2 | Rich galaxy clusters (Abell richness 0 and up, 1 and up) | 13.5e-6 and 6.0e-6 `h^3 Mpc^-3`, mean separations 42 and 55 `h^-1 Mpc` [R4] | 1.06e24 (R 0+), 1.38e24 (R 1+) | 300 (R 0+), 134 (R 1+) |
| L3 | Galaxies of the Virgo cluster | About 2,000 galaxies, 90 percent dwarfs, in 15 million ly; its map labels the 160 largest [R5] | 5.0e21 (all), 1.16e22 (160 largest only) | 2,000 (160 largest) |
| L4 | Molecular clouds of the Milky Way disk | 8,107 clouds over the whole Galactic plane [R7]; disk about 100,000 ly across [R8] | 4.4e18 (thin disk) | 8,107 |
| L5 | Stars near the Sun | 0.14 stars per cubic parsec [R9] | 3.3e16 | About 140,000 |
| L6 | Star systems near the Sun | Same density [R9] | 3.3e16 | 0.35 (most cells this size hold no star) |
| L7 | The Sun's own system; Oort cloud bodies | One star; hundreds of billions to trillions of icy bodies [R10] | One star; 1.3e12 between bodies if a trillion fill 100,000 AU | One star; up to a trillion small bodies |

### Sources

Every address below was fetched and read on 2026-11-01; the quoted
fragment is what the number rests on.

- **[R1]** Supercluster, Wikipedia: "The number of superclusters in the observable universe is estimated to be 10 million." https://en.wikipedia.org/wiki/Supercluster
- **[R2]** Planck 2018 results VI, arXiv 1807.06209: "Hubble constant H0 = (67.4 ± 0.5) km/s/Mpc". https://arxiv.org/abs/1807.06209
- **[R3]** Observable universe, Wikipedia (diameter 93 Gly, 8.8e26 m; ladder R5 anchor). https://en.wikipedia.org/wiki/Observable_universe
- **[R4]** Bahcall, Clusters and cosmology, NED Level 5, Table 3 (Bahcall and Cen 1993): richness 0, 30 galaxies, 13.5e-6, 42; richness 1, 50 galaxies, 6.0e-6, 55. https://ned.ipac.caltech.edu/level5/Sept01/Bahcall2/Bahcall2_3.html
- **[R5]** The Virgo Cluster, Atlas of the Universe: "There are roughly 2000 galaxies in this cluster (although ninety percent of them are dwarf galaxies). This cluster has a diameter of approximately 15 million light years". http://www.atlasoftheuniverse.com/galgrps/vir.html
- **[R6]** Same Atlas page as [R5]: "The 160 largest galaxies are labelled on this map." This is the map's own cut, used here only as a count of the big members.
- **[R7]** Miville-Deschênes, Murray and Lee 2017, arXiv 1610.05918: "a catalog of 8107 molecular clouds that covers the entire Galactic plane". https://arxiv.org/abs/1610.05918
- **[R8]** The Milky Way Galaxy, NASA Imagine the Universe: "The Milky Way is approximately 100,000 light-years in diameter." https://imagine.gsfc.nasa.gov/science/featured_science/milkyway/
- **[R9]** Stellar density, Wikipedia: "The true stellar density near the Sun is estimated as 0.004 stars per cubic light year, or 0.14 stars pc^-3." https://en.wikipedia.org/wiki/Stellar_density
- **[R10]** Oort Cloud Facts, NASA: "There may be hundreds of billions, even trillions, of icy bodies in the Oort Cloud." https://science.nasa.gov/solar-system/oort-cloud/facts/
