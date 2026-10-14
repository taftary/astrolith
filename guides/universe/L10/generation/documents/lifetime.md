# Lifetime — generating a world at a given age

A planet is not built once: it changes for billions of years. The same
seed should look different at 100 million years than at 4.5 billion years.
This page treats **age** as a generation input and explains, for each
process, how it changes the world over time and how a generator can apply
it without simulating every year. Earth's own dated story lives in the
descriptive [L10 lifetime](../../documents/lifetime.md); this page
generalises it to any world.

Labels: **fact** (cited), **practice**, **recommendation** (a proposal for
Astrolith, not a decision).

## Age as an input

- **System age `t_sys`:** drawn per star system (the Sun's is about
  4.57 billion years).
- **Surface age `T` per region:** the time since that region was last
  resurfaced (by lava, ice or plates). Old highlands can be 4.4 billion
  years old next to 3.5-billion-year-old lava plains (the lunar maria).
- **Recommendation:** draw a short list of resurfacing events per world
  (big basins, flood lavas, ice resurfacing). Each one gets a time and a
  mask on the sphere. A point's surface age is the time since the youngest
  event that covers it. Every age-dependent layer below then reads `T`
  at that point, so the world stays a pure function of the direction, the
  seed and the age.

## The star brightens

- **Fact:** the young Sun was about 70 percent as bright as today, and its
  output rises by a little under 1 percent per 100 million years
  [GEN-S035]. Estimates of the early value range from 70 to 80 percent
  [GEN-S035].
- **Effect:** a world's equilibrium temperature
  ([planet types — step 2](./planet-types.md#step-2-gravity-and-temperature))
  creeps up with age. Earth-like worlds slowly lose their oceans in the
  far future; see the far-future section of the
  [L10 lifetime](../../documents/lifetime.md#the-far-future-fate--1-billion-and-5-billion-years).
- **Recommendation:** compute the star's luminosity at the world's age
  with the ramp below and feed it into the classifier. The same planet can
  then be an ice world young and an Earth-like world later.

### Brightening by star class

How fast a star brightens depends on its mass:

- **Fact (scaling):** main-sequence luminosity grows roughly as `M^3.5`,
  so the main-sequence lifetime is about `t_MS ≈ 10 billion years × M^-2.5`
  (`M` in Sun masses) [GEN-S063]. The exponent varies with mass (closer to
  `M^3` below about 0.7 Sun masses) [GEN-S063].
- **Fact (red dwarfs):** fully convective red dwarfs burn about 90 percent
  of their hydrogen and live far longer than the formula says; estimates
  reach trillions of years for 0.1-0.16 Sun masses [GEN-S063].
- **Fact (young red dwarfs):** a red dwarf's long pre-main-sequence phase
  is much brighter than its later life. Planets in its later habitable
  zone can spend several hundred million years in a runaway greenhouse
  and lose several Earth oceans of water, if they formed wet; the loss
  scales with the planet's surface gravity [GEN-S064]. The phase lasts
  about 100 million years or more, depending on the model [GEN-S064].

Recommendation, by class (a heuristic tuned so the Sun's row matches the
facts above):

| Class | Mass (Sun = 1) | Main-sequence life | Luminosity over the main sequence | Early phase |
|---|---|---|---|---|
| F | 1.1-1.5 | about 4-8 billion years | `0.7 → 1.3 × L_mid`, fast: worlds warm quickly and the star leaves the main sequence early | short |
| G (Sun) | 0.8-1.1 | about 10 billion years | `L(t) = L_mid × (0.7 + 0.6 t / t_MS)`: 0.7 at birth, about 1.0 at mid-life (the Sun now), 1.3 at the end | short |
| K | 0.5-0.8 | about 17-57 billion years (formula) | the same ramp, but `t / t_MS` stays small at any system age: nearly constant | moderate |
| M | 0.08-0.5 | trillions of years | flat after the early phase | bright for 100 million to about 1 billion years: dry out close-in wet worlds if the age is past it |

- Use `t_MS = 10 × M^-2.5` billion years for F, G and K stars, capped at the
  age of the universe for drawing ages.
- For M dwarfs, add a "dried out" flag to rocky planets inside the later
  habitable zone, with a seeded chance, scaled by gravity as in
  [GEN-S064].
- A system older than its star's `t_MS` has a giant or a white dwarf
  instead: that is outside this page (see
  [L09 — Stars](../../../L09/documents/README.md)).

## Rogue planets

- **Derived:** with no star, a world's only heat is its own. Earth's
  internal heat flow is about 0.09 W/m² (about 47 TW in all, a commonly
  quoted value). Radiating that as a black body gives
  `T = (q / σ)^(1/4) ≈ (0.09 / 5.67e-8)^(1/4) ≈ 35 K`. So an Earth-like
  rogue planet's surface sits near 30-40 K, colder on older, cooler
  worlds.
- **Fact (hidden water):** a rogue planet with a very thick hydrogen
  atmosphere could keep liquid water on its surface by geothermal heat
  alone (Stevenson 1999); without one, an Earth-composition rogue planet
  about 3.5 times Earth's mass could keep an ocean under about 8 km of ice
  [GEN-S065].
- **Recommendation:** rogue planets get `T_eq` from the derived value
  above (scaled by the [interior activity](#the-interior-cools) at their
  age), ice-world or dwarf recipes, no biomes, and a rare "subglacial
  ocean" flag for massive ones, or a "hydrogen blanket" flag (liquid
  surface water) for very rare cases.

## Craters pile up

- **Fact (lunar chronology):** the cumulative density of craters of 1 km
  and larger on a surface of age `T` (billions of years) is

  ```text
  N(1 km) = a (exp(λ T) - 1) + b T     per km²
  a = 5.44e-14, b = 8.38e-4, λ = 6.93    (Neukum et al. 2001)
  ```

  [GEN-S032]. The exponential term is the heavy early bombardment, and the
  linear term is the steady rate since. The curve is not constrained
  before about 4.1 billion years, and there are almost no dated surfaces
  between 1 and 3 billion years [GEN-S032].
- **Worked numbers:** `T = 3.8` gives about 0.018 craters of 1 km or more
  per km²; `T = 1.0` gives about 0.00084, about twenty times fewer.
- **Recommendation:** use this curve, scaled by an impact-rate factor for
  other systems, to set the crater counts per size class in the
  [crater block](./height-maps.md#building-block-6-craters), from the
  surface age `T` at each point. Cap the counts at saturation (the
  `D^-2` law).

## Craters soften

- **Fact:** on the Moon, crater shapes relax by slow topographic diffusion,
  with a diffusivity of about 5 m² per million years at kilometre scale
  over the last 3 billion years; a 1 km crater loses half its depth in
  about 3 billion years [GEN-S036].
- **Practice:** diffusion of a profile is a Gaussian blur of width
  `σ = sqrt(2 κ t)`. With `κ = 5 m²/Myr` and `t = 3,000` million years,
  `σ ≈ 170 m`, enough to round a 1 km crater's rim and fill its floor.
- **Recommendation:** give each crater stamp its own age. Draw it with a
  profile blurred by `σ` from that age, so old craters look soft and young
  ones sharp, without running any simulation. Worlds with air and water
  use a much larger `κ` (Earth's is far higher than the Moon's
  [GEN-S036]), so their old craters vanish.

## The interior cools

- **Fact:** small planets lose their internal heat faster. Mars's core
  dynamo was short-lived (about 0.5-0.8 billion years) and stopped about
  4 billion years ago [GEN-S037].
- **Effect:** volcanism, plate motion and the magnetic field fade with
  age, sooner on small worlds. Without a field, the air is exposed to the
  stellar wind; see [magnetosphere](../../documents/magnetosphere.md).
- **Recommendation (heuristic):** an activity factor
  `activity = exp(-t / τ)`, with `τ` growing with radius (for example
  about 1 billion years per Mars radius). Scale the number of young
  volcanoes, young lava plains and [plate](./plates.md) relief by it. Tidal
  heating ([moons](./moons.md#moon-types-and-their-recipes)) adds activity
  back to some moons, whatever their size.

## Air is kept or lost

- **Fact (cosmic shoreline):** plotted by escape velocity and the light
  they receive, bodies with and without atmospheres split along a line
  where insolation `I ∝ v_esc^4` [GEN-S033]. Bodies far from the star or
  with high escape velocity keep their air; small, hot ones lose it.
- **Recommendation:** replace the size-only air rule
  ([planet types — shared look rules](./planet-types.md#shared-look-rules))
  with the shoreline test, integrated over age: a world that sat on the
  airless side for most of its life keeps no air. Combine it with the
  dynamo fade above for an extra loss on small, old worlds.

## Spins slow and moons move

- **Tidal locking (fact):** a body's spin is braked by tides from its
  partner. The time to lock scales as
  `t_lock ≈ ω a^6 I Q / (3 G M^2 k2 R^5)`, so it is extremely sensitive to
  the distance `a` (twice as far takes 64 times longer). Real values are
  uncertain by factors of ten or more [GEN-S034].
- **Simplified form (fact):** for a spherical moon with `Q = 100` and an
  initial spin of one turn per 12 hours,
  `t_lock ≈ 6 × a^6 R μ / (m_s m_p^2) × 10^10 years` (SI units), with a
  rigidity `μ` of about `3 × 10^10` N/m² for rock and `4 × 10^9` N/m² for
  ice [GEN-S034]. The full formula is equation 9 of Gladman et al. (1996),
  after Peale (1977) [GEN-S034].
- **Effect:** close-in moons lock first (most large moons face their
  planet). Planets very close to small stars lock to their star, with a
  permanent day side.
- **Moon recession:** tides push a moon outward while the planet's day
  lengthens. Earth's Moon drifts about 4 cm farther each year
  ([L10 README](../../documents/README.md#figures-at-a-glance)).
- **Recommendation:** at generation, compute `t_lock` for each moon and
  for the planet against its star. If `t_lock < age`, set the spin to the
  orbital period (locked). For big moons of rocky planets, start the moon
  closer and the planet's day shorter in the past, and interpolate to the
  drawn age.

## Weathering and erosion grow with time

- On worlds with air and liquid, erosion keeps working: mountains wear
  down unless plates rebuild them.
- **Recommendation:** scale the faked erosion and valley terms of
  [height maps](./height-maps.md#building-block-8-erosion) by the time
  since uplift, and lower the old mountain belts (old, worn ranges like
  the Appalachians are low and rounded; young ones like the Himalaya are
  high and sharp). Use the [plate](./plates.md) boundary age for that.

## A generation order that follows time

Recommendation:

1. Star and age: luminosity history.
2. Planet formation: mass, radius, composition, the type at birth.
3. Resurfacing events with their times and masks.
4. Craters per surface age; each crater softened by its own age.
5. Interior activity at the present age: volcanoes, plates.
6. Air: the shoreline test over the whole history.
7. Water: present only if the air kept it, and the temperature allows it;
   [lakes](./lakes.md) and seas follow.
8. Spin and moons: locking and recession at the present age.
9. Erosion scaled by time since uplift.

Each step reads the previous ones, so the same seed and the same age give
the same world, and a different age gives the same world at another time
of its life.

## Checks

- **Same world, other age:** the same seed at two ages keeps its large
  shapes (basins, old belts), while crater counts, sharpness, air and water
  change in the right direction.
- **Old surfaces are cratered:** crater density on old highlands is higher
  than on young plains.
- **No air on small hot worlds:** every world on the airless side of the
  shoreline has no air.
- **Locking:** close-in moons with `t_lock` under the age are locked.

## Where to go next

- [Height maps](./height-maps.md), [plates](./plates.md),
  [moons](./moons.md), [lakes](./lakes.md).
- Earth's dated history: [L10 lifetime](../../documents/lifetime.md).

## References

- [Sources](./sources.md): GEN-S032 to GEN-S037.
