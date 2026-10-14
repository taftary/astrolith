# Landforms — volcanoes, rivers, deltas and karst

This page gives shape rules for landforms that sit on top of the large
relief: volcano types, river networks and meanders, deltas at river
mouths, and karst. Each section gives the measured shape (where a source
gives one) and a stamp or rule to generate it. The large relief comes
from [height maps](./height-maps.md), [plates](./plates.md) and
[terrain techniques](./terrain-techniques.md).

Labels: **fact** (cited), **practice**, **recommendation** (a proposal for
Astrolith, not a decision).

## Volcanoes

- **Fact:** the USGS groups volcanoes into cinder cones, composite
  volcanoes (stratovolcanoes), shield volcanoes and lava domes [GEN-S056].
  - **Shield volcanoes** are built of fluid, low-viscosity basalt that
    flows far, so they are very wide with gentle slopes [GEN-S056].
  - **Stratovolcanoes** erupt more viscous andesite and rhyolite that does
    not travel far, so they are steep and tall [GEN-S056].
  - **Cinder cones** are the simplest: loose fragments from one vent,
    rarely more than about 300 m high, with slopes of about 25-35 degrees
    (the angle of repose of loose material) [GEN-S056].
- **Recommendation — profiles:**

| Type | Profile `h(r)` | Size | Where |
|---|---|---|---|
| Shield | `H (1 - r/R)^1.5`, flat summit caldera | `R` tens to hundreds of km (Olympus Mons scale on low-gravity worlds) | hot spots, rifts, lava worlds |
| Stratovolcano | concave `H (1 - r/R)^0.6` near the top, steepening to the summit crater | `R` 10-20 km, `H` 2-4 km on Earth | above subduction zones (volcanic arcs, see [plates](./plates.md#how-mountains-form)) |
| Cinder cone | straight sides at 30 degrees, small crater | `R` under 1 km | clusters on shield flanks and in volcanic fields |
| Lava dome | rounded bulge, steep sides | `R` under 1 km | inside craters of stratovolcanoes |

- **Lava plains:** fill low ground around active volcanoes by raising the
  height to a flat level inside a mask (the same flood rule as
  [lakes](./lakes.md#finding-lakes-in-a-height-map), with lava).
- **Scale with gravity:** volcano height obeys the same `1/g` limit as
  mountains ([height maps](./height-maps.md#how-gravity-limits-relief)).

## River networks

- **Fact (stream order):** in Strahler's scheme, source streams are order
  1; two streams of equal order join into one of the next order; when
  different orders meet, the higher order continues. Horton found that the
  ratio of stream counts between successive orders (the bifurcation ratio)
  stays about constant across a basin [GEN-S057].
- **Fact (Hack's law):** the main stream's length grows with the drained
  area as `L = c A^h`, with `h` about 0.6 (Hack 1956; 0.568 by Gray 1961);
  real networks give 0.6-0.7, a sign of their fractal structure [GEN-S057].
- **Recommendation:** when growing the rivers-first network of
  [terrain techniques](./terrain-techniques.md#rivers-first):
  - keep a constant branching ratio per basin (a seeded value of about 3
    to 5 is common practice; the range was not confirmed by a source);
  - check basin shape with Hack's law: main-stream length growing about
    as `A^0.6` with drained area (fit the constant once on a few
    real-looking basins), so basins are neither round nor stringy;
  - make valley depth and width grow with the stream order.

## Meanders

- **Fact:** meander wavelength is about 7 to 12 times the channel width
  (Leopold and Wolman, 1957); the radius of curvature is about 2 to 3
  channel widths; a straight reach rarely exceeds about 10 channel widths
  [GEN-S058].
- **Recommendation:** on flat, low ground (gentle slope along the river),
  replace the straight river edge with a sine-like path whose wavelength
  is about 10 times the channel width, with a little noise in amplitude
  and phase; leave oxbow [lakes](./lakes.md) where a loop is cut off.
  Steep rivers stay straight or braided.

## Deltas

- **Fact (Galloway):** deltas fall on a triangle between three
  end-members, by which force dominates [GEN-S059]:
  - **river-dominated:** elongated, many branching channels, a
    "bird's-foot" shore (the Mississippi);
  - **wave-dominated:** a smooth, arc-shaped shore with few channels,
    shaped by longshore drift (the Nile, the São Francisco);
  - **tide-dominated:** estuary-like bays with islands lined up with the
    tidal flow (the Ganges-Brahmaputra, the Fly).
- **Recommendation:** at every large river mouth, pick the delta type
  from three seeded or derived weights: river size (sediment), wave
  exposure (open coast against sheltered) and tidal range (large moons and
  wide shelves give bigger tides). Stamp:
  - river-dominated: a fan of 3-7 distributary channels with levee
    ridges;
  - wave-dominated: a smooth arc of low land with one or two mouths;
  - tide-dominated: a funnel-shaped estuary with elongated islands.

## Karst

- **Fact:** karst is a landscape formed by dissolving soluble rock such
  as limestone and dolomite; about 20 percent of the land surface of the
  United States is karst; sinkhole collapse usually takes centuries to
  millennia in carbonate rock [GEN-S060].
- **Recommendation:** in regions marked as carbonate (from the
  [rocks catalog](../../../catalogs/documents/rocks.md) and a seeded
  sedimentary mask), add clusters of round sinkholes (cellular noise
  minima), fewer surface rivers (streams that vanish into the ground),
  and, on wet worlds, tower or cone karst (steep isolated hills) in the
  tropics. Sinkholes can hold small [solution lakes](./lakes.md#how-lake-basins-form).

## Checks

- Stratovolcanoes sit on convergent boundaries; shields on hot spots,
  rifts and lava worlds.
- Cinder cones are small and at about 30 degrees.
- River networks follow Hack's law within the target range.
- Meanders appear only on gentle slopes, with wavelengths near 10 channel
  widths.
- Every large river ends in a delta or an estuary on the coast, or in a
  closed lake.

## Where to go next

- [Terrain techniques](./terrain-techniques.md) — rivers first.
- [Plates](./plates.md) — where volcanoes and ranges come from.
- [Lakes](./lakes.md) — oxbows, sinkholes, crater lakes.
- Background: [volcanism](../../documents/volcanism.md),
  [hydrosphere](../../documents/hydrosphere.md).

## References

- [Sources](./sources.md): GEN-S056 to GEN-S060.
