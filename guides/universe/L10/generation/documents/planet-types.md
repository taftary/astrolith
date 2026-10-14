# Planet types — classify, then generate each type

This page explains how a generator can decide **what kind** of planet a
seed gives, and then **how** to build each kind. The look and science of
each kind are described in the
[planet and moon kinds catalog](../../../catalogs/documents/planets.md);
this page turns them into rules and recipes. Relief blocks are explained
in [height maps](./height-maps.md).

Labels: **fact** (cited), **practice** (common in generators),
**recommendation** (a proposal for Astrolith, not a decision).

## The classifier

Generate the physical numbers first, then name the type from them. Never
draw the type first and then invent numbers to fit: the numbers drive the
look, the type is only a label for a recipe.

### Step 1: mass and radius

Draw the mass (log-uniform within what the system allows, see
[L09](../../../L09/documents/README.md)), then get the radius from a
mass-radius relation. The probabilistic relation of Chen and Kipping
(fitted on 316 objects) finds three planet regimes and stars; its Table 1
gives the fitted values below [GEN-S012] (fact, read in full):

| Regime | Mass | Radius grows as | Spread of radius |
|---|---|---|---|
| Terran (rocky) | under 2.04 Earth masses | `M^0.279` | about 4 % |
| Neptunian (thick gas envelope) | 2.04 Earth masses to 0.414 Jupiter masses (about 132 Earth masses) | `M^0.589` | about 15 % |
| Jovian (giant) | 0.414 Jupiter masses to 0.080 Sun masses | `M^-0.044` (flat) | about 7 % |

The Terran law's constant is 1.008 Earth radii at one Earth mass. Joined
into one continuous curve (with `R` in Earth radii, `M` in Earth masses):

```text
M <  2.04  :  R = 1.008 * M^0.279
M <  131.6 :  R = 1.231 * (M / 2.04)^0.589
otherwise  :  R = 14.3  * (M / 131.6)^-0.044
```

- **Spread (recommendation):** add a seeded scatter with the regime's
  spread (4, 15 or 7 percent), so not every planet of one mass is the
  same size.
- **Giants come out large (interpretation):** this mean curve gives about
  13.8 Earth radii at Jupiter's mass, while Jupiter is 11.2. The fitted
  sample includes many strongly irradiated, inflated giants. For cold
  giants far from their star, scale the Jovian branch down to about 11
  Earth radii at one Jupiter mass; keep the full value for hot ones.

**The radius gap (fact):** small planets cluster below 1.5 Earth radii
(super-Earths) and from 2.0 to 4.0 Earth radii (sub-Neptunes), with few in
between. The gap fits atmosphere loss, and it moves with the star's mass
and the orbit [GEN-S014]. **Recommendation:** for Neptunian-regime planets
on hot orbits, draw whether the envelope was stripped. A stripped one keeps
only its rocky core (radius from the Terran law) and becomes a super-Earth.

### Step 2: gravity and temperature

- **Surface gravity:** `g = 9.81 * M / R^2` m/s² (Earth units for `M` and
  `R`). It sets the relief limit; see
  [how gravity limits relief](./height-maps.md#how-gravity-limits-relief).
- **Equilibrium temperature (fact):** absorbed starlight equals emitted
  heat [GEN-S015]:

  ```text
  T_eq = 278 K * (L / L_sun)^(1/4) * (1 - A)^(1/4) / sqrt(a / 1 AU)
  ```

  `A` is the albedo and `a` the orbit. Greenhouse warming comes on top
  (Earth's `T_eq` is about 255 K, and its surface averages about 288 K).

### Step 3: where it formed

- **Ice line (fact):** around a Sun-like star, water ice survives beyond
  about 2.7 AU (zero albedo, ice at 170 K) [GEN-S016].
- **Recommendation:** scale it as `a_ice = 2.7 AU * sqrt(L / L_sun)`.
  Planets that formed beyond it are water- and ice-rich; inside it they
  are rock and metal. Allow a seeded chance of migration inward (hot
  Jupiters, ocean worlds close in).

### Step 4: pick the type

Recommendation — evaluate in this order; the first match wins:

| Test | Type |
|---|---|
| no star (rogue draw) | rogue: use the type below with `T_eq` from internal heat only (see [lifetime — rogue planets](./lifetime.md#rogue-planets)) |
| Jovian regime and `T_eq` over about 1,000 K | hot Jupiter (puffy if the radius draw is large) |
| Jovian regime | gas giant |
| Neptunian regime, formed beyond the ice line | ice giant |
| Neptunian regime, envelope kept | sub-Neptune |
| Neptunian regime, envelope stripped | super-Earth (rocky recipe below at that size) |
| Terran, radius under about 0.3 Earth radii (about 2,000 km) | dwarf planet |
| Terran, `T_eq` over about 1,200 K | lava world |
| Terran, iron-fraction draw over about 0.6 | iron world |
| Terran, formed beyond the ice line, water-fraction draw high | ocean world (warm) or ice world (cold) |
| Terran, carbon-rich star draw | carbon world |
| Terran, `T_eq` about 200-320 K with water | Earth-like |
| Terran, `T_eq` over about 320 K or little water | desert world |
| Terran, `T_eq` under about 200 K | ice world |

Circumbinary is not a surface type: it is an orbit around two stars, and
the planet uses whichever type above applies (catalog:
[circumbinary worlds](../../../catalogs/documents/planets.md#circumbinary-worlds--two-sunsets-observed)).

## Recipes by type

Each recipe sets the [height-map](./height-maps.md) layers and the look.
Starting amplitudes per type, peak caps, hypsometry targets and the
gallery check are in
[height maps — tuned starting values](./height-maps.md#tuned-starting-values).
All amplitudes are for Earth gravity and are multiplied by `relief_scale`
from the gravity limit. They are starting values to tune by eye, not
measured constants (recommendation).

### Rocky worlds

| Type | Relief layers | Sea / ice | Craters | Volcanoes | Palette and air |
|---|---|---|---|---|---|
| Earth-like | continents mask at 25-40 % land; hybrid fBm hills; ridged belts on continent edges; strong warp; faked erosion | sea by percentile; polar caps by latitude | few, soft (erased by weather) | some, on belts and hot spots | green-brown land, blue sea, white caps; blue thin air |
| Desert | hybrid fBm; ridged ranges; dune fields as fine directional noise in basins | no sea; small polar caps | moderate, softened by wind | some, old shields | ochre to red; thin pinkish or no air |
| Lava | low relief; smooth plains; cracked crust as Voronoi cell edges | a "sea" of lava filling low ground | few (resurfaced) | many, active | black crust, glowing orange cracks on the night side; thin or rock-vapour air |
| Iron | strong relief allowed (dense, high `g` lowers it again); scarps as long ridged lines | none | many | none | dark grey metal tones; no air |
| Ice | ridged noise for pressure ridges; long crack lines; smooth plains | all ice; maybe subsurface sea | moderate, viscous (shallow, relaxed) | cryovolcanoes, rare | white to blue-grey; thin or no air |
| Ocean | sea-floor relief only, deep; a few island arcs | 95-100 % sea | none visible | island volcanoes | deep blue, cloud-heavy; thick humid air |
| Carbon | rugged ridged terrain; dark plains | tar-like low areas (assumption) | moderate | some | black and grey, oily sheen; hazy air (theoretical [catalog]) |
| Super-Earth | as the rocky type it matches, with `relief_scale` under 1 (high `g`) | as matched | as matched | more (more internal heat) | as matched; thicker air |
| Dwarf | crater-dominated; few hills | ice or none | saturated (`D^-2`, see [craters](./height-maps.md#building-block-6-craters)) | rare cryovolcanoes | grey to reddish (organics); no air |

Notes:
- **Earth-like** is the only type that needs the full toolbox. The
  continent mask and mountain belts matter more than the fine noise for
  looking like Earth from orbit.
- **Lava worlds** show their relief mostly through emission: glowing
  cracks and lava lakes, which only read clearly on the night side.
- **Ocean worlds** need a believable sea floor only if the player can go
  under water. Otherwise the height map can be very coarse.

### Giant planets (no solid surface)

Giants have no ground to fly to. Their "height map" becomes a **cloud
field**: colour bands, storms and haze on a sphere at the cloud top.

#### Gas giants

- **Bands (fact):** Jupiter's belts and zones go with alternating zonal
  jets that reach to the poles. The band width scales with the Rhines
  length `L ~ π (U/β)^1/2`, so a faster spin gives narrower, more numerous
  bands [GEN-S017].
- **Recipe (recommendation):**
  - latitude bands from a 1D noise in latitude, the band count from the
    spin;
  - turbulence: domain-warp the bands along longitude with fBm, strongest
    at band edges (shear);
  - storms: ovals at band edges, a few large and many small, with a
    seeded long-lived great spot on some planets;
  - palette: cream, tan and brown for ammonia clouds (Jupiter-like); paler
    gold for cold, hazier ones (Saturn-like);
  - rings: see [moons — rings](./moons.md#rings).

#### Ice giants

- Few, faint bands, and a blue-green tint (methane absorbs red). Use the
  gas-giant recipe with a low band contrast, occasional dark spots, and
  bright high clouds.

#### Hot Jupiters and puffy planets

- A hot day side and a cooler night side (tidally locked). Little banding,
  dark (low albedo), glowing on the night side if very hot. Puffy ones
  use a larger radius at the same mass.

#### Sub-Neptunes

- A hazy envelope, no visible ground. Use a soft, low-contrast band field
  and a thick rim glow.

## Shared look rules

- **Air:** whether a body keeps an atmosphere depends on its gravity and
  temperature (escape speed against molecule speed). Astrolith's rule
  today (no air under 0.5 Earth radii) is a size stand-in.
  **Recommendation:** replace it with the [air model](./air-model.md):
  the cosmic-shoreline test over the world's age, then a gas mix,
  pressure band and greenhouse offset per type.
- **Tilt and spin:** independent draws (Astrolith already draws both).
  Spin feeds the gas-giant band count; tilt feeds the ice caps' position.
- **Climate:** temperature, rain belts, biomes and clouds come from
  [climate and look](./climate-and-look.md).
- **Colour from composition:** use the
  [rocks catalog](../../../catalogs/documents/rocks.md) for rock and ice
  tones, so palettes stay grounded.

## Checks for a generated planet

- **Classifier coverage:** over many seeds, every type appears, with
  rocky types the most common and giants rarer, as in the
  [exoplanet census](../../../catalogs/documents/planets.md).
- **Same numbers, same type:** the type is a pure function of the drawn
  numbers.
- **Relief limit:** the highest peak of every rocky type stays under the
  gravity bound.
- **Look test:** a gallery of one planet per type at a fixed seed,
  compared by eye with the catalog descriptions.

## Where to go next

- [Height maps](./height-maps.md) — the blocks used above.
- [Moons](./moons.md) — moon systems for each planet type.
- [Planet and moon kinds catalog](../../../catalogs/documents/planets.md).

## References

- [Sources](./sources.md): GEN-S012, GEN-S014 to GEN-S017.
