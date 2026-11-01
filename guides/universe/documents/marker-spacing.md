# Marker spacing study, L1 to L7

Issue #996, Spec v1 (study only). How far apart neighbouring markers are in
the game, against how far apart the real objects are, for each level from
L1 to L7. This page proposes; it changes nothing in the game. Each fix is
chosen by the owner and built later through its own spec version.

L8 to L10 are left out: they already sit at true distances (#995, ADR 0051).

> **Correction (2026-11-01, Issue #996).** §3 to §5 compare each level's
> markers with the real objects of the markers' *size*. At L3, L4 and L5
> the game did not draw those objects: it drew the level above's objects,
> one level behind the size. Measured against what it drew, those levels
> were 40 to 86 times too crowded, not too sparse. §6 gives the corrected
> table, and §7 the result of the build that fixed it.

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

## 3. Game against real

"Too wide" means the game's markers sit farther apart than the real
objects; "too close" the reverse. The ratio is the game median over the real
median. The game column uses all markers and then portals only (§1).

| Level | Game gap, all / portals (m) | Real gap (m) | Ratio, all / portals | Game count | Real count |
|---|---|---|---|---|---|
| L1 | 4.4e26 / 4.4e26 | 1.8e24 | 240 / 240 times too wide | 8 | 10 million superclusters |
| L2 | 2.6e24 / 2.0e24 | 1.38e24 (R 1+) | 1.9 / 1.4 times too wide | 28 | 134 (R 1+) to 300 (R 0+) |
| L3 | 1.6e22 / 2.2e22 | 5.0e21 (all), 1.16e22 (160 largest) | 3.2 times too wide / 1.9 times against the largest | 33 | 2,000 |
| L4 | 1.3e20 / 4.8e20 | 4.4e18 | 29 / 110 times too wide | 32 | 8,107 |
| L5 | 5.1e16 / 2.0e17 | 3.3e16 | 1.6 / 6.1 times too wide | 32 | About 140,000 |
| L6 | 6.5e15 / 2.3e16 | 3.3e16 | 5 / 1.4 times too close | 6 | 0.35 |
| L7 | One star, 31 shells 1.7e15 apart | One star; small bodies 1.3e12 apart | Star right; shells are a stand-in | 32 | One star |

In short: the game is far too sparse at L1 and L4, clearly too sparse at L3,
close at L2 and L5 (but with far too few markers), too crowded at L6, and
right at L7 for the one thing that matters there, the star.

## 4. What fixing each level would change

At real density, Marker detail (Low 500, Normal 2,000, High 5,000; #998)
decides how many draw at once, nearest and biggest first (Q3 A). Every fix
moves the places the fixed seed makes at that level and below it. The golden
files `crates/universe-app/tests/golden/capture.txt` and `verify.txt` are
rewritten once with the reason, and saved views that point at a moved place
land somewhere else. The home journey (Milky Way, Sun, Earth) is pinned, so it
is kept in every option (Q4 A).

- **L1.** Real: 10 million superclusters, or about 310,000 L2-sized regions
  filling the cell. Far above every Marker detail level, and from outside the
  universe they are all equally far away, so "nearest first" picks nothing
  useful. Today's 8 markers are an index of the universe, and the real
  structure appears inside them at L2.
- **L2.** Real: 134 to 300 rich clusters. All three Marker detail levels hold
  them. Generation goes from 48 to about 134 to 300 points per cell. Every L2
  cell and everything below moves, apart from the pinned home path.
- **L3.** Real: 2,000 galaxies, 160 of them large. The 160 largest become
  portals, the dwarfs become population points. Normal holds exactly 2,000,
  High all of them, Low the 500 nearest and biggest. Virgo and the other
  L3 cells move; the Milky Way stays pinned.
- **L4.** Real: 8,107 molecular clouds in a thin disk. Above High (5,000),
  so the nearest and biggest draw. The cells become flat disks rather than
  round clouds of markers. The largest moves of all: the inside of every
  galaxy is laid out again, the home cloud stays pinned.
- **L5.** Real: about 140,000 stars in a 100-pc cell. No Marker detail level
  holds this, and generating them all at once is too slow and too big for
  this PC. A fix would make stars only around the camera, in small blocks
  (as the ground does), at real spacing. This is the costliest change and the
  one with the most risk to frame time (ADR 0044).
- **L6.** Real: 0.35 stars per cell, so most cells this size hold no star.
  The cell around a star holds that star, and its nearest neighbour is about
  as far as the cell is wide (Alpha Centauri is 1.34 pc from the Sun, the L6
  cell is 1.35 pc). The fix lowers the count from 6 to 1 or 2. The home cell
  keeps the Sun and Alpha Centauri.
- **L7.** Real: the one star, plus up to a trillion icy bodies, each a few
  km across, invisible at this scale. Today's shells stand for the Oort cloud
  as a whole. Real spacing would be hundreds of billions of invisible points.

## 5. Recommendation per level

| Level | Recommendation | Why |
|---|---|---|
| L1 | Leave | Real counts are far above any budget, all equally far from the camera; the 8 markers index the universe and the real structure shows at L2 |
| L2 | Fix | 134 to 300 clusters fit every Marker detail level; a cheap change |
| L3 | Fix | The headline case: Virgo with 2,000 galaxies instead of 33, within the Normal budget |
| L4 | Fix | The largest error (29 to 110 times too wide); 8,107 clouds with nearest and biggest first |
| L5 | Fix partly, later | Real spacing only around the camera; needs block-by-block star making and a frame-time check, so best as its own item after L2 to L4 |
| L6 | Fix | Cheap: 1 or 2 systems a cell instead of 6, the home cell kept |
| L7 | Leave | The star is already right; the shells are a fair stand-in for bodies too small to see |

Suggested order if chosen: L3 and L4 first (largest visible gain), then L2
and L6 (cheap), then L5 as its own item.

## 6. Correction: what each level drew

Before #996, the game's content at L3 to L5 was one level behind the
level's real size (the #151 content model, kept through the R8 renumbering).
Compared with what it actually drew:

| Level | Cell size | Drawn as markers | Gap drawn (m) | Real gap for what was drawn (m) | Ratio |
|---|---|---|---|---|---|
| L2 | 1.4 billion ly | superclusters and clusters | 2.0e24 (portals) | 1.4e24 to 1.8e24 | about right |
| L3 | 15 million ly | galaxy clusters and groups | 2.2e22 | 1.1e24 (clusters) | 50 times too close |
| L4 | 100,000 ly | galaxies, the Milky Way among them | 1.3e20 | 5.0e21 (galaxies in Virgo) | 40 times too close |
| L5 | 100 pc | clouds and spiral arms | 5.1e16 | 4.4e18 (clouds) | 86 times too close |
| L6 | 4.4 ly | star systems | 2.3e16 (portals) | 3.3e16 | 1.4 times too close |

Real galaxies cannot sit at real distances inside a box the size of one
galaxy, so spacing alone could not fix L3 to L5. The fix shifted each
level's content to match its size (owner decision D1 A).

## 7. After the build (#996, Spec v6)

Measured by the `SPACING` lines of the headless check (`--verify`), with
§1's method, on the fixed journey at `DEMO_SEED` (plus a rich L3 cell next
to it):

| Level | Draws | Markers | Median gap (m) | Real gap (m) | Within a factor of 2 |
|---|---|---|---|---|---|
| L1 | 8 regions (unchanged) | 8 | 4.4e26 | | unchanged |
| L2 | clusters and groups | 259 | 9.6e23 | 1.06e24 | yes |
| L3, home | galaxies: the Local Group with the Milky Way and Andromeda | 150 | 1.20e22 | 1.18e22 | yes |
| L3, rich | galaxies of a Virgo-like cluster | 2,000 | 4.5e21 | 5.0e21 | yes |
| L4 | the Milky Way's molecular clouds | 8,107 | 3.2e18 | 4.4e18 | yes |
| L5 | stars, the 5,000 nearest the centre | 5,000 | 3.26e16 | 3.3e16 | yes |
| L6 | Alpha Centauri and its companions | 3 | | 1 to 3 markers | yes |
| L7 | the Sun's Oort cloud (unchanged) | 32 | 1.8e15 | | unchanged |

Two real-world numbers came in with the build:
- The home L3 cell, 150 galaxies: the Local Group's 134 known members
  within 1 Mpc (Local Group, Wikipedia, read 2026-11-01) and the near side
  of its neighbouring groups inside a 4.6 Mpc cell. The real gap is the
  random-scatter median for 150 galaxies in that cell.
- Clouds in other galaxies: by disk area against the Milky Way for
  spirals, by the LMC's 272 clouds (Fukui et al. 2008) for irregulars, and
  very few for ellipticals (ATLAS3D, Young et al. 2011: about one in five
  holds molecular gas).

What waited, L1 at its real count and L5 stars made around the camera as
you fly, is §8 (#1104).

## 8. L1 and L5 made near the camera (#1104, Spec v3)

L1 and L5 now hold their real counts. They are too many for any Marker
detail level, so the game makes them in blocks near the camera; each
marker keeps a permanent number and comes back the same. Measured as in §7
over the whole cell (every block, not only those near the camera):

| Level | Draws | Markers | Median gap (m) | Real gap (m) | Within a factor of 2 |
|---|---|---|---|---|---|
| L1 | L2-sized regions inside the observable-universe sphere, every one enterable | 164,968 | 1.09e25 | 1.3e25 (one L2 cell) | yes |
| L5 | stars over the whole 100-pc cell | 139,987 | 3.28e16 | 3.3e16 | yes |
| L5, outer 10 pc | the stars beyond the old 20-pc ball | 68,320 | 3.33e16 | 3.3e16 | yes |

- L1: a 68 x 68 x 68 lattice of L2-sized cells, each region jittered a
  little inside its cell; only cells whose centre lies inside the sphere
  hold one (the owner's D1 A: nothing outside the observable universe).
  The ladder ratio gives 309,030 L2 cells in the cube, about 161,800 in
  the sphere. The eight old regions keep their numbers 0 to 7, so every
  journey is unchanged.
- L5: the 5,000 stars nearest the centre are §7's, bit for bit; outside
  that ball, 10-pc blocks hold 0.14 stars per cubic parsec [R9], so the
  edge of the ball no longer shows.
- The rules are in `docs/universes/ladder.md` (R16) and the memory bound in
  ADR 0012 (#1104 amendment).
