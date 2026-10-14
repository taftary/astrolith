# Plates — moving plates, rifts and mountains

This page explains how plate movement builds a planet's large relief
(rifts, ocean ridges, trenches and mountain ranges) and how a generator can
imitate it. The descriptive science of Earth's plates (names, speeds,
earthquakes) lives in [tectonics](../../documents/tectonics.md); this page
reuses its numbers and turns them into generation rules. The result feeds
the mountain-belt and continent masks of the [height map](./height-maps.md).

Labels: **fact** (cited, or from [tectonics](../../documents/tectonics.md)),
**practice**, **recommendation** (a proposal for Astrolith, not a
decision).

## How plates move (facts)

- Earth's outer shell is broken into about seven or eight great plates and
  several medium ones. They move from about 1 cm to over 10 cm per year
  ([tectonics — key numbers](../../documents/tectonics.md#key-numbers)).
- What happens at a boundary depends on the relative motion of the two
  plates:

| Boundary | Motion | Ocean-ocean | Ocean-continent | Continent-continent |
|---|---|---|---|---|
| Divergent | apart | mid-ocean ridge (new crust) | — | rift valley, later a new ocean |
| Convergent | together | trench + volcanic island arc | trench + coastal volcanic range (Andes type) | folded high range and plateau (Himalaya type) |
| Transform | sideways | fracture zone | long strike-slip fault | long strike-slip fault |

## How rifts form

1. A continent stretches above hot mantle. The crust thins and breaks into
   blocks; the middle block drops between two faults (a rift valley).
2. The rift shoulders rise, volcanoes appear along it, and long deep lakes
   fill the floor (see [lakes — tectonic](./lakes.md#how-lake-basins-form)).
3. If stretching continues, the sea floods in and the rift becomes a
   narrow sea, then a widening ocean with a ridge in the middle.

This opening-then-closing story is the **Wilson cycle**: a supercontinent
rifts into a young ocean that widens, then starts to close by subduction,
and ends in a continent-continent collision that can build a new
supercontinent [GEN-S029].

**Ridge depth (fact):** new ocean floor is shallowest at the ridge and
sinks as it cools and ages: `depth ≈ 2500 + 350 √t` metres for floor up to
about 70 million years old (`t` in millions of years), flattening toward
about 6,400 m for older floor [GEN-S030]. Later work finds a slope about
10 percent smaller [GEN-S030].

## How mountains form

- **Ocean-continent convergence:** the denser ocean plate dives
  (subducts). A deep trench forms offshore, and a volcanic range rises on
  the continent's edge, parallel to the trench (the Andes).
- **Ocean-ocean convergence:** one ocean plate dives under the other. A
  trench forms with a curved chain of volcanic islands behind it (Japan,
  the Aleutians).
- **Continent-continent collision:** neither continent sinks, so the crust
  piles up and folds into the highest ranges and a broad high plateau
  behind them (the Himalaya and Tibet).
- **Roots (fact):** high ground floats on a thick crustal root. With
  typical densities, the root is about 5 to 6 times the height of the
  mountains; under the Himalaya and southern Tibet the crust reaches about
  70 km [GEN-S031]. Active collision zones are also held up partly by
  horizontal push, so they are not in full balance [GEN-S031].
- **Hot spots:** volcano chains in the middle of a plate (Hawaii) come from
  a fixed plume under a moving plate. The chain records the plate's
  direction and gets older away from the active end
  ([tectonics](../../documents/tectonics.md)).

## Generating plates (recommendation)

Procedural tectonics in graphics research models subduction, collisions
and rifting procedurally (not as a physics simulation) to produce
continents, ridges, mountain ranges and island arcs, with the user able to
steer the plates [GEN-S028]. A simpler, pure version for Astrolith:

1. **Plates.** Scatter `N` seed points on the sphere from the planet's
   seed (`N` about 6 to 20, more on bigger planets). Each surface direction
   belongs to the nearest seed (a spherical Voronoi partition). Warp the
   direction with low-frequency noise first, so the plate edges are not
   straight arcs.
2. **Plate kind.** Mark each plate continental or oceanic, so that the
   continental share matches the type's land share (about 30 percent for
   Earth-like).
3. **Motion.** Give each plate a rotation about a random axis through the
   planet's centre (an Euler pole) with a speed of 1-10 cm per year. The
   velocity at a point is `ω × p`.
4. **Boundaries.** Near the edge between plates A and B, compare their
   velocities along the edge normal:
   - closing speed above a threshold: convergent;
   - opening speed above a threshold: divergent;
   - otherwise: transform.
5. **Relief by boundary** (amplitudes before the gravity
   `relief_scale` of [height maps](./height-maps.md#how-gravity-limits-relief)):

| Boundary and plates | Relief profile across the boundary |
|---|---|
| divergent, ocean | a ridge about 2.5 km below sea level, the floor deepening away from it by the √age law |
| divergent, continent | a rift: a narrow trough 1-3 km below its raised shoulders, with lakes |
| convergent, ocean under continent | a trench about 6-8 km deep on the ocean side; a volcanic range of 4-6 km about 100-300 km inland |
| convergent, ocean under ocean | a trench and a curved island arc behind it |
| convergent, continent with continent | a wide belt of ridged mountains of 5-9 km, and a plateau of 4-5 km behind it |
| transform | a long straight scarp of a few hundred metres, offset ridges |

6. **Width and strength.** Shape each profile with the distance to the
   boundary. Scale its height by the closing or opening speed, so fast
   collisions make the highest ranges.
7. **Age (optional).** For ocean floor, use the distance from the ridge
   divided by the spreading speed as the age, and apply the √age depth
   law. That gives the right deep-ocean shape for free.
8. **Detail.** Add ridged multifractal noise inside the belt masks and
   [domain warping](./height-maps.md#building-block-4-domain-warping) so
   ranges branch and bend.

All of this is a pure function of the direction and the seed (nearest
seed points, their motions, distances to edges), so it stays compatible
with level of detail and determinism.

## Which planets have plates

| Planet type | Plates? |
|---|---|
| Earth-like | yes: full recipe |
| Super-Earth | likely yes (an open question in research): use the recipe with more, smaller plates |
| Desert (Mars-like, small) | no moving plates now: old rift canyons and giant shield volcanoes over hot spots instead |
| Venus-like (thick air, hot) | no plates: resurfaced by volcanism, with folded highlands |
| Lava world | a thin, constantly breaking crust: many small plates as cracked cells |
| Ice world, icy moon | "ice tectonics": ridges, bands and cracks in the ice shell rather than mountain belts |
| Airless small bodies | no: craters dominate |

This follows the comparison in [tectonics — example objects](../../documents/tectonics.md#example-objects)
(Mars, Venus, Io, Europa) and is a recommendation for the generator.

## Checks for generated plates

- Every direction belongs to exactly one plate; plate edges are continuous.
- Plate speeds stay in 1-10 cm per year.
- Mountains and trenches only appear at convergent edges; rifts and ridges
  only at divergent ones.
- The ocean floor deepens away from every ridge.
- The highest peak stays under the gravity limit.
- The same seed gives the same plates.

## Where to go next

- [Height maps](./height-maps.md) — masks and ridged noise used here.
- [Lakes](./lakes.md) — rift lakes along divergent boundaries.
- [Planet types](./planet-types.md) — which types get plates.
- Science background: [tectonics](../../documents/tectonics.md),
  [volcanism](../../documents/volcanism.md).

## References

- [Sources](./sources.md): GEN-S028 to GEN-S031, plus the USGS sources
  listed in [tectonics](../../documents/tectonics.md#sources).
