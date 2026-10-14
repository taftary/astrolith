# Climate and look — from heights to colours

The [height map](./height-maps.md) says where the ground is; climate says
what covers it. This page explains how a generator can derive temperature
and rain from latitude, height and the planet's spin, turn them into
biomes and colours, add clouds, and tint everything by the star's light
and the air. It applies to [planet types](./planet-types.md) with air and
water; airless bodies take their colour from rock and ice only.

Labels: **fact** (cited), **practice**, **recommendation** (a proposal for
Astrolith, not a decision).

## Step 1: temperature

- **Base:** the planet's equilibrium temperature
  ([planet types — step 2](./planet-types.md#step-2-gravity-and-temperature))
  plus the greenhouse offset of its air
  ([air model — greenhouse warming](./air-model.md#step-3-greenhouse-warming));
  Earth is the reference row, about +33 K.

### The energy-balance model (by latitude)

- **Fact:** the one-dimensional energy-balance model (Budyko-Sellers,
  with North's diffusive heat transport) balances, at each latitude `φ`:

  ```text
  C ∂T/∂t = (1 − α) Q − (A + B T) + (D / cos φ) ∂/∂φ (cos φ ∂T/∂φ)
  ```

  with absorbed sunlight `(1 − α) Q`, outgoing heat `A + B T`, and
  poleward heat transport as diffusion with diffusivity `D`. Fitted to
  Earth: `A = 210 W/m²`, `B = 2 W/m²/°C` (`T` in °C), albedo
  `α ≈ 0.354 + 0.25 P2(sin φ)` with `P2(x) = (3x² − 1)/2`, and `D` tuned
  to about `0.55 W/m²/°C` [GEN-S067].
- **Practice:** in steady state with `Q` and `α` expanded in `P2`, the
  solution is closed-form: `T(φ) = T0 + T2 P2(sin φ)`, where the
  equator-to-pole contrast `T2` shrinks as `D` grows (more transport,
  milder poles).
- **Recommendation:**
  - solve the steady state per planet (two numbers, `T0` and `T2`);
  - scale `A` with the greenhouse (more greenhouse, lower `A`);
  - scale `D` up for thick air and big oceans, down for thin air
    (`D` about 0 on airless worlds: each latitude balances alone);
  - add an ice-albedo switch: where `T` falls below about −10 °C, raise
    `α` toward ice values, and re-solve once (this gives ice caps and the
    possibility of snowball worlds).

### Seasons

- **Recommendation:** the subsolar latitude moves over the year as
  `φ_sun(t) = tilt × sin(2π t / year)`. For the annual mean, `Q(φ)` comes
  from the tilt (large tilts send more sunlight to the poles); for a
  seasonal view, re-solve with `Q` for the current `φ_sun`, damped by `C`
  (oceans lag and soften the seasons, land swings more).

### Tidally locked worlds

- **Fact:** on tidally locked planets of small stars, models find that
  thick clouds gather over the point under the star and reflect light,
  which cools the planet and widens the range of habitable orbits; the
  night side acts as a radiator; climate models still disagree by
  20-30 K in global mean temperature for such cases [GEN-S068].
- **Recommendation:** use the star direction instead of latitude:
  `T(θ)` from the angle `θ` to the point under the star, with a hot
  cloud-covered disc there, a cold night side, and a mild ring along the
  terminator whose width grows with `D` (thick air spreads the heat).

### Height

- **Practice:** air cools with height, about 6.5 K per km in Earth's
  standard lower atmosphere. Scale by the planet's gravity and air type.

## Step 2: wind and rain belts

- **Fact:** the rising branch of the tropical (Hadley) circulation brings
  rain near the equator, and the descending edge brings dry air to the
  subtropics. Planet rotation is one of the strongest controls on how wide
  this cell is: it widens as rotation slows and covers the whole globe on
  slow enough rotators [GEN-S047]. In idealised models, its winter edge
  scales about as `Ω^-1/2` with the rotation rate `Ω` [GEN-S047].
- **Earth pattern (practice summary):** rain near the equator, deserts
  near 30 degrees, wetter mid-latitudes with westerly storms, dry cold
  poles.
- **Recommendation:**
  - place the subtropical dry belt at a latitude that grows as the spin
    slows, from about 30 degrees for an Earth day toward the poles for
    slow rotators;
  - fast rotators (giant-like days) get more, narrower belts, as giant
    planets do ([planet types — gas giants](./planet-types.md#gas-giants));
  - add **rain shadows:** rain falls on the windward side of mountain
    belts and is missing on the lee side. Use the prevailing wind direction
    of the belt and the height difference along it.
  - add **distance from the sea:** continental interiors dry out.

## Step 3: biomes

- **Fact (Whittaker):** plotting each place's mean annual temperature
  against its annual rain predicts which biome develops there, from
  tropical rainforest (hot and wet) to tundra (cold); the borders are
  approximate [GEN-S045].
- **Fact (Köppen):** climates fall into five groups: A tropical (every
  month 18 °C or warmer), B arid (rain below a threshold of
  `20 × mean temperature + 0 to 280` mm), C temperate, D continental and
  E polar [GEN-S046].
- **Recommendation:** classify each point with a Whittaker-style lookup
  (temperature, rain) into about ten biomes, and give each a palette:

| Biome | When | Ground colour |
|---|---|---|
| Ice cap | mean temperature well below 0 °C | white, blue-white |
| Tundra | cold, any rain | grey-green, brown |
| Boreal forest | cool, moderate rain | dark green |
| Temperate forest | mild, wet | green |
| Grassland | mild to warm, moderate rain | yellow-green |
| Shrubland | warm, dryish | olive, tan |
| Hot desert | warm, below the Köppen B threshold | sand, ochre, red |
| Cold desert | cold, below the threshold | grey, pale brown |
| Savanna | hot, seasonal rain | yellow-brown with green |
| Tropical forest | hot, very wet | deep green |

- **Blend:** blend the palettes across biome borders with noise, so
  borders are not hard lines; snow on high ground follows the temperature
  with height.

## Step 4: plant colour under other stars

- **Fact:** modelling of plant pigments for other stars suggests pigments
  would peak in the blue around hotter F stars, in the red-orange around K
  stars, and in the near-infrared around M stars, with the exact band
  depending on the star and the air (especially water vapour) [GEN-S048].
  Organisms under water are limited to shorter wavelengths [GEN-S048].
- **Recommendation:** "vegetation" palettes depend on the star type: green
  for Sun-like stars, but tinted (for example toward darker, redder or
  near-black foliage for cool red stars) as an artistic reading of the
  research. Label such worlds as speculative in any in-game text.

## Step 5: clouds

- **Practice:** clouds are a separate shell above the ground, from 3D
  noise (fBm) thresholded by a cover share, with:
  - the cover share from the rain belts (more over the equatorial rain belt
    and mid-latitude storm tracks, less over the subtropical deserts);
  - the noise stretched along longitude in belts with strong east-west
    winds, so clouds streak;
  - occasional spiral storms (a warp around a centre) in warm oceans.
- **Cloud layers (practice, Earth's families):** low, flat sheets and
  heaps (stratus, cumulus) over cool seas and fronts; tall towers
  (cumulonimbus) in the equatorial rain belt; thin high wisps (cirrus)
  above both. Recommendation: two shells, low (thick, bright, blocky
  noise) and high (thin, streaked noise), with the tall towers as bright
  blobs in the rain belt. See
  [clouds and weather](../../documents/clouds-weather.md) for the families.
- **Drift (recommendation):** the cloud noise moves with the belt winds:
  sample it at `direction` rotated about the spin axis by
  `ω_belt(lat) × t`, where `t` is the game clock and `ω_belt` a seeded
  angular speed per belt (eastward in mid-latitudes, westward near the
  equator on Earth-like worlds). Add a slow evolution by sampling 4D
  noise or by blending two time slices. Deterministic from the seed and
  the clock.
- Background: [clouds and weather](../../documents/clouds-weather.md).

## Step 6: sky and air colour

- **Fact:** a clear, thin gas atmosphere scatters blue light most
  (Rayleigh scattering): a dust-free Mars would have a blue sky [GEN-S049].
  Real Mars has a butterscotch daytime sky because its dust absorbs blue
  and scatters the rest, and blue sunsets, because micron-sized dust
  scatters blue light forward near the Sun [GEN-S049].
- **Recommendation:** the air tint (Astrolith's `THIN_AIR_TINT` and
  `THICK_AIR_TINT` today) becomes a function of the air's thickness, its
  dust and haze load, and the star's colour:
  - thin, clean air: blue sky and rim;
  - dusty air: tan-orange day sky, blue glow around a setting sun;
  - thick haze (Titan-like, see [moons](./moons.md)): orange, featureless;
  - starlight colour multiplies everything: redder light under cool stars.

## Checks

- Deserts sit mostly in the subtropical dry belts and in rain shadows.
- Ice caps and snow lines follow temperature with latitude and height.
- No biome colours on airless or waterless worlds.
- A slower spin moves the dry belts poleward.
- The same seed gives the same climate map.

## Where to go next

- [Planet types](./planet-types.md) — which types have climates.
- [Lakes](./lakes.md) — the wet and dry budget per region.
- [Lifetime](./lifetime.md) — climate shifts as the star brightens.
- Background: [atmosphere](../../documents/atmosphere.md),
  [clouds and weather](../../documents/clouds-weather.md).

## References

- [Research-pass sources](./sources-passes.md): GEN-S045 to GEN-S049,
  GEN-S067 to GEN-S068.
