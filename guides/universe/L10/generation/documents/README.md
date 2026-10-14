# L10 generation — making planets and moons automatically

This subtopic explains how to **generate** planets and moons from a seed:
how a world's type is chosen, how its height map (relief) is built, and how
its moons are chosen and shaped, so that every type comes out of the
generator on its own. It is the "how to make one" companion to the
descriptive pages of [L10 — Planets and moons](../../documents/README.md)
and to the [planet and moon kinds catalog](../../../catalogs/documents/planets.md),
which say what real worlds look like.

## Scope

- In scope: the generation pipeline from seed to finished world; type
  classification from size, mass and temperature; height maps on a sphere
  (noise, ridges, craters, volcanoes, erosion, seas, lakes); plate
  movement, rifts and mountain building; one recipe per planet
  type; moon systems (how many, where, how big) and one recipe per moon
  type; the checks that keep generated worlds plausible.
- Out of scope: drawing and shading (see `docs/` and the renderer), star
  and system layout above the planet (see
  [L09 — Stars](../../../L09/documents/README.md)), and the frozen scale
  ladder (`docs/universes/ladder.md`).

## Documents

- [Height maps](./height-maps.md) — the relief toolbox: sampling noise on
  a sphere, fBm, ridged and hybrid multifractals, domain warping, craters,
  volcanoes, erosion, sea level, and how gravity limits relief.
- [Terrain techniques](./terrain-techniques.md) — deeper methods:
  derivative and cellular noise, rivers-first networks, stream-power and
  simulated erosion, glacial valleys, fractal coastlines.
- [Planet types](./planet-types.md) — the classifier (size, mass,
  temperature) and one generation recipe per planet type, from iron worlds
  to gas giants.
- [Climate and look](./climate-and-look.md) — temperature, rain belts,
  biomes, plant colour under other stars, clouds, sky colour.
- [Moons](./moons.md) — moon systems (count, orbits, mass budget, rings)
  and one recipe per moon type, including irregular "potato" moons.
- [Lifetime](./lifetime.md) — age as an input: the brightening star,
  craters piling up and softening, cooling interiors, air kept or lost,
  tidal locking and moon recession, erosion over time.
- [Moon dynamics](./moon-dynamics.md) — how moons form, tidal heating,
  resonances, decay and drift, co-orbital moons, rings that fade.
- [Plates](./plates.md) — how plates move, how rifts and mountains
  form, and how to generate plates, boundaries and their relief.
- [Lakes](./lakes.md) — how lake basins form (tectonic, volcanic,
  glacial, impact and more), lakes on other worlds, and how to fill
  basins from a liquid budget.
- [Sources](./sources.md) — every external reference, `GEN-S001` to
  `GEN-S055`.

## The pipeline at a glance

![Generation pipeline: seed to type, recipe, height map, colour and moons](../images/generation-pipeline.svg)

0. **Age.** The system's age and each region's surface age drive the
   time-dependent layers ([lifetime](./lifetime.md)).
1. **Seed.** Every world gets its own random stream from the system seed
   and its index, so it is the same on every run.
2. **Physical draw.** Mass (or radius), orbit distance and the star give
   radius, surface gravity and equilibrium temperature.
3. **Type.** A classifier turns those numbers into one planet type
   ([planet types](./planet-types.md#the-classifier)).
4. **Recipe.** The type picks a recipe: which height-map layers, their
   strengths, sea level, palette, air.
5. **Height map.** The layers are summed into one height function of the
   surface direction ([height maps](./height-maps.md)), and basins are
   filled with lakes from the liquid budget ([lakes](./lakes.md)).
6. **Moons.** The planet's mass, Hill sphere and Roche limit bound its
   moon system, and each moon runs the same pipeline with a moon recipe
   ([moons](./moons.md)).

## How Astrolith does it today

As read from `crates/universe-core/src/terrain.rs` on 2026-10-14:
- the planet surface is the icosphere split once (80 flat faces) with **no
  relief** since #593;
- the body recipe picks air from radius (airless under 0.5 Earth radii,
  giant from 3 Earth radii);
- the moon count is 0-2 for rocky planets (under 2 Earth radii) and a draw
  up to 28 or 274 for giants;
- `noise.rs` holds a deterministic, periodic value noise shared with L1-L3.

The pages here describe a fuller generator. Where they recommend something
for Astrolith, the text says so; nothing here is a decision.

## Related

- Parent topic: [L10 — Planets and moons](../../documents/README.md).
- Catalog of kinds: [planet and moon kinds](../../../catalogs/documents/planets.md),
  [rocks](../../../catalogs/documents/rocks.md).
- Descriptive pages this builds on: [surface](../../documents/surface.md),
  [tectonics](../../documents/tectonics.md),
  [volcanism](../../documents/volcanism.md),
  [impacts](../../documents/impacts.md),
  [moons](../../documents/moons.md).
- Below the planet: [L11 — Regions of a planet](../../../L11/documents/README.md).

## Documentation status

First version (2026-10-14): pipeline, height-map toolbox, planet recipes,
moon recipes, plates and lakes, with sources. One SVG diagram. Proposed next: worked
parameter tables tuned against real Solar System relief (Moon, Mars,
Earth hypsometry), once the game has relief again.
