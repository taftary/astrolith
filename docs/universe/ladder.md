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
