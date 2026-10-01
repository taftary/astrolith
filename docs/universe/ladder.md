# Universe scale ladder (M1, frozen)

Source: Issue #35 notion, sections 3 (Notation) and 4 (Scale ladder).
Status: frozen for M1. Do not change without owner approval.

Scale range: 10²⁷ m → 10⁰ m (27 orders of magnitude).
Levels: 15 named levels (L1–L15).

## Symbol table

| Symbol | Meaning |
|---|---|
| `ℓ` | Level index, 1 (observable universe) to 15 (room) |
| `S_ℓ` | Characteristic size of level ℓ, in meters |
| `e_ℓ` | Order of magnitude, `e_ℓ = log₁₀(S_ℓ)` |
| `C = (ℓ, i, j, k)` | A cell: level plus integer grid index |
| `σ` | Seed (64-bit integer) |
| `σ₀` | Root seed of the universe |
| `H(·)` | Hash function (e.g. splitmix64 or xxHash) |
| `R(σ)` | Deterministic PRNG initialized from σ |
| `G_ℓ` | Generator function of level ℓ |
| `K_ℓ` | Constraints passed from level ℓ to level ℓ+1 |
| `F_ℓ` | Local coordinate frame of level ℓ |
| `o` | Local offset (float) inside the deepest active cell |
| `h` | Camera distance to the nearest significant surface or object |
| `θ` | Apparent angular size of an object, `θ ≈ S / d` |
| `θ_min` | Threshold above which an object is subdivided |

Core formulas:

- Seed derivation: `σ_child = H(σ_parent, ℓ, i, j, k)`
- Generator: `(contents_C, K_child) = G_ℓ( R(σ_C), K_parent )`
- Subdivide rule: subdivide `C` if `θ(C) > θ_min`
- Camera speed: `v = k_speed · h`

## Scale ladder

| Level | Name | Typical size | Range e | Contents |
|---|---|---|---|---|
| L1 | Observable universe | ~10²⁷ m | 27 | Everything observable (~93 Gly across) |
| L2 | Cosmic web | 10²⁴–10²⁶ m | 24–26 | Filaments, walls, giant voids |
| L3 | Superclusters | 10²⁴–10²⁵ m | 24–25 | e.g. Laniakea (~5×10²⁴ m) |
| L4 | Galaxy clusters and groups | ~10²³ m | 23 | Virgo Cluster, Local Group |
| L5 | Galaxies | 10²⁰–10²¹ m | 20–21 | Milky Way (~10²¹ m) |
| L6 | Galactic structures | 10¹⁷–10¹⁹ m | 17–19 | Spiral arms, molecular clouds, star clusters |
| L7 | Stellar neighborhood | 10¹⁶–10¹⁷ m | 16–17 | Nearby stars (Alpha Centauri ~4×10¹⁶ m) |
| L8 | Outer solar system | 10¹⁵–10¹⁶ m | 15–16 | Oort cloud |
| L9 | Planetary system | 10¹²–10¹³ m | 12–13 | Planet orbits, Kuiper belt, heliopause |
| L10 | Stars | ~10⁹ m | 9 | The Sun (1.4×10⁹ m) |
| L11 | Planets and moons | 10⁶–10⁸ m | 6–8 | Jupiter, Earth (1.3×10⁷ m), the Moon |
| L12 | Regions of a planet | 10⁵–10⁶ m | 5–6 | Continents, countries, mountain ranges |
| L13 | Cities and landscapes | 10³–10⁵ m | 3–5 | Cities, lakes, valleys |
| L14 | Buildings | 10¹–10² m | 1–2 | Streets, houses, stadiums |
| L15 | Room | 10⁰–10¹ m | 0–1 | Furniture, layout, the 1 m end point |

## Gap note

Some gaps between levels are large (10²¹ → 10¹⁹ m, 10⁹ → 10⁷ m).
Between L5 and L10 these gaps are filled by anonymous octree (or grid)
cells as described in notion section 5.4: each node gets its own seed
via `H`, holds at most a manageable number of objects, and appears in
the cell path without appearing in the ladder above.

## R5 amendment (owner-approved 2026-10-01: published-constants recalibration)

The frozen table above is unchanged. This section pins one
characteristic anchor size per rung from published values, so display
sizes are truthful in log space. True linear scale is impossible on one
screen (10²⁷ m → 10⁰ m); display spacing is the affine
`0.22 + 0.055 · e_ℓ`, linear in the true decade `e_ℓ`, so rung gaps on
screen are proportional to the real decade gaps.

| Level | Anchor object | Size (m) | e_ℓ | Source |
|---|---|---|---|---|
| L1 | Observable universe diameter, 93 Gly | 8.80e26 | 26.94 | Standard ΛCDM diameter (NASA/WMAP) |
| L2 | Sloan Great Wall length, 1.37 Gly | 1.30e25 | 25.11 | Gott et al. 2005, SDSS (NASA) |
| L3 | Laniakea diameter, 520 Mly | 4.92e24 | 24.69 | Tully et al., Nature 2014 |
| L4 | Virgo Cluster diameter, 15 Mly | 1.42e23 | 23.15 | Virgo Cluster surveys (NASA) |
| L5 | Milky Way diameter, 100 kly | 9.46e20 | 20.98 | Standard galactic scale |
| L6 | Molecular-cloud complex, 100 pc | 3.09e18 | 18.49 | Representative GMC scale |
| L7 | Alpha Centauri distance, 4.37 ly | 4.13e16 | 16.62 | Stellar parallax |
| L8 | Oort cloud outer edge, 100,000 AU | 1.50e16 | 16.17 | NASA Science, Oort Cloud facts |
| L9 | Heliopause, 120 AU | 1.80e13 | 13.25 | NASA Voyager 1/2 crossings |
| L10 | Sun diameter (IAU nominal radius) | 1.39e9 | 9.14 | IAU 2015 Resolution B3 |
| L11 | Earth equatorial diameter | 1.28e7 | 7.11 | NASA planetary facts |

Exact constants used: 1 ly = 9.4607304725808e15 m, 1 AU = 149597870700 m
(IAU 2012), 1 pc = 648000/π AU. L7/L8 sit close on screen because the
Oort edge really is a third of the way to Alpha Centauri in log terms —
that compression is the science, not a bug. The window title carries the
per-rung order readout (`10^N m`).

## R6 amendment (owner-approved 2026-10-01: marker-tree nesting)

The frozen table is unchanged. R6 replaces the R5 on-screen spacing with
true nesting: **every marker drawn in a level-`ℓ` cell is the level
`ℓ+1` cell**, sized by the real ratio of the anchors above,
`ratio_ℓ = 10^(e_(ℓ+1) − e_ℓ)`. The observer holds a chain of marker
indices from L1 down to the open cell plus a float64 offset in units of
that cell (the notion's `P`, with a marker index as the integer cell
index). The open cell is the render origin, so coordinates stay O(1) at
every depth; the parent's siblings are drawn `1/ratio` cells away and
nothing shallower is drawn.

| Rung | ratio (child / parent) | Markers per cell |
|---|---|---|
| L1 → L2 | 1.48e-2 | 48 |
| L2 → L3 | 3.80e-1 | 5 |
| L3 → L4 | 2.88e-2 | 48 |
| L4 → L5 | 6.76e-3 | 48 |
| L5 → L6 | 3.24e-3 | 32 |
| L6 → L7 | 1.35e-2 | 32 |
| L7 → L8 | 3.55e-1 | 6 |
| L8 → L9 | 1.20e-3 | 32 |
| L9 → L10 | 7.76e-5 | 32 |
| L10 → L11 | 9.33e-3 | 32 |

Marker counts are a consequence of scale: a cell holds at most
`0.3 / ratio³` children before they overlap (floor 4), so the near-equal
rungs L2/L3 and L7/L8 are sparse. The strict chain applies at every rung
(owner decision for the MVP): the L10 star marker opens into L11 planets.

Navigation: hover highlights a marker, click targets it, the wheel (or
ArrowUp/ArrowDown) moves with `v = k·h` toward the target's surface. The
target opens when its angular radius exceeds 0.14 rad (about a third of
the view) and the open cell closes back into its marker, re-targeted,
below 0.10 rad. Spacebar runs the seeded autopilot L1 → L11 (~40 s).
Travel time per rung is `Δe · ln 10 / k`, so the heliopause → Sun leg
(four decades) is the longest; that is the honest version.

Octree cells (section 5.4) remain available in `universe-core::coords`
for splitting a crowded cell; the marker tree is the navigation
structure.
