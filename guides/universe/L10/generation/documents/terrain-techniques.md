# Terrain techniques — rivers, erosion models and shaped noise

This page goes deeper than the [height-map toolbox](./height-maps.md): it
compares the main ways generators get **river valleys, eroded slopes,
cracks, glacial valleys and natural coastlines**, with what each costs and
whether it fits a height that must stay a pure function of the direction
and the seed. Read [height maps](./height-maps.md) first.

Labels: **fact** (cited), **practice**, **recommendation** (a proposal for
Astrolith, not a decision).

## The core trade-off

| Family | Looks | Cost | Pure function of direction? |
|---|---|---|---|
| Shaped noise (derivative noise, ridges, warping) | eroded-like, no real drainage | cheap, per sample | yes |
| Rivers first (drainage graph, then terrain) | real drainage, controllable | a graph per region | yes, if the graph is generated per region from the seed |
| Analytic stream-power solutions | physical valleys and ridges | moderate | mostly (needs a drainage order) |
| Simulated erosion (droplets, grid water, stream-power iteration) | most realistic | high, iterative | no: needs a grid bake |

**Recommendation:** use shaped noise everywhere, add rivers-first networks
on land regions, and keep simulation as an optional bake. The sections
below explain each.

## Shaped noise

### Derivative (slope-aware) noise

- **Fact:** value noise can return its analytic derivative together with
  its value, faster and more accurately than finite differences; Quilez
  used the derivatives to build the landscape of the *Elevated* demo, and
  analytic normals can be computed for the whole fractal sum [GEN-S039].
- **Practice:** in the fBm loop, sum the derivatives of the octaves so
  far, and divide each new octave's amplitude by `1 + |sum of slopes|²`.
  Steep places then get less fine detail, flat places more, which reads as
  slopes swept smooth and gullies collecting detail, an "eroded" look at
  no extra cost. This is the form commonly implemented from Quilez's demo;
  the exact formula was not confirmed in the article text [GEN-S039].
- **Recommendation:** use it for the hills layer on worlds with weather.
  The analytic normal is a bonus: lighting does not need extra samples.

### Cellular (Worley) noise

- **Fact:** Worley's cellular basis takes the distance to the nearest
  random feature point (`F1`), the second nearest (`F2`), and so on; it
  was used for flagstones, crusty skin, ice, rock, mountain ranges and
  craters [GEN-S040]. `F2 − F1` is near zero along cell edges, which
  outlines the cells.
- **Practice uses:**
  - cracks in ice shells and lava crusts: thin valleys where `F2 − F1` is
    small;
  - mud cracks and polygonal ground at small scale;
  - plate edges ([plates](./plates.md)) are a coarse version of the same
    partition.
- **On a sphere:** scatter the feature points in 3D cells around the unit
  sphere, so the pattern has no seam (as with 3D noise).

## Rivers first

- **Fact:** Génevaux and co-authors build a hierarchical drainage network
  (a geometric graph) over the land first, analyse it into watersheds and
  river types, then build the terrain by blending procedural terrain and
  river patches, all from a simple sketch and a few parameters [GEN-S038].
- **Why it matters:** noise never makes rivers that flow downhill to the
  sea, and erosion simulation is slow. Generating the rivers first and
  shaping the land around them guarantees a consistent drainage.
- **Recommendation for Astrolith (per land region):**
  1. Start river mouths on the coast of each continent (from the
     continent mask), spaced by a seeded draw.
  2. Grow the network inland by branching (each new node higher than its
     parent), stopping at the ridges from the
     [plate belts](./plates.md#generating-plates-recommendation).
  3. Give each node an elevation that rises with its distance upstream,
     steeper near the ridges.
  4. The final height is the larger of the land noise and a valley carved
     under each river edge, its depth and width growing with the drained
     area.
  5. Each region's graph comes from the seed and the region's index, so
     looking up a point only needs the graph of its region: the height
     stays a function of the direction.

## Analytic stream-power terrain

- **Fact:** the stream-power law of geomorphology describes river erosion
  from drained area and slope. Cordonnier and co-authors combine tectonic
  uplift with stream-power erosion on a stream graph to generate large
  terrains with controllable river and ridge structure [GEN-S041]. A 2024
  follow-up uses analytical solutions of the stream-power law to make the
  same physics fast and procedural [GEN-S041].
- **Relevance:** it links [plates](./plates.md) (uplift) and rivers
  (erosion) into one consistent model. Uplift raises the land, rivers cut
  it, and the balance sets the height of ranges.
- **Recommendation:** a later step, once the rivers-first network exists:
  use the stream-power balance to set each river node's elevation from
  the uplift rate at its place.

## Simulated erosion

- **Droplet (particle) erosion (practice):** drop many water particles,
  move each downhill with some inertia, let it pick up sediment up to a
  capacity and drop it when it slows. The particle method goes back to
  Chiba and colleagues (1999); widely used implementations expose an
  inertia (default about 0.1) and a capacity factor (default about 10)
  [GEN-S042]. Lower inertia follows the slope closely; higher inertia
  carves long channels [GEN-S042].
- **Grid water erosion:** the shallow-water grid model of Mei and
  co-authors, with thermal erosion added by later work [GEN-S004],
  [GEN-S005].
- **Recommendation:** only as an optional, deterministic bake per tile at
  load (fixed particle count and order from the seed), stored as a
  correction on the pure height. Never in the per-sample path.

## Glacial valleys and fjords

- **Fact:** a valley's cross-section can be fitted by `y = a x^b`. An
  exponent near 1 means straight V-shaped flanks (river cut); exponents of
  about 1.5 to 2 give the U shape of glacial valleys, with measured values
  mostly between 1.3 and 2.5 [GEN-S043].
- **Recommendation:** on worlds and latitudes that were glaciated (cold
  types, polar regions, high ranges), reshape the river valleys from the
  rivers-first network with a U profile (`b` about 2) and a flat floor;
  where a U valley meets the sea, flood it to make a fjord. Add cirques
  (bowl hollows) at valley heads and small [glacial lakes](./lakes.md).

## Coastlines

- **Fact:** a natural coastline's measured length grows as the ruler
  shrinks; its fractal dimension is about 1.25 for the west coast of
  Britain (reported values 1.24-1.26) [GEN-S044].
- **Practice:** the coast is just the sea-level contour of the height map,
  so its roughness comes from the noise near sea level. If coasts look too
  smooth or too wiggly, change the noise only in a band around sea level.
- **Recommendation:** check generated coasts with a box-counting
  measurement: aim for a dimension of about 1.1-1.3 (smooth to rugged),
  and add warp near sea level until it falls in that range.

## Checks

- **Rivers flow down:** every river edge goes from higher to lower ground,
  and every network ends at the sea or in a [closed lake](./lakes.md).
- **Valleys fit their origin:** glaciated valleys are U-shaped (fitted
  `b` over about 1.5), river valleys V-shaped (`b` near 1).
- **Coast roughness:** box-counting dimension within the target range.
- **Purity:** sampling the same direction twice, at different levels of
  detail, gives the same large-scale height.

## Where to go next

- [Height maps](./height-maps.md) — the base toolbox.
- [Plates](./plates.md) — uplift and ridges that rivers cut.
- [Lakes](./lakes.md) — where rivers end and start.
- [Lifetime](./lifetime.md) — erosion grows with time.

## References

- [Sources](./sources.md): GEN-S004, GEN-S005, GEN-S038 to GEN-S044.
