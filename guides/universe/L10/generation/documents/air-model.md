# Air model — keeping air, its mix, pressure and greenhouse

This page replaces the size-only air rule with one model: decide whether
a world keeps an atmosphere over its life, then give it a gas mix, a
surface pressure and a greenhouse warming from its type. The measured
airs of the Solar System are described in
[atmospheres](../../documents/atmospheres.md); this page turns them into
generation rules. Temperature and climate then follow in
[climate and look](./climate-and-look.md).

Labels: **fact** (cited), **practice**, **recommendation** (a proposal for
Astrolith, not a decision), **estimate** (a value chosen for the
generator, not measured).

## Step 1: does the world keep air?

- **Fact:** bodies with and without atmospheres split along a line where
  the light they receive grows as the fourth power of their escape speed,
  `I ∝ v_esc^4`, across planets, moons, asteroids, Kuiper Belt objects and
  exoplanets (the cosmic shoreline) [GEN-S033].
- **Recommendation:**
  - compute `v_esc = sqrt(2 G M / R)` and the light received `I`
    (relative to Earth) from the star's luminosity at each age
    ([lifetime — brightening by star class](./lifetime.md#brightening-by-star-class));
  - define the line by `I_line(v) = I_0 × (v / v_0)^4`, with `I_0, v_0`
    fitted once so that Mercury and the Moon fall on the airless side and
    Mars, Titan, Earth and Venus on the air side;
  - integrate over age: the world keeps air only if it spent most of its
    life on the air side. Young red dwarfs push close-in worlds across the
    line ([lifetime — young red dwarfs](./lifetime.md#brightening-by-star-class));
  - a world whose dynamo died early (small, old) loses extra air, as in
    [lifetime — the interior cools](./lifetime.md#the-interior-cools).
- Giants always keep their air; this test is for rocky worlds and moons.

## Step 2: gas mix and pressure by type

Measured reference worlds (fact, from
[atmospheres](../../documents/atmospheres.md#key-numbers)):

| World | Main gases | Surface pressure |
|---|---|---|
| Earth | 78 % nitrogen, 21 % oxygen | 1 bar |
| Venus | over 96 % carbon dioxide, 3.5 % nitrogen | about 93 bar |
| Mars | mostly carbon dioxide | under 1 % of Earth's |
| Titan | 94 % nitrogen, about 5.6 % methane | 1.5 bar |

Recommendation (**estimates**, seeded within the band):

| Type (with air) | Gas mix | Pressure band |
|---|---|---|
| Earth-like | nitrogen-oxygen if life is flagged, otherwise nitrogen-carbon dioxide | 0.5-3 bar |
| Desert, small or old | carbon dioxide | 0.001-0.1 bar |
| Hot rocky, Venus-like (`T_eq` over about 300 K, water lost) | carbon dioxide, sulfur clouds | 10-100 bar |
| Lava | rock vapour, sodium, silicon oxides | under 0.001 bar |
| Ice world, cold moon, Titan-like | nitrogen with methane | 0.5-5 bar for large cold moons, otherwise trace |
| Ocean | water vapour, nitrogen, carbon dioxide | 1-10 bar |
| Super-Earth | as its rocky type, thicker | twice its rocky band |
| Sub-Neptune, giants | hydrogen and helium | no surface |

## Step 3: greenhouse warming

- **Fact:** greenhouse warming is the gap between the surface and the
  equilibrium temperature: Earth about 255 K against 288 K (+33 K), Venus
  about 220-230 K against about 730 K (about +510 K), Mars a few kelvin
  (sources give 6 to 23 K, depending on the temperatures used), and Titan a
  few kelvin (82 K against 94 K in one comparison) [GEN-S066].
- **Recommendation (estimates):** a greenhouse offset `ΔT_gh` from the
  pressure and the gas:

| Air | `ΔT_gh` |
|---|---|
| none or trace | 0 K |
| thin carbon dioxide (under 0.1 bar) | 2-10 K |
| Earth-like (about 1 bar, some water and carbon dioxide) | 20-50 K |
| thick carbon dioxide (over 10 bar) | 200-500 K, a runaway |
| cold nitrogen-methane (Titan-like) | 5-15 K, minus haze cooling |
| hydrogen blanket (rogue, see [lifetime — rogue planets](./lifetime.md#rogue-planets)) | enough for liquid water, rare |

- The surface temperature is then `T_eq + ΔT_gh`, used by
  [climate and look](./climate-and-look.md#step-1-temperature).

## Step 4: the look

- The sky, rim and haze colours follow from the pressure, dust and haze,
  and the starlight; see
  [climate and look — sky and air colour](./climate-and-look.md#step-6-sky-and-air-colour).
- Clouds by gas: water (white), sulfuric acid (yellow-white, Venus),
  methane and organic haze (orange, Titan), ammonia (cream, giants).

## Checks

- Every world on the airless side of the shoreline has no air.
- Mercury-like and Moon-like worlds come out airless; Mars-like, Titan-like
  and Venus-like ones keep air.
- Surface temperatures of the reference rows land near the measured
  values.
- Air pressure and gas mix stay inside the type's band.

## Where to go next

- [Planet types](./planet-types.md) — where this replaces the size rule.
- [Lifetime](./lifetime.md) — the star's light and the interior over age.
- [Climate and look](./climate-and-look.md) — temperature and colour.
- Background: [atmospheres](../../documents/atmospheres.md),
  [atmosphere](../../documents/atmosphere.md).

## References

- [Sources](./sources.md) and [research-pass sources](./sources-passes.md): GEN-S033, GEN-S066.
