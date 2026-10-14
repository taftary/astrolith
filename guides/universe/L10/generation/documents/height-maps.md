# Height maps — building relief on a sphere

This guide explains how to build a planet's or moon's relief from a seed:
the building blocks, how they are combined, and the limits that keep the
result believable. The [planet types](./planet-types.md) and
[moons](./moons.md) pages then say which blocks each type uses and how
strongly.

Labels used below: **fact** (from a source, cited), **practice** (common
technique in terrain generators), **recommendation** (a proposal for
Astrolith, not a decision).

## What a height map is here

A planet's height map is one function:

```text
h(direction, seed) -> metres above (or below) the mean radius
```

- `direction` is a unit vector from the planet's centre. The ground point
  is `centre + (R + h) * direction`, where `R` is the mean radius.
- The function is evaluated wherever a vertex is needed, at any level of
  detail. It is not an image stored in advance.
- **Recommendation:** keep `h` a pure function of the direction and the
  seed, computed in `f64` in `universe-core`. Then every level of detail,
  the collision ground and the headless check all agree, and the same
  seed gives the same planet on every machine (the project's determinism
  rule, `E-DET-TIERS`).

## Sampling on a sphere

A flat image wrapped on a sphere pinches at the poles and has a seam. Two
practices avoid it:

1. **3D noise at the direction (practice).** Sample a 3D noise at
   `direction * scale`. A sphere cut through a 3D field has no seam and no
   pole, because the field itself has none. This is the simplest choice and
   suits an icosphere mesh such as Astrolith's (ADR 0024).
2. **Cube sphere with per-face 2D maps (practice).** Project a cube onto
   the sphere and build each face as a 2D grid [GEN-S006]. It suits erosion
   on grids and tiled storage, but needs care at the face edges and corners.

**Recommendation:** use 3D noise at the direction for everything that is
generated in one pass (noise, craters, volcanoes). Keep grid methods
(erosion) for a later, optional bake on cube-sphere tiles.

Precision note: at Earth size a direction must be resolved to about
`1 m / 6.4e6 m ≈ 1.6e-7` rad to place metre-scale detail. `f32` carries
about 7 digits, which is at the edge, so the noise must be sampled in `f64`
(or with a per-region offset) when close to the ground.

## Building block 1: noise

- **Value noise:** random values on a lattice, smoothly interpolated.
  Cheap and deterministic. Astrolith's `noise.rs` is a periodic value
  noise. It shows a grid pattern if used alone at large amplitude.
- **Gradient (Perlin) noise:** random gradients on the lattice. Smoother
  and less grid-like than value noise (practice).
- **Simplex noise:** Perlin's 2001 successor, cheaper in 3D and above, with
  no noticeable directional artifacts [GEN-S001].

**Recommendation:** gradient or simplex noise in 3D for relief. Keep the
existing value noise for coarse fields where grid look does not matter.
Every noise function must be written with integer hashing and plain
float arithmetic, so it gives identical results on every platform (as
`noise.rs` does today).

## Building block 2: fractal sums (fBm)

One noise layer has one feature size. Real relief has features at every
size. Fractional Brownian motion (fBm) sums several "octaves":

```text
fbm(p) = sum over i = 0..n-1 of  amplitude^i * noise(lacunarity^i * p)
```

- **Octaves `n`:** from the largest feature to the smallest needed. At
  planet scale, about `log2(largest / smallest)`; 20 or more octaves span
  continents to metres.
- **Lacunarity:** frequency step per octave, usually about 2.
- **Gain (amplitude per octave):** about 0.5. Lower gives smoother land,
  higher gives rougher land.
- fBm's self-similarity resembles eroded mountains, which is why it is the
  standard landscape base (practice, quoted in [GEN-S002]).
- **Level of detail (practice):** only sum the octaves whose feature size
  is larger than about one pixel at the current distance. Fading the last
  octave in avoids popping.

## Building block 3: multifractals (ridges and plains)

Plain fBm is equally rough everywhere. Real worlds have smooth plains next
to sharp ranges.

- **Ridged multifractal:** take `1 - |noise|`, square it, and feed each
  octave's value into the next octave's weight. The absolute value makes
  "canyons", and flipping them makes sharp crests [GEN-S003]. Use it for
  mountain belts and ice ridges.
- **Hybrid multifractal:** multiply octaves by the height so far, so high
  ground gets rough while low ground stays smooth [GEN-S003]. Use it for
  continents with plains.
- **Offset and gain** set where the fine detail starts and how strong it
  is [GEN-S003].

## Building block 4: domain warping

Warp the input before sampling: `f(p + h(p))` with fBm for both [GEN-S002].
The nested form `fbm(p + fbm(p + fbm(p)))` bends straight ridges into
organic, flowing shapes. This is the cheapest way to make noise relief look
less like noise. Keep the warp strength under about one feature size, or
features smear.

## Building block 5: masks (where each layer applies)

Real relief is regional. Use low-frequency fields as masks:

- **Continents:** a low-frequency fBm thresholded at the sea-level share
  (for example 29 percent land on Earth) gives land and ocean shapes.
- **Mountain belts:** ridged noise is multiplied by a mask that is high
  along continent edges (practice that imitates plate boundaries; see
  [tectonics](../../documents/tectonics.md)).
- **Plates (optional):** a Voronoi partition of the sphere into 6-20
  plates, with relief raised where plates converge and lowered where they
  rift. More work, but it gives believable chains and rifts; see
  [plates](./plates.md).

## Building block 6: craters

Craters dominate airless bodies. Each crater is a radial profile stamped
on the sphere:

- **Shape (fact):** fresh simple lunar craters (under about 15 km) are
  about 0.2 times as deep as they are wide; complex ones are much
  shallower, `d = 1.044 D^0.301` km, with a flat floor, terraces and a
  central peak [GEN-S007].
- **Transition size (fact):** the switch from simple to complex scales
  with `1/g`, `D_tr ≈ 16533.8 g^-1.0303` m [GEN-S008]. The fit gives about
  10 km at the Moon's 1.62 m/s² (Pike puts it at 10.6 to 15 km [GEN-S007])
  and about 1.6 km at Earth's gravity.
- **How many (fact):** a saturated surface has a cumulative count of
  craters larger than `D` close to `D^-2`; new craters then erase old ones
  [GEN-S009].
- **Profile (practice):** a bowl `-d (1 - (r/R)^2)` inside the rim, a
  raised rim of about 0.04 D, and ejecta falling off as `(R/r)^3` outside.
  Add a central peak and a flat floor above the transition size.

Placement **(recommendation):** draw crater centres per size class from
the seed, with counts from the `D^-2` law and an age factor. Look them up
with a spatial hash on the sphere, so each sample only checks nearby
craters. Older craters are drawn first and softened (degraded), younger
ones stamped over them.

## Building block 7: volcanoes and basins

- **Shield volcano:** a broad cone, `h = H (1 - r/R)^k` with `k` from 1
  to 2, a summit caldera, and lava-plain smoothing around it.
- **Basins and maria:** very large impacts flooded with lava. Stamp a wide
  shallow depression, then flatten its floor (set a smoothing mask over
  the noise).

## Building block 8: erosion

Erosion turns noise into landscape: river valleys, alluvial fans, smooth
debris slopes.

- **Hydraulic erosion (fact):** a grid water model moves water and
  sediment, cutting valleys. It can run interactively on a GPU [GEN-S004],
  [GEN-S005].
- **Thermal erosion (fact):** slopes steeper than a talus angle collapse
  [GEN-S005].
- **Cost:** both are iterative grid simulations. They are not pure
  functions of the direction, so they conflict with the "compute anywhere,
  at any detail" rule above.

Deeper methods (rivers-first networks, stream-power terrain, droplet
erosion) are compared in [terrain techniques](./terrain-techniques.md).

**Recommendation:** fake erosion in the pure function first (ridged noise
for drainage-like crests, slope-dependent gain, and a valley term that
lowers ground in noise valleys). If real erosion is wanted later, bake it
once per planet tile at load, deterministically, and store the result as a
correction on top of the pure height.

## Building block 9: sea level and ice

- **Sea level:** pick the height percentile that matches the type's water
  share. The sea surface is the sphere at radius `R + h_sea`. Sea floor
  relief is kept, but drawn under water.
- **Ice caps:** a latitude and height mask, colder toward the poles and
  higher up, scaled by the type's temperature.

## How gravity limits relief

Rock strength caps relief: the tallest mountain scales roughly as `1/g`,
and height times gravity is about constant for Venus, Earth and Mars
[GEN-S010]. Olympus Mons (about 21.9 km at 0.38 g) is about 2.5 times
Everest. It is an upper bound: plate motion, erosion and base width also
limit real mountains [GEN-S010].

**Recommendation:** scale every relief amplitude by
`relief_scale = clamp(g_earth / g, 0.3, 40)`. Earth sets the base: about
9 km highest peak and about 11 km deepest trench. Small bodies below the
potato radius are not spheres at all; see
[moons — round or potato](./moons.md#round-or-potato).

## Putting it together

```text
h(dir) =  continent_mask(dir) * sea_offset
        + hybrid_fbm(warp(dir))                * amp_hills    * relief_scale
        + belt_mask(dir) * ridged(warp(dir))   * amp_ranges   * relief_scale
        + sum craters(dir)                     * crater_age
        + sum volcanoes(dir)
        + fine_fbm(dir, octaves for this LOD)  * amp_detail
```

The recipe for each type sets the amplitudes, masks and counts; see
[planet types](./planet-types.md#recipes-by-type).

## Checks for a generated height map

- **Same seed, same planet:** two runs give identical heights (a test).
- **Level of detail agrees:** the height at a point does not change when
  more octaves are added beyond the pixel size (only finer detail appears).
- **Hypsometry:** the share of surface by height looks like the type's
  target (Earth has two peaks, continents and ocean floor; the Moon has one
  broad one).
- **Relief bound:** the highest peak stays under the gravity limit above.
- **No seams or poles:** sample a ring around each pole and across any face
  edges, and check that the slope stays continuous.

## Where to go next

- [Planet types](./planet-types.md) — which blocks each type uses.
- [Moons](./moons.md) — crater-heavy and irregular bodies.
- [Plates](./plates.md) — rifts, ridges, trenches and mountain belts.
- [Lakes](./lakes.md) — filling the basins.
- Descriptive background: [surface](../../documents/surface.md),
  [impacts](../../documents/impacts.md),
  [volcanism](../../documents/volcanism.md).

## References

- [Sources](./sources.md): GEN-S001 to GEN-S010.
