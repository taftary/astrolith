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
- **Fact (quantitative, Nienhuis et al. 2020):** the delta type follows
  from three sediment fluxes at the river mouth: the river's supply
  `Q_river`, the waves' capacity to move sediment away `Q_wave`, and a
  tidal sediment flux `Q_tide` (all in kg/s) [GEN-S071]:
  - the fluvial dominance ratio `R = Q_river / Q_wave`: `R ≈ 0` leaves the
    coast undeflected; `0 < R < 1` gives a roughly triangular,
    wave-dominated delta (shore angle 0-45 degrees); `R > 1` is
    river-dominated;
  - the tidal dominance ratio `T = Q_tide / Q_river`: `T < 1` keeps
    channels of near-constant width (river-dominated); `T > 1` gives
    channels that widen seaward with reversing flow (tide-dominated);
  - across about 11,000 deltas, about 79 percent are wave-dominated and
    about 10 percent each tide- and river-dominated, while most large
    deltas are tide- or river-dominated [GEN-S071].
- **Recommendation (proxies for a generator):**
  - `Q_river` from the drained area of the [rivers-first network](./terrain-techniques.md#rivers-first),
    the rain of its basin ([climate and look](./climate-and-look.md)) and
    the relief (steep, wet basins carry more sediment);
  - `Q_wave` from the coast's wave exposure: open ocean against a sheltered
    sea, and the storminess of its latitude belt;
  - `Q_tide` from the tidal range (bigger, closer moons and a nearby star
    raise it; see [moon dynamics](./moon-dynamics.md)) times the mouth's
    width;
  - classify: tide-dominated if `T > 1`; otherwise river-dominated if
    `R > 1`; otherwise wave-dominated (or no delta if `R` is near 0).
  The proxies are **estimates**; the ratios and their thresholds are the
  paper's.
- **Stamps:**
  - river-dominated: a fan of 3-7 distributary channels with levee
    ridges;
  - wave-dominated: a smooth triangle or arc of low land with one or two
    mouths, the shore angle growing with `R`;
  - tide-dominated: a funnel-shaped estuary with elongated islands along
    the flow.

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

## Crater rays, secondary craters and chains

- **Fact:** secondary craters, made by blocks thrown out of a fresh
  impact, line up along its bright rays, which can reach many crater
  diameters; secondaries are often irregular, shallower than primaries,
  and form chains radiating from the primary [GEN-S072].
- **Fact:** ray length follows a power law in the primary crater's
  radius [GEN-S072].
- **Fact (fading):** fresh ejecta is bright because it is unweathered;
  solar wind, cosmic rays and micrometeorites darken it, thin rays first.
  Rays fade over roughly 0.75 to 1.1 billion years on the Moon, though
  compositional rays can last longer [GEN-S072].
- **Recommendation:**
  - for each young crater ([lifetime — craters pile up](./lifetime.md#craters-pile-up)),
    draw 6-20 rays as narrow bright wedges, length a seeded multiple of the
    radius (several to tens of radii), brightness fading with the crater's
    age over about 1 billion years;
  - along each ray, place a few small, shallow, elongated secondary pits
    in a chain;
  - this is a colour layer more than a height layer: it reads from orbit
    on every airless body.

## Dunes

- **Fact:** dune shape follows the wind regime and the sand supply:
  crescent-shaped barchans under one dominant wind with limited sand (the
  most common dune form on Earth and on Mars); linear (seif) dunes under
  two alternating wind directions; star dunes under winds from many
  directions with plenty of sand; parabolic dunes where plants anchor the
  arms [GEN-S073]. Names for transverse dunes differ between sources
  [GEN-S073].
- **Recommendation:** in dry basins and deserts
  ([climate and look](./climate-and-look.md)), add a dune field as a
  directional ridged noise:
  - barchan: sparse crescents (a ridged cell pattern) facing downwind;
  - linear: long ridges along the mean wind, from 1D noise across it;
  - star: radial ridges from scattered centres (cellular noise);
  - spacing about 0.5-2 km for large dunes (an **estimate**; no spacing
    law was confirmed), height a few tens of metres.

## Glacial deposits: moraines and eskers

- **Practice:** a glacier pushes and drops rock along its edges and at its
  snout: lateral moraines (ridges along a valley's sides), end moraines
  (arcs across the valley where the ice stopped, often damming a
  [lake](./lakes.md#how-lake-basins-form)), and eskers (long winding
  gravel ridges left by rivers under the ice).
- **Recommendation:** in once-glaciated zones, along the U valleys of
  [terrain techniques](./terrain-techniques.md#glacial-valleys-and-fjords),
  add low ridges (tens of metres) at the valley sides and one or two arcs
  across the valley mouth; scatter a few winding eskers on lowland plains.

## Supervolcanoes

- **Fact:** the USGS defines a supervolcano as a volcanic centre with an
  eruption of VEI 8, meaning deposits over 1,000 km³; the Toba eruption
  (about 75,000 years ago) produced about 2,800 km³ dense-rock equivalent,
  and its caldera now holds the world's largest volcanic lake [GEN-S074].
- **Recommendation:** a rare stamp: a wide caldera (tens of km across),
  shallow and flat-floored, often
  filled with a lake, surrounded by a broad low plateau of ash. At hot
  spots (Yellowstone type) or above subduction zones (Toba type)
  [GEN-S074].

## Out of scope: lava tubes and caves

Lava tubes, caves and other underground voids are below what L10 shows;
they belong to the ground levels (L12-L14). They are listed here only so
the generator reserves a flag (for example "has lava tubes" on volcanic
plains) for later levels.

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

- [Sources](./sources.md) and [research-pass sources](./sources-passes.md): GEN-S056 to GEN-S060, GEN-S071 to GEN-S074.
