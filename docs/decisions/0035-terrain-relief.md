# 0035 — Terrain relief (rebuild step 3b)

- Status: accepted
- Date: 2026-10-18
- Issue: #799

## Context

Step 3b of the cube-sphere rebuild (#770 Phase 2, added in #776 Q8 A)
gives rocky planets real relief between the quadtree LOD (#776) and the
cracks/morphing step (#777). Owner answers Q1-Q6 (all A) pick the base
layers from the Height-maps guide (fBm continents and plains,
ridged-multifractal mountain ranges, domain warping, masks; no plates,
erosion, rivers or craters yet), gravity-scaled relief on rocky worlds only
(0.3 and 1 Earth radii; giants stay smooth), unlit colour by height (deep,
low, high, snow), no water yet, the spot rising to terrain plus 2 m, and no
age input.

## Decision

- **Height function (Q1 A, Q2 A).** New core module `relief.rs` (std only,
  `E-CORE-NO-BEVY`): `Relief::for_planet(root, chain)` and
  `height_m(dir)` in metres on the unit direction in `f64`, pure and
  deterministic (`E-DET-TIERS`). Continent mask with sea offset (land
  `+0.5` to `+1` km, ocean floor `-4.3` km), hybrid-style hills
  (`AMP_HILLS_M` 700 m), belt mask times ridged ranges (`AMP_RANGES_M`
  6 km, mountains on land and trenches in oceans so chains form and the
  deepest reaches about 11 km), domain warping by two low-frequency offsets.
  Every amplitude scales by `relief_scale = clamp(9.81 / g, 0.3, 40)` with
  `g` from `gravity::planet_mass_kg`; giants (`radius_earth >= 11`) return 0.
  Transcendental calls stay in core (`E-TRANSCENDENTAL`).
- **Noise.** High hill octaves reach far above 256 per radius, where the
  shared periodic `value_noise_3d` would tile. New
  `noise::value_noise_3d_aperiodic` uses the same `mix_noise` hash on raw
  `i64` indices with no wrap; octaves cap at about a kilometre finest on
  Earth so worker builds stay cheap. A test samples two directions one
  period apart and expects different heights.
- **One source per point.** `build_patch(key, radius_m, relief)` samples
  heights at each point's folded direction (`grid_point`), so every level
  and both sides of a seam agree bit for bit. Positions are
  `(radius + h) x dir`, still relative to the patch centre (Q5 A). Each
  build records `relief_error_m` (largest gap to double-resolution heights)
  and the height range.
- **LOD error with relief (AC5).** New `lod.rs` helpers
  `patch_screen_error_with_relief_px` (`max(quad_sag, relief_error)` over
  the relief-grown nearest distance times focal) and
  `QuadTree::update_with_relief_reused` (sag selection, then one relief
  split pass from the built-patch cache plus neighbour re-enforcement;
  missing entries read as zero, so relief adds at most one level per frame
  behind the builds). Mountains split earlier than plains at the same
  distance; the 1.5 px rule still holds. The bounding radius grows by the
  height range.
- **Tint (Q3 A, Q4 A).** Core `ReliefTint` (Deep, Low, High, Snow) with
  thresholds scaled by `relief_scale` (below 0, 0-1 km, 1-4 km, above 4 km
  at Earth scale); render `tokens.rs` gains `RELIEF_*_TINT` (`E-TOKENS`),
  `patch_bevy_mesh` writes per-vertex colours, and the shared material goes
  white and stays unlit. F3 keeps its colours by level.
- **Spot (Q5 A).** `PlayerSpot::seeded` puts the spot at the relief surface
  plus 2 m along its direction (giants: smooth ball plus 2 m); the body
  still floats until #779.
- **Headless.** New `headless_relief.rs` (split under `E-FILE-SIZE`)
  prints `RELIEF highest=<m> lowest=<m> land=<share> modes=<n>
  gravity=<PASS|FAIL> seams=<n> poles=<PASS|FAIL> PASS`; two runs
  byte-identical; goldens gain the line plus the relief-driven LOD shake
  change (documented in the PR, `E-GOLDEN`). `--screens` `planet-l10.png`
  and `lod-overlay.png` now show the tinted relief.

## Consequences

Extends ADR 0034 (patches gain relief, tint, and relief-aware selection).
`E-STDOUT-PROTOCOL` gains the `RELIEF` line; `E-RENDER-NO-MESH` now reads
one height-tinted patch mesh per drawn patch plus one body per moon (plus
the player figure and billboards). Measured: `--verify` prints `RELIEF
highest=5780.8 lowest=-9490.8 land=0.252 modes=2 gravity=PASS seams=0
poles=PASS PASS` and `VERIFY-OK`; headless LOD counts read 9, 9, 216, 216,
216 at 2,000 km down to 1 m with shake 0.031 mm (cameras ride the relief
surface — the pole sits about 6 km deep, so sphere-relative cameras would
leave the shake check measuring nothing; selection on this line stays
sag-only with sphere-shell distances, and near-field relief refinement
happens in the window); `--screens` runs `SCREENS-OK files=18` on
the dev PC (Intel(R) UHD Graphics 620, Dx12) with tinted continents,
snowy chained ranges, and deep basins, and no seams at cube edges or poles.
Cracks with relief may now show (#777 removes them); water and climate,
light (#780), plates/erosion/rivers/craters/volcanoes, recipes per type and
age, and walking (#779) come later.
