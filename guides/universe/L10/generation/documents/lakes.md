# Lakes — how they form and how to generate them

A lake is liquid standing in a closed low spot of the ground. To generate
lakes believably, a generator needs two things: a **basin** (a depression
in the [height map](./height-maps.md)) and a **liquid budget** (enough
inflow to fill it against evaporation or seepage). This page lists how
real basins form, then turns each kind into a generation rule.

Labels: **fact** (cited), **practice**, **recommendation** (a proposal for
Astrolith, not a decision).

## How lake basins form

Limnology classifies lakes by origin. Hutchinson's 1957 treatise names 76
subtypes, which later summaries group into about 11 major kinds
[GEN-S023]:

| Kind | How the basin forms | Typical look | Generation rule (recommendation) |
|---|---|---|---|
| Tectonic | the crust pulls apart (rift) or sinks along faults | long, narrow, very deep lakes in a chain (rift valleys) | along mountain-belt or plate masks: a long trough of negative ridged noise, deepest in the middle |
| Volcanic | a crater or collapsed caldera fills; or a lava flow dams a valley | round, steep-sided, deep; or lobed behind a lava front | stamp a caldera (a flat-floored pit) at volcano summits; fill it if wet |
| Glacial | ice sheets scour rock hollows, leave moraine dams, or melt buried ice blocks (kettles) | thousands of small lakes on old ice-sheet ground; long finger lakes in valleys; round kettles | in high-latitude, once-glaciated zones: many small depressions from high-frequency noise below a threshold; elongated along the old ice-flow direction |
| Fluvial | a river cuts off a meander (oxbow), or builds levees and deltas | crescent lakes beside rivers | only with a river system; skip in a pure height function |
| Landslide | a slope collapses and dams a valley | a lake behind a fresh debris dam in mountains | rare: in steep valleys, add a small dam bump across the valley floor |
| Solution (karst) | water dissolves limestone, gypsum or salt, and the ground sinks | round sinkhole lakes in clusters | in sedimentary zones: clusters of small round pits |
| Aeolian | wind scours hollows between dunes | shallow lakes between dunes, often seasonal | in dune fields: hollows from the dune noise, filled when wet |
| Shoreline | a sandbar cuts off a bay (lagoon) | lagoons along coasts | at sea level along coasts: a thin barrier ridge |
| Impact (meteorite) | an impact crater holds water | very round lakes, often with a central island (peak) | crater stamps from the [crater block](./height-maps.md#building-block-6-craters), filled where the planet is wet |
| Organic | beaver dams, plant mats, peat | small, local | not needed at planet scale |
| Anthropogenic | reservoirs behind dams | lakes with one straight edge | only for inhabited worlds |

**Open and closed lakes (fact, from Mars):** a lake whose inflow fills it
to its lowest rim **overflows** and cuts an outlet valley (an open basin);
a lake whose inflow is too small stays below the rim, with no outlet (a
closed basin). On Mars, most valley-fed crater lakes were open, with
exceptions such as Gale crater [GEN-S026]. Jezero crater, about 45 km
across, had two inlet valleys and one outlet valley, Pliva Vallis
[GEN-S027]. On Earth, closed lakes become salty (the Dead Sea, the Great
Salt Lake), because water leaves only by evaporation.

## Lakes on other worlds

- **Titan (fact):** liquid methane and ethane lakes and seas cluster near
  the poles, mostly in the north (Kraken Mare, the largest, and Ligeia
  Mare, about 126,000 km²); hundreds of small lakes dot the north polar
  region, and the south has only a few lakes such as Ontario Lacus
  [GEN-S025].
- **Mars (fact):** dry crater lakes with deltas and outlet valleys record
  an ancient wet period [GEN-S026], [GEN-S027].
- **Lava lakes (practice):** on lava worlds and volcanic moons, molten
  rock pools in calderas and low plains; generate them like water lakes,
  but with a glowing palette.

## Finding lakes in a height map

A basin is a closed depression: a low area whose lowest rim point (the
**spill point**) is higher than its floor. Filled to the spill point, it
is a full lake.

- **Practice:** the Priority-Flood algorithm floods a grid of heights
  inward from its edges with a priority queue, fills every depression to
  its spill height, and labels watersheds. It runs in `O(n)` for integer
  heights and `O(n log n)` for floating-point ones [GEN-S024].
- **The catch:** Priority-Flood needs a grid. That breaks the "height is a
  pure function of the direction" rule from
  [height maps](./height-maps.md#what-a-height-map-is-here).

**Recommendation for Astrolith, in two tiers:**

1. **Pure lakes (no grid):** lakes in basins whose shape the generator
   already knows: crater lakes, caldera lakes, rift troughs, kettles and
   sinkholes. Each stamp knows its own floor and rim, so its water level
   is a pure function too: `level = floor + fill * (rim - floor)`, with
   `fill` from the liquid budget. These cover most of what a player sees
   from orbit.
2. **Baked lakes (grid, optional):** for depressions that come out of the
   noise itself, run Priority-Flood once per terrain tile at load,
   deterministically, and store each lake's polygon and level. Do it on a
   cube-sphere tile grid ([sampling on a sphere](./height-maps.md#sampling-on-a-sphere)).

## The liquid budget

How full each basin gets (recommendation):

```text
fill = clamp(inflow / (evaporation * area), 0, 1)
  inflow      ~ catchment area * precipitation (wetter types, wetter zones)
  evaporation ~ rises with temperature, falls with humidity
```

- `fill = 1`: an open lake with an outlet notch at the spill point (a
  river leaves it).
- `0 < fill < 1`: a closed lake below the rim; salt-crust colour on the
  dry ring between the water and the rim.
- `fill = 0`: a dry lake bed (playa), flat and bright.

By planet type ([planet types](./planet-types.md)):

| Type | Lakes |
|---|---|
| Earth-like | many: glacial lakes at high latitudes, rift lakes along belts, crater and caldera lakes, salt lakes in dry zones |
| Desert | mostly dry beds and a few salty closed lakes |
| Ocean | none above sea level (or on islands only) |
| Ice | frozen lakes (smooth flat ice), possibly liquid under the ice |
| Lava, volcanic moon | lava lakes in calderas and low plains |
| Hazy moon (Titan-like) | methane lakes, many near the poles, more in one hemisphere |
| Dry rocky, Mars-like | dry crater lakes with deltas and outlet valleys (fossil lakes) |
| Airless | none (no liquid can stay without air pressure) |

## Checks for generated lakes

- Every lake's surface is flat and lies at or below its basin's spill
  point.
- A full lake has an outlet at its spill point; a closed one has none.
- No lakes on airless bodies.
- Lake counts and kinds match the planet type table above.
- The same seed gives the same lakes.

## Where to go next

- [Height maps](./height-maps.md) — the depressions lakes sit in.
- [Planet types](./planet-types.md) — wet and dry types.
- Descriptive background: [hydrosphere](../../documents/hydrosphere.md).

## References

- [Sources](./sources.md): GEN-S023 to GEN-S027.
