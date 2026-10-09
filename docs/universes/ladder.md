# Universe scale ladder (M1, frozen; merged L2 by #151)

Source: Issue #35 notion, sections 3 (Notation) and 4 (Scale ladder).
Status: frozen for M1. Do not change without owner approval.
Change record: R8 amendment (owner-approved 2026-10-04, #151) merges old
L3 into L2 and renumbers below; the table below is the approved 10+4 form.
R9 amendment (owner-approved 2026-10-09, #375) moves the terminal rung
from L10 to L14; see the R9 section at the end.

Scale range: 10^26.94 m → 10^0.5 m.
Levels: 14 named levels (L1–L14).

## Symbol table

| Symbol | Meaning |
|---|---|
| `ℓ` | Level index, 1 (observable universe) to 14 (room) |
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
| L2 | Cosmic web | 10²⁴–10²⁶ m | 24–26 | Filaments, walls, giant voids, superclusters (Laniakea lives here) |
| L3 | Galaxy clusters and groups | ~10²³ m | 23 | Cluster and group portals plus field populations |
| L4 | Galaxies | 10²⁰–10²¹ m | 20–21 | Members of one cluster or group (thousands for Virgo) |
| L5 | Galactic structures | 10¹⁷–10¹⁹ m | 17–19 | Spiral arms, molecular clouds, star clusters of one galaxy |
| L6 | Stellar neighborhood | 10¹⁶–10¹⁷ m | 16–17 | Systems around one cloud (usually one) |
| L7 | Outer solar system | 10¹⁵–10¹⁶ m | 15–16 | One star plus Oort cloud shells |
| L8 | Planetary system | 10¹²–10¹³ m | 12–13 | One star, its planets, belt populations |
| L9 | Stars | ~10⁹ m | 9 | Star close-up: companions plus the same planets |
| L10 | Planets and moons | 10⁶–10⁸ m | 6–8 | One planet, its moons |
| L11 | Regions of a planet | 10⁵–10⁶ m | 5–6 | Continents, countries, mountain ranges |
| L12 | Cities and landscapes | 10³–10⁵ m | 3–5 | Cities, lakes, valleys |
| L13 | Buildings | 10¹–10² m | 1–2 | Streets, houses, stadiums |
| L14 | Room | 10⁰–10¹ m | 0–1 | Furniture, layout, the 1 m end point (terminal) |

## Gap note

Spans over 1.5 decades are crossed through invisible magnification
milestones (#151): L1-L2, L2-L3, L3-L4, L4-L5 and L5-L6 take two each,
L7-L8 two, L8-L9 three, L9-L10 two; short spans (L6-L7) take none. Each
milestone is a silent exact no-op on the path (zero position, unit ratio),
so generation, snapshots, labels, and previews never see them; the dive
pushes them by proximity to a targeted portal and unwinds them the same
way. Groups sit in L3 cells beside clusters (never in containers).

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
| L3 | Virgo Cluster diameter, 15 Mly | 1.42e23 | 23.15 | Virgo Cluster surveys (NASA) |
| L4 | Milky Way diameter, 100 kly | 9.46e20 | 20.98 | Standard galactic scale |
| L5 | Molecular-cloud complex, 100 pc | 3.09e18 | 18.49 | Representative GMC scale |
| L6 | Alpha Centauri distance, 4.37 ly | 4.13e16 | 16.62 | Stellar parallax |
| L7 | Oort cloud outer edge, 100,000 AU | 1.50e16 | 16.17 | NASA Science, Oort Cloud facts |
| L8 | Heliopause, 120 AU | 1.80e13 | 13.25 | NASA Voyager 1/2 crossings |
| L9 | Sun diameter (IAU nominal radius) | 1.39e9 | 9.14 | IAU 2015 Resolution B3 |
| L10 | Earth equatorial diameter | 1.28e7 | 7.11 | NASA planetary facts |

Exact constants used: 1 ly = 9.4607304725808e15 m, 1 AU = 149597870700 m
(IAU 2012), 1 pc = 648000/π AU. L7/L8 sit close on screen because the
Oort edge really is a third of the way to Alpha Centauri in log terms —
that compression is the science, not a bug. The window title carries the
per-rung order readout (`10^N m`).

## R6 amendment (owner-approved 2026-10-01: marker-tree nesting)

The frozen table is unchanged. R6 replaces the R5 on-screen spacing with
true nesting: **every portal marker drawn in a level-`ℓ` cell is the level
`ℓ+1` cell** (ADR 0010 exempts population points, which never open), sized by the real ratio of the anchors above,
`ratio_ℓ = 10^(e_(ℓ+1) − e_ℓ)`. The observer holds a chain of marker
indices from L1 down to the open cell plus a float64 offset in units of
that cell (the notion's `P`, with a marker index as the integer cell
index). The open cell is the render origin, so coordinates stay O(1) at
every depth; the parent's siblings are drawn `1/ratio` cells away and
nothing shallower is drawn.

| Rung | ratio (child / parent) | Markers per cell |
|---|---|---|
| L1 → L2 | 1.48e-2 | 8 octant portals |
| L2 → L3 | 1.10e-2 | 32 (supercluster portals plus field populations) |
| L3 → L4 | 6.76e-3 | 32 (cluster and group portals plus field) |
| L4 → L5 | 3.24e-3 | 32 poor clusters, 2000 rich (Virgo) |
| L5 → L6 | 1.35e-2 | 32 (cloud populations plus largest-cloud portals) |
| L6 → L7 | 3.55e-1 | 6 (packing cap; usually one system portal) |
| L7 → L8 | 1.20e-3 | 32 (one star plus shell populations) |
| L8 → L9 | 7.76e-5 | 32 (one star, its planets, belt populations) |
| L9 → L10 | 9.33e-3 | ~12 (planets plus rare companions) |

Marker counts are a consequence of scale: a cell holds at most
`0.3 / ratio³` children before they overlap (floor 4), so the near-equal
rungs L6/L7 are sparse. Portals open the next cell; populations are shown
and counted but never open (ADR 0010, amending the strict chain below for
markers that are not continuations): one star per system with its planets
around it, one cloud per star, groups beside clusters in L3 cells. Laniakea
lives inside L2 as the home supercluster; the L2 anchor stays the Sloan
Great Wall.

Navigation: hover highlights a portal marker, click targets it (population
points never highlight or target), the wheel (or ArrowUp/ArrowDown) moves
with `v = k·h` toward the target's surface. The target opens when its
angular radius exceeds 0.14 rad (about a third of the view) and the open
cell closes back into its marker, re-targeted, below 0.10 rad. Long spans
are crossed through invisible magnification milestones (gap note above).
Spacebar runs the seeded autopilot L1 → L10 (~40 s). Travel time per rung
is `Δe · ln 10 / k`, so the heliopause → Sun leg (four decades) is the
longest; that is the honest version.

Octree cells (section 5.4) remain available in `universe-core::coords`
for splitting a crowded cell; the marker tree is the navigation
structure.

## R7 amendment (owner-approved 2026-10-01, #63: pre-entry preview)

Entering or leaving a dimension changes nothing on screen. Every marker
whose angular radius exceeds `PREVIEW_ANGLE = 0.02` rad draws its interior
inside itself at the exact positions the open cell will show,
`marker + child × ratio` (the inverse of the frame's open map), at true
size with the pixel clamp; at most `PREVIEW_CAP = 6` markers (largest on
screen) preview at once, so `2 + 6` generations are alive at most. The
marker's dot fades from full brightness at 0.02 rad to a faint floor
(0.15) at the open angle 0.14 rad while its children brighten from 0 to
1 over the same range; past the open angle the dot eases to zero as the
camera passes inside. Both curves are continuous, so the open and close
events (section 5.5 impostor ↔ detail) are pure origin shifts. The open
cell's own boundary sphere follows the same curve in the parent's era
color, so the marker you entered and the cell you are in are one object.
`--verify` prints a `PREVIEW` line per opening (previewed, cap, position
error, alive, plus the `anon` milestone depth) plus a brightness-continuity summary.

## R8 amendment (owner-approved 2026-10-04, #151: scale semantics)

The table above is the approved 10+4 form: old L3 (Superclusters) folded
into L2 as content (supercluster portals plus field populations), old
L4–L11 shifted down one number, Earth at L10, beyond-MVP tail L11–L14.
The R5 anchors keep their values at the shifted rows; Laniakea retires as
a rung anchor and lives inside L2. The R6 strict chain is amended by
ADR 0010 (portal vs population); ratios recomputed above. The autopilot
journey is L1–L10 through the Local Group with Virgo as the rich sibling.
Proposed follow-ups live in `../../guides/universe/documents/realism-review.md`.

## R9 amendment (owner-approved 2026-10-09, #375: bare planet, ladder to the room)

The frozen table above is unchanged except the terminal note, which moves
from L10 to L14. Planet cells carry region portals that open into L11;
L11–L13 are sparse deterministic scatters; L14 Room holds furniture
outlines and never opens. The autopilot journey is L1–L14; the snapshot
surface token drops the climate fields (`T height flattening tilt spin`,
ADR 0016). The historical R6 journey sentence above stays as the record
of its time.
